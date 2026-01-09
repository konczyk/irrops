use std::sync::Arc;
use crate::airport::Airport;
use crate::time::Time;

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