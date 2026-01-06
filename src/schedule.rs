use crate::aircraft::{Aircraft, AircraftId};
use crate::flight::Flight;
use std::collections::HashMap;
use std::sync::Arc;
use crate::airport::AirportId;

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