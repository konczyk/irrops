use std::sync::Arc;
use crate::aircraft::AircraftId;
use crate::airport::Airport;
use crate::time::Time;

pub type FlightId = Arc<str>;

#[derive(Debug)]
pub struct Flight {
    pub id: FlightId,
    pub aircraft_id: Option<AircraftId>,
    pub origin: Airport,
    pub destination: Airport,
    pub departure_time: Time,
    pub arrival_time: Time,
}