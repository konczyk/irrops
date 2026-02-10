use crate::aircraft::{Aircraft, AircraftId, Availability};
use crate::airport::{Airport, AirportId, Curfew};
use crate::flight::FlightStatus::{Delayed, Scheduled, Unscheduled};
use crate::flight::UnscheduledReason::{
    AircraftMaintenance, AirportCurfew, BrokenChain, MaxDelayExceeded,
};
use crate::flight::{Flight, FlightId, UnscheduledReason};
use crate::time::Time;
use serde::Deserialize;
use std::collections::HashMap;
use std::io;

pub enum DisruptionType {
    Delay { flight: FlightId },
    Curfew { airport: AirportId },
}

pub struct DisruptionReport {
    pub kind: DisruptionType,
    pub affected: Vec<FlightId>,
    pub unscheduled: Vec<(FlightId, UnscheduledReason)>,
    pub first_break: Option<(FlightId, UnscheduledReason)>,
}

pub struct Schedule {
    aircraft: HashMap<AircraftId, Aircraft>,
    airports: HashMap<AirportId, Airport>,
    pub flights: Vec<Flight>,
    flights_index: HashMap<FlightId, usize>,
}

impl Schedule {
    const MAX_DELAY: u64 = 2000;

    pub fn new(
        aircraft: HashMap<AircraftId, Aircraft>,
        airports: HashMap<AirportId, Airport>,
        mut flights: Vec<Flight>,
    ) -> Schedule {
        flights.sort_by_key(|f| f.departure_time);
        let flights_index = flights
            .iter()
            .enumerate()
            .map(|(i, v)| (v.id.clone(), i))
            .collect::<HashMap<FlightId, usize>>();
        Schedule {
            aircraft,
            airports,
            flights,
            flights_index,
        }
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        #[derive(Deserialize)]
        struct RawData {
            aircraft: Vec<Aircraft>,
            airports: Vec<Airport>,
            flights: Vec<Flight>,
        }
        let raw: RawData = serde_json::from_str(&data)?;

        let ac_map = raw
            .aircraft
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();

        let ap_map = raw
            .airports
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();

        Ok(Schedule::new(ac_map, ap_map, raw.flights))
    }

    fn unschedule(&mut self, flight_id: &FlightId, reason: UnscheduledReason) {
        if let Some(idx) = self.flights_index.get(flight_id) {
            self.flights[*idx].status = Unscheduled(reason);
            self.flights[*idx].aircraft_id = None;
        }
    }

    fn is_at_wrong_airport(
        disruptions: &[Availability],
        departure_time: Time,
        ready_at: Option<&(AirportId, Time)>,
    ) -> bool {
        ready_at
            .map(|(location_id, arrival_time)| {
                disruptions
                    .iter()
                    .filter(|d| {
                        d.from >= *arrival_time && d.to <= departure_time && d.location_id.is_some()
                    })
                    .any(|d| Some(location_id.clone()) != d.location_id)
            })
            .unwrap_or(false)
    }

    fn is_airport_closed(
        airports: &HashMap<AirportId, Airport>,
        flight: &Flight,
        dep_time: Time,
        arr_time: Time,
    ) -> bool {
        let orig_closed = airports.get(&flight.origin_id).map_or(false, |ap| {
            ap.disruptions
                .iter()
                .any(|d| d.from <= dep_time && d.to >= dep_time)
        });
        let dest_closed = airports.get(&flight.destination_id).map_or(false, |ap| {
            ap.disruptions
                .iter()
                .any(|d| d.from <= arr_time && d.to >= arr_time)
        });
        orig_closed || dest_closed
    }

    fn violates_aircraft_maintenance(disruptions: &[Availability], dep: Time, arr: Time) -> bool {
        disruptions
            .iter()
            .any(|d| Time::is_overlapping(&(dep, arr), &(d.from, d.to)))
    }

    fn get_ready_time(
        airports: &HashMap<AirportId, Airport>,
        arrival_time: Time,
        airport_id: &AirportId,
    ) -> Time {
        arrival_time + airports.get(airport_id).map(|x| x.mtt).unwrap_or(0)
    }

