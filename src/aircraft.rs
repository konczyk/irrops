use std::sync::Arc;
use crate::airport::Airport;

pub struct Time {
    hour: u16,
    minute: u16,
}

impl Time {
    pub fn to_minutes(&self) -> u16 {
        self.hour * 60 + self.minute
    }
}

pub struct Availability {
    pub from: Time,
    pub to: Time,
}

pub type AircraftId = Arc<str>;

pub struct Aircraft {
    pub id: AircraftId,
    pub disruptions: Vec<Availability>,
    pub initial_location: Airport,
}