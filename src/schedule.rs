use crate::aircraft::{Aircraft, AircraftId};
use crate::airport::AirportId;
use crate::flight::{Flight, FlightId};
use std::collections::HashMap;
use std::sync::Arc;
use crate::time::Time;

pub struct Schedule {
    aircraft: HashMap<AircraftId, Aircraft>,
    flights: Vec<Flight>,
    flights_index: HashMap<FlightId, usize>
}

impl Schedule {
    pub fn new(aircraft: HashMap<AircraftId, Aircraft>, mut flights: Vec<Flight>) -> Schedule {
        flights.sort_by_key(|f| f.departure_time);
        let flights_index = flights.iter().enumerate().map(|(i, v)| (v.id.clone(), i)).collect::<HashMap<FlightId, usize>>();
        Schedule {
            aircraft,
            flights,
            flights_index
        }
    }

    pub fn assign(&mut self)  {
        let mut sorted_ids = self.aircraft.keys().collect::<Vec<&Arc<str>>>();
        sorted_ids.sort();
        let mut busy = sorted_ids.iter()
            .filter_map(|id| self.aircraft.get(*id).map(|ac| (*id, ac)))
            .map(|(id, ac)| {
                (id.clone(), ac.initial_location.id.clone(), ac.disruptions.iter().map(|d| (d.from, d.to)).collect())
            }).collect::<Vec<(AircraftId, AirportId, Vec<(Time, Time)>)>>();

        self.flights.sort_by_key(|f| f.departure_time);
        self.flights.iter_mut().for_each(|flight| {
            if let Some((id, loc, intervals)) = busy.iter_mut()
                .filter(|(_, loc, _)| flight.origin.id == *loc)
                .find(|(_, _, blocks)| {
                    blocks.iter().all(|(from, to)| {
                        flight.departure_time >= *to || flight.arrival_time <= *from
                    })
                }) {
                    flight.aircraft_id = Some(id.clone());
                    *loc = flight.destination.id.clone();
                    intervals.push((flight.departure_time, flight.arrival_time + flight.destination.mtt))
                }

        });
    }

