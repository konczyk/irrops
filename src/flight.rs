use crate::aircraft::AircraftId;
use crate::airport::AirportId;
use crate::time::Time;
use colored::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tabled::Tabled;

pub type FlightId = Arc<str>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum UnscheduledReason {
    Waiting,
    MaxDelayExceeded,
    AirportCurfew,
    AircraftMaintenance,
    BrokenChain,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Tabled)]
pub enum FlightStatus {
    Unscheduled(UnscheduledReason),
    Scheduled,
    Delayed,
}

impl FlightStatus {
    pub fn is_unscheduled(&self) -> bool {
        match self {
            FlightStatus::Unscheduled(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for FlightStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FlightStatus::Scheduled => "Scheduled".green(),
            FlightStatus::Delayed => "Delayed".yellow(),
            FlightStatus::Unscheduled(_) => "Unscheduled".red(),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct Flight {
    pub id: FlightId,
    #[tabled(display = "display_option")]
    pub aircraft_id: Option<AircraftId>,
    pub origin_id: AirportId,
    pub destination_id: AirportId,
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
