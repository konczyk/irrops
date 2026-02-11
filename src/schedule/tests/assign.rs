use crate::flight::FlightStatus::{Scheduled, Unscheduled};
use crate::flight::UnscheduledReason::{AircraftMaintenance, BrokenChain, Waiting};
use crate::schedule::schedule::Schedule;
use crate::schedule::tests::utils::{add_aircraft, add_airport, add_flight, availability, id};
use crate::time::Time;
use std::collections::HashMap;

#[test]
fn test_location_consistency() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "KRK",
        "GDN",
        300,
        400,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
    assert_eq!(schedule.flights[1].aircraft_id, None);
}

#[test]
fn test_mtt_conflict() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WAW",
        "GDN",
        220,
        300,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
    assert_eq!(schedule.flights[1].aircraft_id, None);
}

#[test]
fn test_continuity_schedule() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WAW",
        "GDN",
        240,
        300,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
    assert_eq!(schedule.flights[1].aircraft_id, Some(id("PLANE_1")));
}

#[test]
fn test_determinism() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);

    add_aircraft(&mut aircraft, "A", "GDN", vec![]);
    add_aircraft(&mut aircraft, "B", "GDN", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "GDN",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("A")));
}

#[test]
fn test_availability_disruption_without_location() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(150, 250, None)],
    );

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, None);
}

#[test]
fn test_availability_disruption_with_location() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(250, 300, Some(id("GDN")))],
    );

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WAW",
        "GDN",
        400,
        500,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
    assert_eq!(schedule.flights[1].aircraft_id, None);
}

#[test]
fn test_perfect_fit_mtt() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        100,
        200,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WAW",
        "GDN",
        230,
        300,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
    assert_eq!(schedule.flights[1].aircraft_id, Some(id("PLANE_1")));
}

#[test]
fn test_multiday_flight() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WAW",
        1200,
        1500,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "GDN",
        1100,
        1800,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();

    assert_eq!(schedule.flights[0].aircraft_id, Some(id("PLANE_1")));
    assert_eq!(schedule.flights[1].aircraft_id, None);
}

#[test]
fn test_recovery_after_disruption() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(600, 800, None)],
    );
    add_aircraft(&mut aircraft, "PLANE_2", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        200,
        500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "KRK",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 400);

    assert_eq!(None, schedule.flights[0].aircraft_id);
    assert_eq!(Time(200) + 400, schedule.flights[0].departure_time);
    assert_eq!(Time(500) + 400, schedule.flights[0].arrival_time);
    assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[0].status);

    assert_eq!(None, schedule.flights[1].aircraft_id);
    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

    schedule.assign();

    assert_eq!(Some(id("PLANE_2")), schedule.flights[0].aircraft_id);
    assert_eq!(Time(200) + 400, schedule.flights[0].departure_time);
    assert_eq!(Time(500) + 400, schedule.flights[0].arrival_time);
    assert_eq!(Scheduled, schedule.flights[0].status);

    assert_eq!(Some(id("PLANE_1")), schedule.flights[1].aircraft_id);
    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Scheduled, schedule.flights[1].status);
}
