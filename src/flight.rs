use std::fmt;
use std::sync::Arc;
use colored::*;
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use crate::aircraft::AircraftId;
use crate::airport::Airport;
use crate::time::Time;

pub type FlightId = Arc<str>;

#[derive(Debug, PartialEq, Serialize, Deserialize, Tabled)]
pub enum FlightStatus {
    Unscheduled,
    Scheduled,
    Delayed,
}

impl fmt::Display for FlightStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FlightStatus::Scheduled => "Scheduled".green(),
            FlightStatus::Delayed => "Delayed".yellow(),
            FlightStatus::Unscheduled => "Unscheduled".red(),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Flight {
    pub id: FlightId,
    #[tabled(display = "display_option")]
    pub aircraft_id: Option<AircraftId>,
    pub origin: Airport,
    pub destination: Airport,
    pub departure_time: Time,
    pub arrival_time: Time,
    pub status: FlightStatus,
}

fn display_option(o: &Option<AircraftId>) -> String {
    match o {
        Some(id) => id.to_string(),
        None => "---".to_string(),
    }
}