    fn compute_shifted_times(
        airports: &HashMap<AirportId, Airport>,
        flight: &Flight,
        prev_arrival: Time,
    ) -> (Time, Time, bool) {
        let len = flight.arrival_time - flight.departure_time;
        let ready_at = Self::get_ready_time(airports, prev_arrival, &flight.origin_id);
        let dep_time = ready_at.max(flight.departure_time);
        let arr_time = dep_time + len;
        let is_overlapping = flight.departure_time < ready_at;
        (dep_time, arr_time, is_overlapping)
    }

    pub fn assign(&mut self) {
        let mut sorted_ids = self.aircraft.keys().collect::<Vec<&AircraftId>>();
        sorted_ids.sort();

        let mut current_locations: HashMap<AircraftId, (AirportId, Time)> = self
            .aircraft
            .iter()
            .map(|(id, ac)| (id.clone(), (ac.initial_location_id.clone(), Time(0))))
            .collect();

        self.flights
            .iter()
            .filter(|f| !f.status.is_unscheduled())
            .for_each(|f| {
                if let Some(ac_id) = &f.aircraft_id {
                    current_locations.insert(
                        ac_id.clone(),
                        (
                            f.destination_id.clone(),
                            Self::get_ready_time(&self.airports, f.arrival_time, &f.destination_id),
                        ),
                    );
                }
            });

        // collect aircraft per airport, sorted by aircraft name
        let mut aircraft_by_airport = HashMap::<AirportId, Vec<&AircraftId>>::new();
        sorted_ids.iter().for_each(|ac_id| {
            if let Some(ap_id) = current_locations.get(*ac_id).map(|x| x.0.clone()) {
                aircraft_by_airport
                    .entry(ap_id.clone())
                    .or_default()
                    .push(*ac_id)
            }
        });

        // collect disruptions due to currently scheduled flights
        let mut busy = HashMap::<AircraftId, Vec<(Time, Time)>>::new();
        self.flights
            .iter()
            .map(|f| {
                (
                    f.aircraft_id.as_ref(),
                    f.departure_time,
                    Self::get_ready_time(&self.airports, f.arrival_time, &f.destination_id),
                )
            })
            .filter_map(|(maybe_id, dep, arr)| maybe_id.map(|id| (id.clone(), (dep, arr))))
            .for_each(|(id, val)| busy.entry(id).or_default().push(val));

        self.flights
            .iter_mut()
            .filter(|flight| flight.status.is_unscheduled())
            .for_each(|flight| {
                // collect candidates at the origin airport that are not disrupted
                let chosen_aircraft =
                    aircraft_by_airport
                        .get(&flight.origin_id)
                        .and_then(|ac_ids| {
                            ac_ids
                                .iter()
                                .filter_map(|ac_id| self.aircraft.get(*ac_id))
                                // filter aircraft at the origin airport that are not disrupted
                                .filter(|a| {
                                    a.disruptions.iter().all(|d| {
                                        !Time::is_overlapping(
                                            &(flight.departure_time, flight.arrival_time),
                                            &(d.from, d.to),
                                        )
                                    })
                                })
                                // filter aircraft that have maintenance window ending before the flight and are at the proper airport
                                .filter(|a| {
                                    !Self::is_at_wrong_airport(
                                        &a.disruptions,
                                        flight.departure_time,
                                        current_locations.get(&a.id),
                                    )
                                })
                                // filter out busy ones
                                .filter(|ac| {
                                    busy.get(&ac.id).map_or(true, |intervals| {
                                        intervals.iter().all(|(from, to)| {
                                            !Time::is_overlapping(
                                                &(flight.departure_time, flight.arrival_time),
                                                &(*from, *to),
                                            )
                                        })
                                    })
                                })
                                // filter out busy due to curfew
                                .find(|_| {
                                    let origin_open =
                                        self.airports.get(&flight.origin_id).map_or(true, |ap| {
                                            !ap.disruptions.iter().any(|d| {
                                                d.from <= flight.departure_time
                                                    && d.to >= flight.departure_time
                                            })
                                        });
                                    let destination_open = self
                                        .airports
                                        .get(&flight.destination_id)
                                        .map_or(true, |ap| {
                                            !ap.disruptions.iter().any(|d| {
                                                d.from <= flight.arrival_time
                                                    && d.to >= flight.arrival_time
                                            })
                                        });
                                    origin_open && destination_open
                                })
                        });

                if let Some(aircraft) = chosen_aircraft {
                    flight.aircraft_id = Some(aircraft.id.clone());
                    flight.status = Scheduled;
                    let mtt = self
                        .airports
                        .get(&flight.destination_id)
                        .map(|ap| ap.mtt)
                        .unwrap_or(0);
                    busy.entry(aircraft.id.clone())
                        .or_default()
                        .push((flight.departure_time, flight.arrival_time + mtt));
                    aircraft_by_airport
                        .entry(flight.destination_id.clone())
                        .and_modify(|val| {
                            val.push(&aircraft.id);
                            val.sort();
                        })
                        .or_insert(vec![&aircraft.id]);
                    aircraft_by_airport
                        .entry(flight.origin_id.clone())
                        .and_modify(|val| val.retain(|id| **id != aircraft.id));
                    current_locations.insert(
                        aircraft.id.clone(),
                        (
                            flight.destination_id.clone(),
                            Self::get_ready_time(
                                &self.airports,
                                flight.arrival_time,
                                &flight.destination_id,
                            ),
                        ),
                    );
                }
            })
    }