    pub fn apply_delay(&mut self, flight_id: FlightId, shift: u64) {
        let idx = self.flights_index.get(&flight_id);
        let result = idx.and_then(|i| Some((i, self.flights[*i].aircraft_id.clone())));
        if let Some((f_id, aid)) = result {
            self.flights[*f_id].departure_time += shift;
            self.flights[*f_id].arrival_time += shift;

            if let Some(ac_id) = aid {
                let mut prev_arrival_time = self.flights[*f_id].arrival_time;

                for flight in self.flights.iter_mut().skip(*f_id + 1).filter(|f| f.aircraft_id.as_ref().map(|x| **x == *ac_id).unwrap_or(false)) {
                    if flight.departure_time < prev_arrival_time + flight.origin.mtt {
                        let len = flight.arrival_time - flight.departure_time;
                        flight.departure_time = prev_arrival_time + flight.origin.mtt;
                        flight.arrival_time = flight.departure_time + len;
                        prev_arrival_time = flight.arrival_time;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::aircraft::Availability;
    use super::*;
    use crate::airport::Airport;

    fn id(s: &str) -> Arc<str> { Arc::from(s) }

    #[test]
    fn test_location_consistency() {
        let ac_id = id("PLANE_1");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id.clone(), Aircraft {
            id: ac_id.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 100,
                arrival_time: 200,
                aircraft_id: None
            },
            Flight {
                id: id("FLIGHT_2"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 300,
                arrival_time: 400,
                aircraft_id: None
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(ac_id));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_mtt_conflict() {
        let ac_id = id("PLANE_1");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id.clone(), Aircraft {
            id: ac_id.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 100,
                arrival_time: 200,
                aircraft_id: None
            },
            Flight {
                id: id("FLIGHT_2"),
                origin: Airport { id: id("WAW"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 220,
                arrival_time: 300,
                aircraft_id: None
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(ac_id));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_continuity_schedule() {
        let ac_id = id("PLANE_1");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id.clone(), Aircraft {
            id: ac_id.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 100,
                arrival_time: 200,
                aircraft_id: None
            },
            Flight {
                id: id("FLIGHT_2"),
                origin: Airport { id: id("WAW"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 240,
                arrival_time: 300,
                aircraft_id: None
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(ac_id.clone()));
        assert_eq!(schedule.flights[1].aircraft_id, Some(ac_id.clone()));
    }
    
    #[test]
    fn test_determinism() {
        let ac_a = id("A");
        let ac_b = id("B");
        let mut aircraft = HashMap::new();
        let airport = Airport { id: id("GDN"), mtt: 30 };
    
        aircraft.insert(ac_a.clone(), Aircraft { id: ac_a.clone(), initial_location: airport.clone(), disruptions: vec![] });
        aircraft.insert(ac_b.clone(), Aircraft { id: ac_b.clone(), initial_location: airport.clone(), disruptions: vec![] });
    
        let flights = vec![Flight {
            id: id("FLIGHT_1"),
            origin: airport.clone(), 
            destination: Airport { id: id("WAW"), mtt: 30 },
            departure_time: 100, 
            arrival_time: 200, 
            aircraft_id: None
        }];
    
        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();
    
        assert_eq!(schedule.flights[0].aircraft_id, Some(ac_a));
    }

    #[test]
    fn test_disruption() {
        let ac_id = id("PLANE_1");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id.clone(), Aircraft {
            id: ac_id.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![Availability { from: 150, to: 250 }],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 100,
                arrival_time: 200,
                aircraft_id: None
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, None);
    }


    #[test]
    fn test_perfect_fit_mtt() {
        let ac_id = id("PLANE_1");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id.clone(), Aircraft {
            id: ac_id.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 100,
                arrival_time: 200,
                aircraft_id: None
            },
            Flight {
                id: id("FLIGHT_2"),
                origin: Airport { id: id("WAW"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 230,
                arrival_time: 300,
                aircraft_id: None
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(ac_id.clone()));
        assert_eq!(schedule.flights[1].aircraft_id, Some(ac_id.clone()));
    }

    #[test]
    fn test_multiday_flight() {
        let ac_id = id("PLANE_1");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id.clone(), Aircraft {
            id: ac_id.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 1200,
                arrival_time: 1500,
                aircraft_id: None
            },
            Flight {
                id: id("FLIGHT_2"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 1100,
                arrival_time: 1800,
                aircraft_id: None
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();

        assert_eq!(schedule.flights[0].aircraft_id, Some(ac_id));
        assert_eq!(schedule.flights[1].aircraft_id, None);
    }

    #[test]
    fn test_delay_full_absorption() {
        let ac_id1 = id("PLANE_1");
        let ac_id2 = id("PLANE_2");
        let mut aircraft = HashMap::new();
        aircraft.insert(ac_id1.clone(), Aircraft {
            id: ac_id1.clone(),
            initial_location: Airport { id: id("KRK"), mtt: 30 },
            disruptions: vec![],
        });
        aircraft.insert(ac_id2.clone(), Aircraft {
            id: ac_id2.clone(),
            initial_location: Airport { id: id("WAW"), mtt: 30 },
            disruptions: vec![],
        });

        let flights = vec![
            Flight {
                id: id("FLIGHT_1"),
                origin: Airport { id: id("KRK"), mtt: 30 },
                destination: Airport { id: id("WRO"), mtt: 30 },
                departure_time: 1200,
                arrival_time: 1500,
                aircraft_id: Some(ac_id1.clone()),
            },
            Flight {
                id: id("FLIGHT_2"),
                origin: Airport { id: id("WRO"), mtt: 30 },
                destination: Airport { id: id("WAW"), mtt: 30 },
                departure_time: 1800,
                arrival_time: 2000,
                aircraft_id: Some(ac_id1.clone()),
            },
            Flight {
                id: id("FLIGHT_3"),
                origin: Airport { id: id("WAW"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 2100,
                arrival_time: 2350,
                aircraft_id: Some(ac_id1.clone()),
            },
            Flight {
                id: id("FLIGHT_4"),
                origin: Airport { id: id("WAW"), mtt: 30 },
                destination: Airport { id: id("GDN"), mtt: 30 },
                departure_time: 2100,
                arrival_time: 2300,
                aircraft_id: Some(ac_id2),
            },
        ];

        let mut schedule = Schedule::new(aircraft, flights);
        schedule.assign();
        schedule.apply_delay(id("FLIGHT_1"), 500);

        assert_eq!(1200 + 500, schedule.flights[0].departure_time);
        assert_eq!(1500 + 500, schedule.flights[0].arrival_time);

        assert_eq!(2000 + 30, schedule.flights[1].departure_time);
        assert_eq!(2000 + 30 + 200, schedule.flights[1].arrival_time);

        assert_eq!(2230 + 30, schedule.flights[2].departure_time);
        assert_eq!(2230 + 30 + 250, schedule.flights[2].arrival_time);

        assert_eq!(2100, schedule.flights[3].departure_time);
        assert_eq!(2300, schedule.flights[3].arrival_time);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use crate::airport::Airport;

    fn arb_id(prefix: &'static str) -> impl Strategy<Value=Arc<str>> {
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
        ).prop_map(|(id, org, dst, dep, dur)| Flight {
            id,
            origin: Airport { id: org, mtt: 30 },
            destination: Airport { id: dst, mtt: 30 },
            departure_time: dep,
            arrival_time: dep + dur,
            aircraft_id: None,
        })
    }

    proptest! {
        #[test]
        fn test_time_and_location_invariants(
            aircraft_data in prop::collection::vec((arb_id("AC"), arb_id("AP")), 1..5),
            flights in prop::collection::vec(arb_flight(), 1..30)
        ) {
            let mut aircraft_map = HashMap::new();
            for (ac_id, loc_id) in aircraft_data {
                aircraft_map.insert(ac_id.clone(), Aircraft {
                    id: ac_id,
                    initial_location: Airport { id: loc_id, mtt: 30 },
                    disruptions: vec![],
                });
            }

            let mut schedule = Schedule::new(aircraft_map, flights);

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
                        first.destination == second.origin,
                        "\nWrong airport:\nFlight {} lands at {} vs Flight {} (takes off at {})",
                        first.id, first.destination.id, second.id, second.origin.id
                    );
                }

                if let Some(first_flight) = assigned.first() {
                    if let Some(ac) = schedule.aircraft.get(&first_flight.aircraft_id.clone().unwrap()) {
                        prop_assert!(
                            first_flight.origin == ac.initial_location,
                            "\nWrong airport:\nAircraft {} originates at {} but Flight {} (takes off at {})",
                            ac.id, ac.initial_location.id, first_flight.id, first_flight.origin.id
                        );
                    }
                }
            }
        }
    }


}