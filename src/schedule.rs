use crate::aircraft::Aircraft;
use crate::flight::Flight;
use std::collections::HashMap;

pub struct Schedule {
    aircraft: HashMap<String, Aircraft>,
    flights: Vec<Flight>
}

impl Schedule {
    pub fn new(aircraft: HashMap<String, Aircraft>, flights: Vec<Flight>) -> Schedule {
        Schedule { aircraft, flights }
    }

    fn assign(&mut self)  {
        let mut sorted_ids = self.aircraft.keys().collect::<Vec<&String>>();
        sorted_ids.sort();
        let mut busy = sorted_ids.iter()
            .filter_map(|id| self.aircraft.get(*id).map(|ac| (*id, ac)))
            .map(|(id, ac)| {
                (id.clone(), ac.disruptions.iter().map(|d| (d.from.to_minutes(), d.to.to_minutes())).collect())
            }).collect::<Vec<(String, Vec<(u16, u16)>)>>();

        self.flights.sort_by_key(|f| f.departure_time);
        self.flights.iter_mut().for_each(|flight| {
            if let Some((id, intervals)) = busy.iter_mut().find(|(_, blocks)| {
                blocks.iter().all(|(from, to)| {
                    flight.departure_time > *to || flight.arrival_time < *from
                })
            }) {
                flight.aircraft_id = Some(id.clone());
                intervals.push((flight.departure_time, (flight.arrival_time + flight.destination.mtt).clamp(0, 60*24)))
            }

        });
    }
}