    pub fn apply_delay(&mut self, flight_id: FlightId, shift: u64) -> DisruptionReport {
        let mut report = DisruptionReport {
            kind: DisruptionType::Delay {
                flight: flight_id.clone(),
            },
            affected: vec![],
            unscheduled: vec![],
            first_break: None,
        };

        if shift == 0 {
            return report;
        }

        // lookup flight & aircraft
        let idx = self.flights_index.get(&flight_id);
        let flight_aircraft =
            idx.and_then(|i| Some((i, self.flights[*i].aircraft_id.as_ref().map(|x| x.clone()))));

        if let Some((f_id, ac_id)) = flight_aircraft {
            let empty_ac_vec = vec![];
            let ac_disruptions = ac_id
                .as_ref()
                .and_then(|i| self.aircraft.get(i))
                .map(|a| a.disruptions.as_slice())
                .unwrap_or(&empty_ac_vec);

            let mut is_broken = false;

            // apply delay to triggering flight
            if shift > Self::MAX_DELAY {
                report
                    .unscheduled
                    .push((self.flights[*f_id].id.clone(), MaxDelayExceeded));
                is_broken = true;
            } else {
                let orig_dep_time = self.flights[*f_id].departure_time;
                self.flights[*f_id].departure_time += shift;
                self.flights[*f_id].arrival_time += shift;
                let shifted_arr_time = self.flights[*f_id].arrival_time;
                if Self::violates_aircraft_maintenance(
                    &ac_disruptions,
                    orig_dep_time,
                    shifted_arr_time,
                ) {
                    report
                        .unscheduled
                        .push((self.flights[*f_id].id.clone(), AircraftMaintenance));
                    is_broken = true;
                } else if Self::is_airport_closed(
                    &self.airports,
                    &self.flights[*f_id],
                    orig_dep_time,
                    shifted_arr_time,
                ) {
                    report
                        .unscheduled
                        .push((self.flights[*f_id].id.clone(), AirportCurfew));
                    is_broken = true;
                } else {
                    self.flights[*f_id].status = Delayed;
                    report.affected.push(self.flights[*f_id].id.clone());
                }
            }

            // propagate delay along aircraft chain
            if let Some(ac_id) = ac_id {
                let mut prev_arrival_time = self.flights[*f_id].arrival_time;
                let mut prev_destination_id = self.flights[*f_id].destination_id.clone();

                for flight in self.flights.iter_mut().skip(*f_id + 1).filter(|f| {
                    f.aircraft_id
                        .as_ref()
                        .map(|x| **x == *ac_id)
                        .unwrap_or(false)
                }) {
                    if is_broken {
                        report.unscheduled.push((flight.id.clone(), BrokenChain));
                        continue;
                    }

                    let (dep_time, arr_time, is_overlapping) =
                        Self::compute_shifted_times(&self.airports, flight, prev_arrival_time);
                    let is_ac_disrupted = Self::violates_aircraft_maintenance(
                        &ac_disruptions,
                        flight.departure_time,
                        arr_time,
                    );
                    let is_at_wrong_airport = Self::is_at_wrong_airport(
                        ac_disruptions,
                        flight.departure_time,
                        Some(&(prev_destination_id.clone(), prev_arrival_time)),
                    );

                    if is_ac_disrupted || is_at_wrong_airport {
                        report
                            .unscheduled
                            .push((flight.id.clone(), AircraftMaintenance));
                        is_broken = true;
                    } else if Self::is_airport_closed(&self.airports, &flight, dep_time, arr_time) {
                        report.unscheduled.push((flight.id.clone(), AirportCurfew));
                        is_broken = true;
                    } else if dep_time - flight.departure_time > Time(Self::MAX_DELAY) {
                        report
                            .unscheduled
                            .push((flight.id.clone(), MaxDelayExceeded));
                        is_broken = true;
                    } else if is_overlapping {
                        flight.departure_time = dep_time;
                        flight.arrival_time = arr_time;
                        flight.status = Delayed;
                        prev_arrival_time = flight.arrival_time;
                        prev_destination_id = flight.destination_id.clone();
                        report.affected.push(flight.id.clone());
                    } else {
                        break;
                    }
                }
            }
        }
        report.unscheduled.iter().for_each(|(f_id, reason)| {
            self.unschedule(f_id, *reason);
        });
        report.first_break = report.unscheduled.first().cloned();

        report
    }

