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

impl From<u16> for Time {
    fn from(value: u16) -> Self {
        Self {
            hour: value / 60,
            minute: value % 60,
        }
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