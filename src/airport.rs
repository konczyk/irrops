use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

pub type AirportId = Arc<str>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Tabled)]
pub struct Airport {
    pub id: Arc<str>,
    pub mtt: u64
}

impl fmt::Display for Airport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}