    pub fn apply_curfew(
        &mut self,
        airport_id: AirportId,
        from: Time,
        to: Time,
    ) -> DisruptionReport {
        let mut report = DisruptionReport {
            kind: DisruptionType::Curfew {
                airport: airport_id.clone(),
            },
            affected: vec![],
            unscheduled: vec![],
            first_break: None,
        };

        let maybe_airport = self.airports.get_mut(&airport_id);
        if let Some(airport) = maybe_airport {
            airport.disruptions.push(Curfew { from, to });

            let broken = self
                .flights
                .iter()
                .filter(|f| !f.status.is_unscheduled())
                .filter(|f| *f.origin_id == *airport_id || *f.destination_id == *airport_id)
                .filter(|f| {
                    airport.disruptions.iter().any(|Curfew { from, to }| {
                        Time::is_overlapping(&(f.departure_time, f.arrival_time), &(*from, *to))
                    })
                })
                .fold(HashMap::new(), |mut acc: HashMap<AircraftId, Time>, f| {
                    if let Some(ac_id) = f.aircraft_id.clone() {
                        acc.entry(ac_id).or_insert(f.departure_time);
                    }
                    acc
                });

            let mut counter: HashMap<AircraftId, usize> = HashMap::new();
            self.flights
                .iter_mut()
                .filter(|f| !f.status.is_unscheduled())
                .for_each(|f| {
                    if let Some(ac_id) = &f.aircraft_id {
                        let broken_time = broken.get(ac_id);
                        if let Some(time) = broken_time {
                            if f.departure_time >= *time {
                                counter
                                    .entry(ac_id.clone())
                                    .and_modify(|e| *e += 1)
                                    .or_insert(0);
                                report.unscheduled.push((
                                    f.id.clone(),
                                    if counter.get(&ac_id.clone()).map_or(true, |x| *x == 0) {
                                        AirportCurfew
                                    } else {
                                        BrokenChain
                                    },
                                ));
                            }
                        }
                    }
                })
        }
        report.unscheduled.iter().for_each(|(f_id, reason)| {
            self.unschedule(f_id, *reason);
        });
        report.first_break = report.unscheduled.first().cloned();

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aircraft::Availability;
    use crate::airport::Airport;
    use crate::flight::FlightStatus;
    use crate::flight::UnscheduledReason::Waiting;
    use std::sync::Arc;

    pub(crate) fn id(s: &str) -> Arc<str> {
        Arc::from(s)
    }

    pub(crate) fn add_aircraft(
        aircraft: &mut HashMap<AircraftId, Aircraft>,
        aircraft_id: &str,
        initial_location_id: &str,
        disruptions: Vec<Availability>,
    ) {
        aircraft.insert(
            id(aircraft_id).clone(),
            Aircraft {
                id: id(aircraft_id).clone(),
                initial_location_id: id(initial_location_id).clone(),
                disruptions,
            },
        );
    }

    pub(crate) fn add_airport(
        airports: &mut HashMap<AirportId, Airport>,
        airport_id: &str,
        mtt: u64,
        disruptions: Vec<Curfew>,
    ) {
        airports.insert(
            id(airport_id).clone(),
            Airport {
                id: id(airport_id).clone(),
                mtt,
                disruptions,
            },
        );
    }

    fn add_flight(
        flights: &mut Vec<Flight>,
        flight_id: &str,
        origin_id: &str,
        destination_id: &str,
        departure_time: u64,
        arrival_time: u64,
        aircraft_id: Option<&str>,
        status: FlightStatus,
    ) {
        flights.push(Flight {
            id: id(flight_id),
            origin_id: id(origin_id),
            destination_id: id(destination_id),
            departure_time: Time(departure_time),
            arrival_time: Time(arrival_time),
            aircraft_id: aircraft_id.map(|x| id(x)),
            status,
        });
    }

    fn availability(from: u64, to: u64, location_id: Option<AirportId>) -> Availability {
        Availability {
            from: Time(from),
            to: Time(to),
            location_id,
        }
    }

    fn curfew(from: u64, to: u64) -> Curfew {
        Curfew {
            from: Time(from),
            to: Time(to),
        }
    }

    #[test]
    fn test_location_consistency() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "KRK",
            "GDN",
            300,
            400,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_mtt_conflict() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WAW",
            "GDN",
            220,
            300,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_continuity_schedule() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WAW",
            "GDN",
            240,
            300,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
        assert_eq!(schedule.flights[1].aircraft_id, Some(id("PLANE_1")));
    }

