use crate::flight::FlightStatus::{Scheduled, Unscheduled};
use crate::flight::UnscheduledReason::{AirportCurfew, BrokenChain};
use crate::schedule::schedule::Schedule;
use crate::schedule::tests::utils::{add_aircraft, add_airport, add_flight, id};
use crate::time::Time;
use std::collections::HashMap;

#[test]
fn test_curfew_chain_reaction() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        200,
        300,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        400,
        500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "KRK",
        600,
        700,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.apply_curfew(id("WAW"), Time(450), Time(550));

    assert_eq!(Some(id("PLANE_1")), schedule.flights[0].aircraft_id);
    assert_eq!(Time(200), schedule.flights[0].departure_time);
    assert_eq!(Time(300), schedule.flights[0].arrival_time);
    assert_eq!(Scheduled, schedule.flights[0].status);

    assert_eq!(None, schedule.flights[1].aircraft_id);
    assert_eq!(Time(400), schedule.flights[1].departure_time);
    assert_eq!(Time(500), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(AirportCurfew), schedule.flights[1].status);

    assert_eq!(None, schedule.flights[2].aircraft_id);
    assert_eq!(Time(600), schedule.flights[2].departure_time);
    assert_eq!(Time(700), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);

    schedule.assign();
    assert_eq!(Scheduled, schedule.flights[0].status);
    assert_eq!(Unscheduled(AirportCurfew), schedule.flights[1].status);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}
