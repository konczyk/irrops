use crate::airport::AirportId;
use crate::time::Time;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct Availability {
    pub from: Time,
    pub to: Time,
    pub location_id: Option<AirportId>,
}

pub type AircraftId = Arc<str>;

#[derive(Serialize, Deserialize)]
pub struct Aircraft {
    pub id: AircraftId,
    pub disruptions: Vec<Availability>,
    pub initial_location_id: AirportId,
}
