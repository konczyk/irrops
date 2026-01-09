use std::sync::Arc;

pub type AirportId = Arc<str>;

#[derive(Clone, Debug, PartialEq)]
pub struct Airport {
    pub id: AirportId,
    pub mtt: u64
}