use crate::time::Time;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;
use tabled::Tabled;

pub type AirportId = Arc<str>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Tabled)]
pub struct Airport {
    pub id: Arc<str>,
    pub mtt: u64,
    #[tabled(display = "format_disruptions")]
    pub disruptions: Vec<(Time, Time)>
}

impl fmt::Display for Airport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

fn format_disruptions(disruptions: &Vec<(Time, Time)>) -> String {
    if disruptions.is_empty() {
        return "None".to_string();
    }
    disruptions.iter()
        .map(|(start, end)| format!("{}-{}", start, end))
        .collect::<Vec<_>>()
        .join(", ")
}