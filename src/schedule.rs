use crate::aircraft::{Aircraft, AircraftId};
use crate::airport::AirportId;
use crate::flight::Flight;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Schedule {
    aircraft: HashMap<AircraftId, Aircraft>,
    flights: Vec<Flight>,
}

impl Schedule {
    pub fn new(aircraft: HashMap<AircraftId, Aircraft>, flights: Vec<Flight>) -> Schedule {
        Schedule { aircraft, flights  }
    }

    fn assign(&mut self)  {
        let mut sorted_ids = self.aircraft.keys().collect::<Vec<&Arc<str>>>();
        sorted_ids.sort();
        let mut busy = sorted_ids.iter()
            .filter_map(|id| self.aircraft.get(*id).map(|ac| (*id, ac)))
            .map(|(id, ac)| {
                (id.clone(), ac.initial_location.id.clone(), ac.disruptions.iter().map(|d| (d.from.to_minutes(), d.to.to_minutes())).collect())
            }).collect::<Vec<(AircraftId, AirportId, Vec<(u16, u16)>)>>();

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
                    intervals.push((flight.departure_time, (flight.arrival_time + flight.destination.mtt).clamp(0, 60*24)))
                }

        });
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
            disruptions: vec![Availability { from: 150u16.into(), to: 250u16.into() }],
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
            0..1300u16,
            10..100u16,
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
        fn test_no_overlaps_invariant(
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

                    let ready_at = (first.arrival_time + 30).min(1440);

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