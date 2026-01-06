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
        let mut busy = self.aircraft.iter()
            .map(|(id, ac)| {
                (id.clone(), ac.disruptions.iter().map(|d| (d.from.to_minutes(), d.to.to_minutes())).collect())
            }).collect::<HashMap<String, Vec<(u16, u16)>>>();

        self.flights.sort_by(|a, b| a.departure_time.cmp(&b.departure_time));
        self.flights.iter_mut().for_each(|flight| {
            let ac = busy.iter().find(|(_, blocks)| {
                blocks.iter().all(|(from, to)| {
                    flight.departure_time > *to || flight.arrival_time < *from
                })
            }).map(|(id, _)| id.clone());

            if let Some(id) = ac {
                flight.aircraft_id = Some(id.clone());
                if let Some(intervals) = busy.get_mut(&id) {
                   intervals.push((flight.departure_time, flight.arrival_time))

                }
            }
        });
    }
}