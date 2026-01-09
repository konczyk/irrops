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