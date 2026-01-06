use std::cmp::PartialEq;
use crate::aircraft::Aircraft;
use crate::flight::Flight;
use std::collections::HashMap;

pub struct Schedule {
    aircraft: HashMap<String, Aircraft>,
    flights: Vec<Flight>,
    aircraft_location: HashMap<String, String>,
}

impl Schedule {
    pub fn new(aircraft: HashMap<String, Aircraft>, flights: Vec<Flight>) -> Schedule {
        let aircraft_location = aircraft.values().map(|a| (a.id.clone(), a.initial_location.id.clone())).collect::<HashMap<String, String>>();
        Schedule { aircraft, flights, aircraft_location  }
    }

    fn assign(&mut self)  {
        let mut sorted_ids = self.aircraft.keys().collect::<Vec<&String>>();
        sorted_ids.sort();
        let mut busy = sorted_ids.iter()
            .filter_map(|id| self.aircraft.get(*id).map(|ac| (*id, ac)))
            .map(|(id, ac)| {
                (id.clone(), ac.disruptions.iter().map(|d| (d.from.to_minutes(), d.to.to_minutes(), ac.initial_location.id.clone())).collect())
            }).collect::<Vec<(String, Vec<(u16, u16, String)>)>>();

        self.flights.sort_by_key(|f| f.departure_time);
        self.flights.iter_mut().for_each(|flight| {
            if let Some((id, intervals)) = busy.iter_mut()
                .filter(|(id, _)| self.aircraft.get(id).and_then(|ac| self.aircraft_location.get(&ac.id).map(|id| flight.origin.id == *id)).unwrap_or(false))
                .find(|(_, blocks)| {
                    blocks.iter().all(|(from, to, _)| {
                        flight.departure_time > *to || flight.arrival_time < *from
                    })
                }) {
                    flight.aircraft_id = Some(id.clone());
                    self.aircraft_location.insert(id.clone(), flight.destination.id.clone());
                    intervals.push((flight.departure_time, (flight.arrival_time + flight.destination.mtt).clamp(0, 60*24), flight.destination.id.clone()))
                }

        });
    }
}