    #[test]
    fn test_determinism() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);

        add_aircraft(&mut aircraft, "A", "GDN", vec![]);
        add_aircraft(&mut aircraft, "B", "GDN", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "GDN",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("A")));
    }

    #[test]
    fn test_availability_disruption_without_location() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(150, 250, None)],
        );

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, None);
    }

    #[test]
    fn test_availability_disruption_with_location() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(250, 300, Some(id("GDN")))],
        );

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WAW",
            "GDN",
            400,
            500,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_perfect_fit_mtt() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            100,
            200,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WAW",
            "GDN",
            230,
            300,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
        assert_eq!(schedule.flights[1].aircraft_id, Some(id("PLANE_1")));
    }

    #[test]
    fn test_multiday_flight() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WAW",
            1200,
            1500,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "GDN",
            1100,
            1800,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_delay_full_absorption() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);
        add_aircraft(&mut aircraft, "PLANE_2", "WAW", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            None,
            Unscheduled(Waiting),
        );
        add_flight(
            &mut flights,
            "FLIGHT_4",
            "WAW",
            "GDN",
            2100,
            2300,
            None,
            Unscheduled(Waiting),
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        schedule.apply_delay(id("FLIGHT_1"), 500);

        assert_eq!(Time(1200) + 500, schedule.flights[0].departure_time);
        assert_eq!(Time(1500) + 500, schedule.flights[0].arrival_time);

        assert_eq!(Time(2000) + 30, schedule.flights[1].departure_time);
        assert_eq!(Time(2000) + 30 + 200, schedule.flights[1].arrival_time);

        assert_eq!(Time(2230) + 30, schedule.flights[2].departure_time);
        assert_eq!(Time(2230) + 30 + 250, schedule.flights[2].arrival_time);

        assert_eq!(Time(2100), schedule.flights[3].departure_time);
        assert_eq!(Time(2300), schedule.flights[3].arrival_time);
    }

    #[test]
    fn test_delay_aircraft_first_flight_into_availability_disruption() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(1800, 1900, None)],
        );

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule
            .apply_delay(id("FLIGHT_1"), 500)
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")], report);

        assert_eq!(Time(1700), schedule.flights[0].departure_time);
        assert_eq!(Time(2000), schedule.flights[0].arrival_time);
        assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

        assert_eq!(Time(2100), schedule.flights[2].departure_time);
        assert_eq!(Time(2350), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_subsequent_flight_into_availability_disruption() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(2100, 2200, None)],
        );

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule
            .apply_delay(id("FLIGHT_1"), 500)
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_2"), id("FLIGHT_3")], report);

        assert_eq!(Time(1200) + 500, schedule.flights[0].departure_time);
        assert_eq!(Time(1500) + 500, schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[1].status);

        assert_eq!(Time(2100), schedule.flights[2].departure_time);
        assert_eq!(Time(2350), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_first_flight_into_curfew() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);
        airports
            .entry(id("WRO"))
            .and_modify(|x| x.disruptions.push(curfew(1600, 1700)));

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule
            .apply_delay(id("FLIGHT_1"), 150)
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")], report);

        assert_eq!(Time(1350), schedule.flights[0].departure_time);
        assert_eq!(Time(1650), schedule.flights[0].arrival_time);
        assert_eq!(Unscheduled(AirportCurfew), schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

        assert_eq!(Time(2100), schedule.flights[2].departure_time);
        assert_eq!(Time(2350), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_subsequent_flight_into_curfew() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);
        airports
            .entry(id("WRO"))
            .and_modify(|x| x.disruptions.push(curfew(2010, 2100)));

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule
            .apply_delay(id("FLIGHT_1"), 500)
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_2"), id("FLIGHT_3")], report);

        assert_eq!(Time(1200) + 500, schedule.flights[0].departure_time);
        assert_eq!(Time(1500) + 500, schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(AirportCurfew), schedule.flights[1].status);

        assert_eq!(Time(2100), schedule.flights[2].departure_time);
        assert_eq!(Time(2350), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_first_flight_into_max_delay() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule
            .apply_delay(id("FLIGHT_1"), 2050)
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")], report);

        assert_eq!(Time(1200), schedule.flights[0].departure_time);
        assert_eq!(Time(1500), schedule.flights[0].arrival_time);
        assert_eq!(Unscheduled(MaxDelayExceeded), schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

        assert_eq!(Time(2100), schedule.flights[2].departure_time);
        assert_eq!(Time(2350), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_subsequent_flight_into_max_delay() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            200,
            300,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            305,
            500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            600,
            700,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule.apply_delay(id("FLIGHT_1"), 1999);
        let broken = report
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_2"), id("FLIGHT_3")], broken);
        assert_eq!(vec![id("FLIGHT_1")], report.affected);

        assert_eq!(Time(200) + 1999, schedule.flights[0].departure_time);
        assert_eq!(Time(300) + 1999, schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(305), schedule.flights[1].departure_time);
        assert_eq!(Time(500), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(MaxDelayExceeded), schedule.flights[1].status);

        assert_eq!(Time(600), schedule.flights[2].departure_time);
        assert_eq!(Time(700), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_no_shift() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule.apply_delay(id("FLIGHT_1"), 100);
        assert!(report.unscheduled.is_empty());
        assert_eq!(vec![id("FLIGHT_1")], report.affected);

        assert_eq!(Time(1300), schedule.flights[0].departure_time);
        assert_eq!(Time(1600), schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Scheduled, schedule.flights[1].status);

        assert_eq!(Time(2100), schedule.flights[2].departure_time);
        assert_eq!(Time(2350), schedule.flights[2].arrival_time);
        assert_eq!(Scheduled, schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_first_flight_by_overlap() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule.apply_delay(id("FLIGHT_1"), 500);
        assert!(report.unscheduled.is_empty());
        assert_eq!(
            vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")],
            report.affected
        );

        assert_eq!(Time(1700), schedule.flights[0].departure_time);
        assert_eq!(Time(2000), schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(2030), schedule.flights[1].departure_time);
        assert_eq!(Time(2230), schedule.flights[1].arrival_time);
        assert_eq!(Delayed, schedule.flights[1].status);

        assert_eq!(Time(2260), schedule.flights[2].departure_time);
        assert_eq!(Time(2510), schedule.flights[2].arrival_time);
        assert_eq!(Delayed, schedule.flights[2].status);
    }

    #[test]
    fn test_delay_aircraft_first_flight_by_leapfrog() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "GDN",
            2100,
            2350,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        let report = schedule.apply_delay(id("FLIGHT_1"), 1000);
        assert!(report.unscheduled.is_empty());
        assert_eq!(
            vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")],
            report.affected
        );

        assert_eq!(Time(2200), schedule.flights[0].departure_time);
        assert_eq!(Time(2500), schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(2530), schedule.flights[1].departure_time);
        assert_eq!(Time(2730), schedule.flights[1].arrival_time);
        assert_eq!(Delayed, schedule.flights[1].status);

        assert_eq!(Time(2760), schedule.flights[2].departure_time);
        assert_eq!(Time(3010), schedule.flights[2].arrival_time);
        assert_eq!(Delayed, schedule.flights[2].status);
    }

    #[test]
    fn test_delay_into_spatial_disruption() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(1600, 1650, Some(id("KRK")))],
        );

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        let report = schedule.apply_delay(id("FLIGHT_1"), 50);
        let broken = report
            .unscheduled
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<FlightId>>();
        assert_eq!(vec![id("FLIGHT_2")], broken);
        assert_eq!(vec![id("FLIGHT_1")], report.affected);

        assert_eq!(Time(1250), schedule.flights[0].departure_time);
        assert_eq!(Time(1550), schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[1].status);
    }

    #[test]
    fn test_delay_into_valid_base_maintenance() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "GDN", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(1600, 1650, Some(id("WRO")))],
        );

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            1200,
            1500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        let report = schedule.apply_delay(id("FLIGHT_1"), 50);
        assert!(report.unscheduled.is_empty());
        assert_eq!(vec![id("FLIGHT_1")], report.affected);

        assert_eq!(Time(1250), schedule.flights[0].departure_time);
        assert_eq!(Time(1550), schedule.flights[0].arrival_time);
        assert_eq!(Delayed, schedule.flights[0].status);

        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Scheduled, schedule.flights[1].status);
    }

    #[test]
    fn test_recovery_after_disruption() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(
            &mut aircraft,
            "PLANE_1",
            "KRK",
            vec![availability(600, 800, None)],
        );
        add_aircraft(&mut aircraft, "PLANE_2", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            200,
            500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "KRK",
            "WAW",
            1800,
            2000,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.assign();
        schedule.apply_delay(id("FLIGHT_1"), 400);

        assert_eq!(None, schedule.flights[0].aircraft_id);
        assert_eq!(Time(200) + 400, schedule.flights[0].departure_time);
        assert_eq!(Time(500) + 400, schedule.flights[0].arrival_time);
        assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[0].status);

        assert_eq!(None, schedule.flights[1].aircraft_id);
        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

        schedule.assign();

        assert_eq!(Some(id("PLANE_2")), schedule.flights[0].aircraft_id);
        assert_eq!(Time(200) + 400, schedule.flights[0].departure_time);
        assert_eq!(Time(500) + 400, schedule.flights[0].arrival_time);
        assert_eq!(Scheduled, schedule.flights[0].status);

        assert_eq!(Some(id("PLANE_1")), schedule.flights[1].aircraft_id);
        assert_eq!(Time(1800), schedule.flights[1].departure_time);
        assert_eq!(Time(2000), schedule.flights[1].arrival_time);
        assert_eq!(Scheduled, schedule.flights[1].status);
    }

    #[test]
    fn test_curfew_chain_reaction() {
        let mut aircraft = HashMap::new();
        let mut airports = HashMap::new();
        let mut flights = Vec::new();

        add_airport(&mut airports, "KRK", 30, vec![]);
        add_airport(&mut airports, "WAW", 30, vec![]);
        add_airport(&mut airports, "WRO", 30, vec![]);

        add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

        add_flight(
            &mut flights,
            "FLIGHT_1",
            "KRK",
            "WRO",
            200,
            300,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_2",
            "WRO",
            "WAW",
            400,
            500,
            Some("PLANE_1"),
            Scheduled,
        );
        add_flight(
            &mut flights,
            "FLIGHT_3",
            "WAW",
            "KRK",
            600,
            700,
            Some("PLANE_1"),
            Scheduled,
        );

        let mut schedule = Schedule::new(aircraft, airports, flights);
        schedule.apply_curfew(id("WAW"), Time(450), Time(550));

        assert_eq!(Some(id("PLANE_1")), schedule.flights[0].aircraft_id);
        assert_eq!(Time(200), schedule.flights[0].departure_time);
        assert_eq!(Time(300), schedule.flights[0].arrival_time);
        assert_eq!(Scheduled, schedule.flights[0].status);

        assert_eq!(None, schedule.flights[1].aircraft_id);
        assert_eq!(Time(400), schedule.flights[1].departure_time);
        assert_eq!(Time(500), schedule.flights[1].arrival_time);
        assert_eq!(Unscheduled(AirportCurfew), schedule.flights[1].status);

        assert_eq!(None, schedule.flights[2].aircraft_id);
        assert_eq!(Time(600), schedule.flights[2].departure_time);
        assert_eq!(Time(700), schedule.flights[2].arrival_time);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);

        schedule.assign();
        assert_eq!(Scheduled, schedule.flights[0].status);
        assert_eq!(Unscheduled(AirportCurfew), schedule.flights[1].status);
        assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
    }
}

