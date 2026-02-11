use crate::aircraft::{Aircraft, AircraftId, Availability};
use crate::airport::{Airport, AirportId, Curfew};
use crate::flight::FlightStatus::Unscheduled;
use crate::flight::UnscheduledReason::Waiting;
use crate::flight::{Flight, FlightStatus};
use crate::time::Time;
use proptest::prelude::Strategy;
use proptest::prop_oneof;
use proptest::strategy::Just;
use std::collections::HashMap;
use std::sync::Arc;

pub fn id(s: &str) -> Arc<str> {
    Arc::from(s)
}

pub fn add_aircraft(
    aircraft: &mut HashMap<AircraftId, Aircraft>,
    aircraft_id: &str,
    initial_location_id: &str,
    disruptions: Vec<Availability>,
) {
    aircraft.insert(
        id(aircraft_id).clone(),
        Aircraft {
            id: id(aircraft_id).clone(),
            initial_location_id: id(initial_location_id).clone(),
            disruptions,
        },
    );
}

pub fn add_airport(
    airports: &mut HashMap<AirportId, Airport>,
    airport_id: &str,
    mtt: u64,
    disruptions: Vec<Curfew>,
) {
    airports.insert(
        id(airport_id).clone(),
        Airport {
            id: id(airport_id).clone(),
            mtt,
            disruptions,
        },
    );
}

pub fn add_flight(
    flights: &mut Vec<Flight>,
    flight_id: &str,
    origin_id: &str,
    destination_id: &str,
    departure_time: u64,
    arrival_time: u64,
    aircraft_id: Option<&str>,
    status: FlightStatus,
) {
    flights.push(Flight {
        id: id(flight_id),
        origin_id: id(origin_id),
        destination_id: id(destination_id),
        departure_time: Time(departure_time),
        arrival_time: Time(arrival_time),
        aircraft_id: aircraft_id.map(|x| id(x)),
        status,
    });
}

pub fn availability(from: u64, to: u64, location_id: Option<AirportId>) -> Availability {
    Availability {
        from: Time(from),
        to: Time(to),
        location_id,
    }
}

pub fn curfew(from: u64, to: u64) -> Curfew {
    Curfew {
        from: Time(from),
        to: Time(to),
    }
}

pub fn arb_id(prefix: &'static str) -> impl Strategy<Value = Arc<str>> {
    prop_oneof![
        Just(Arc::from(format!("{}_1", prefix))),
        Just(Arc::from(format!("{}_2", prefix))),
        Just(Arc::from(format!("{}_3", prefix))),
    ]
}

pub fn arb_flight() -> impl Strategy<Value = Flight> {
    (
        arb_id("FL"),
        arb_id("AP"),
        arb_id("AP"),
        0..2500u64,
        10..1000u64,
    )
        .prop_map(|(fid, org, dst, dep, dur)| Flight {
            id: id(fid.as_ref()),
            origin_id: id(org.as_ref()),
            destination_id: id(dst.as_ref()),
            departure_time: Time(dep),
            arrival_time: Time(dep) + dur,
            aircraft_id: None,
            status: Unscheduled(Waiting),
        })
}
