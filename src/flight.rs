use crate::airport::Airport;

pub struct Flight {
    id: String,
    pub aircraft_id: Option<String>,
    origin: Airport,
    pub destination: Airport,
    pub departure_time: u16,
    pub arrival_time: u16,
}