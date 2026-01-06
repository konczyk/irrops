use std::sync::Arc;

pub type AirportId = Arc<str>;

#[derive(Clone, PartialEq)]
pub struct Airport {
    pub id: AirportId,
    pub mtt: u16
}