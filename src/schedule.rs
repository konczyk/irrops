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

}