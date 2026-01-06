use std::sync::Arc;
use crate::aircraft::AircraftId;
use crate::airport::Airport;

pub type FlightId = Arc<str>;

pub struct Flight {
    id: FlightId,
    pub aircraft_id: Option<AircraftId>,
    pub origin: Airport,
    pub destination: Airport,
    pub departure_time: u16,
    pub arrival_time: u16,
}