#[cfg(test)]
mod proptests {
    use super::tests::{add_aircraft, add_airport, id};
    use super::*;
    use crate::flight::UnscheduledReason::Waiting;
    use proptest::prelude::*;
    use std::sync::Arc;

    fn arb_id(prefix: &'static str) -> impl Strategy<Value = Arc<str>> {
        prop_oneof![
            Just(Arc::from(format!("{}_1", prefix))),
            Just(Arc::from(format!("{}_2", prefix))),
            Just(Arc::from(format!("{}_3", prefix))),
        ]
    }

    fn arb_flight() -> impl Strategy<Value = Flight> {
        (
            arb_id("FL"),
            arb_id("AP"),
            arb_id("AP"),
            0..2500u64,
            10..1000u64,
        )
            .prop_map(|(fid, org, dst, dep, dur)| Flight {
                id: id(fid.as_ref()),
                origin_id: id(org.as_ref()),
                destination_id: id(dst.as_ref()),
                departure_time: Time(dep),
                arrival_time: Time(dep) + dur,
                aircraft_id: None,
                status: Unscheduled(Waiting),
            })
    }

    proptest! {
        #[test]
        fn test_time_and_location_invariants(
            aircraft_data in prop::collection::vec((arb_id("AC"), arb_id("AP")), 1..5),
            flights in prop::collection::vec(arb_flight(), 1..30)
        ) {
            let mut aircraft_map = HashMap::new();
            let mut airports_map = HashMap::new();
            for (ac_id, loc_id) in aircraft_data {
                add_aircraft(&mut aircraft_map, ac_id.as_ref(), loc_id.as_ref(), vec![]);
            }
            add_airport(&mut airports_map, "AP_1", 30, vec![]);
            add_airport(&mut airports_map, "AP_2", 30, vec![]);
            add_airport(&mut airports_map, "AP_3", 30, vec![]);
            let mut schedule = Schedule::new(aircraft_map, airports_map, flights);

            schedule.assign();

            for ac_id in schedule.aircraft.keys() {
                let mut assigned: Vec<_> = schedule.flights.iter()
                    .filter(|f| f.aircraft_id.as_ref() == Some(ac_id))
                    .collect();

                assigned.sort_by_key(|f| f.departure_time);

                for pair in assigned.windows(2) {
                    let first = &pair[0];
                    let second = &pair[1];

                    let ready_at = first.arrival_time + 30;

                    prop_assert!(
                        second.departure_time >= ready_at,
                        "\nOverlap on {}:\nFlight {} (ends {}+30m MTT) vs Flight {} (starts {})",
                        ac_id, first.id, first.arrival_time, second.id, second.departure_time
                    );

                    prop_assert!(
                        first.destination_id == second.origin_id,
                        "\nWrong airport:\nFlight {} lands at {} vs Flight {} (takes off at {})",
                        first.id, first.destination_id, second.id, second.origin_id
                    );
                }

                if let Some(first_flight) = assigned.first() {
                    if let Some(ac) = schedule.aircraft.get(&first_flight.aircraft_id.clone().unwrap()) {
                        prop_assert!(
                            first_flight.origin_id == ac.initial_location_id,
                            "\nWrong airport:\nAircraft {} originates at {} but Flight {} (takes off at {})",
                            ac.id, ac.initial_location_id, first_flight.id, first_flight.origin_id
                        );
                    }
                }
            }
        }
    }
}
