use crate::time::Time;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;
use tabled::Tabled;

pub type AirportId = Arc<str>;

#[derive(Serialize, Deserialize, Tabled, Clone, Debug, PartialEq)]
pub struct Curfew {
    pub from: Time,
    pub to: Time,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Tabled)]
pub struct Airport {
    pub id: Arc<str>,
    pub mtt: u64,
    #[tabled(display = "format_disruptions")]
    pub disruptions: Vec<Curfew>
}

impl fmt::Display for Airport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

fn format_disruptions(disruptions: &Vec<Curfew>) -> String {
    if disruptions.is_empty() {
        return "None".to_string();
    }
    disruptions.iter()
        .map(|Curfew {from: start, to: end }| format!("{}-{}", start, end))
        .collect::<Vec<_>>()
        .join(", ")
}