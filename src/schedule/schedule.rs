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
    Delay {
        flight: FlightId,
        delay_by: u64,
    },
    Curfew {
        airport: AirportId,
        from: Time,
        to: Time,
    },
}

pub struct DisruptionReport {
    pub kind: DisruptionType,
    pub affected: Vec<FlightId>,
    pub unscheduled: Vec<(FlightId, UnscheduledReason)>,
    pub first_break: Option<(FlightId, UnscheduledReason)>,
}

pub struct Schedule {
    pub aircraft: HashMap<AircraftId, Aircraft>,
    pub airports: HashMap<AirportId, Airport>,
    pub flights: Vec<Flight>,
    flights_index: HashMap<FlightId, usize>,
    pub last_report: Option<DisruptionReport>,
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
            last_report: None,
        }
    }

    pub fn last_report(&self) -> Option<&DisruptionReport> {
        self.last_report.as_ref()
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
            });

        self.assert_invariants();
    }

    pub fn apply_delay(&mut self, flight_id: FlightId, shift: u64) {
        let mut report = DisruptionReport {
            kind: DisruptionType::Delay {
                flight: flight_id.clone(),
                delay_by: shift,
            },
            affected: vec![],
            unscheduled: vec![],
            first_break: None,
        };

        if shift == 0 {
            return;
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
                    self.flights[*f_id].status = Delayed { minutes: shift };
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
                        flight.status = Delayed {
                            minutes: (dep_time - flight.departure_time).0,
                        };
                        flight.departure_time = dep_time;
                        flight.arrival_time = arr_time;
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

        self.last_report = Some(report);

        self.assert_invariants();
    }

    pub fn apply_curfew(&mut self, airport_id: AirportId, from: Time, to: Time) {
        let mut report = DisruptionReport {
            kind: DisruptionType::Curfew {
                airport: airport_id.clone(),
                from,
                to,
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

        self.last_report = Some(report);

        self.assert_invariants();
    }

    #[cfg(debug_assertions)]
    fn assert_invariants(&self) {
        debug_assert!(
            self.flights.iter().all(|f| {
                match &f.status {
                    Unscheduled(_) => f.aircraft_id.is_none(),
                    Scheduled | Delayed { .. } => f.aircraft_id.is_some(),
                }
            }),
            "Status <-> aircraft_id invariant violated"
        );

        debug_assert!(
            self.flights.iter().all(|f| {
                match &f.status {
                    Delayed { minutes } => *minutes > 0,
                    _ => true,
                }
            }),
            "Delay > 0 invariant violated"
        );

        let mut flight_by_aircraft: HashMap<AircraftId, Vec<&Flight>> = HashMap::new();
        for flight in &self.flights {
            if let Some(ac_id) = &flight.aircraft_id {
                flight_by_aircraft
                    .entry(ac_id.clone())
                    .or_default()
                    .push(flight);
            }
        }
        for (ac_id, mut flights) in flight_by_aircraft.into_iter() {
            flights.sort_by_key(|f| f.departure_time);
            debug_assert!(
                flights
                    .windows(2)
                    .all(|fs| { fs[0].destination_id == fs[1].origin_id }),
                "Pref destination <-> next origin location continuity violated"
            );
            debug_assert!(
                flights.windows(2).all(|fs| {
                    let mtt = self
                        .airports
                        .get(&fs[0].destination_id)
                        .map(|a| a.mtt)
                        .unwrap();
                    fs[1].departure_time >= fs[0].arrival_time + mtt
                }),
                "Pref destination <-> next origin temporal continuity violated"
            );

            if let Some(flight) = flights.first() {
                debug_assert_eq!(
                    flight.origin_id,
                    self.aircraft.get(&ac_id).unwrap().initial_location_id,
                    "First flight origin <-> aircraft initial location violated"
                );
            }
        }
    }
}
