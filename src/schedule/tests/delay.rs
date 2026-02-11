use crate::flight::FlightId;
use crate::flight::FlightStatus::{Delayed, Scheduled, Unscheduled};
use crate::flight::UnscheduledReason::{
    AircraftMaintenance, AirportCurfew, BrokenChain, MaxDelayExceeded, Waiting,
};
use crate::schedule::schedule::Schedule;
use crate::schedule::tests::utils::{
    add_aircraft, add_airport, add_flight, availability, curfew, id,
};
use crate::time::Time;
use std::collections::HashMap;

#[test]
fn test_delay_full_absorption() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);
    add_aircraft(&mut aircraft, "PLANE_2", "WAW", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        None,
        Unscheduled(Waiting),
    );
    add_flight(
        &mut flights,
        "FLIGHT_4",
        "WAW",
        "GDN",
        2100,
        2300,
        None,
        Unscheduled(Waiting),
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 500);

    assert_eq!(Time(1200) + 500, schedule.flights[0].departure_time);
    assert_eq!(Time(1500) + 500, schedule.flights[0].arrival_time);

    assert_eq!(Time(2000) + 30, schedule.flights[1].departure_time);
    assert_eq!(Time(2000) + 30 + 200, schedule.flights[1].arrival_time);

    assert_eq!(Time(2230) + 30, schedule.flights[2].departure_time);
    assert_eq!(Time(2230) + 30 + 250, schedule.flights[2].arrival_time);

    assert_eq!(Time(2100), schedule.flights[3].departure_time);
    assert_eq!(Time(2300), schedule.flights[3].arrival_time);
}

#[test]
fn test_delay_aircraft_first_flight_into_availability_disruption() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(1800, 1900, None)],
    );

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 500);
    let report = schedule
        .last_report
        .unwrap()
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")], report);

    assert_eq!(Time(1700), schedule.flights[0].departure_time);
    assert_eq!(Time(2000), schedule.flights[0].arrival_time);
    assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

    assert_eq!(Time(2100), schedule.flights[2].departure_time);
    assert_eq!(Time(2350), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_subsequent_flight_into_availability_disruption() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(2100, 2200, None)],
    );

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 500);
    let report = schedule
        .last_report
        .unwrap()
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_2"), id("FLIGHT_3")], report);

    assert_eq!(Time(1200) + 500, schedule.flights[0].departure_time);
    assert_eq!(Time(1500) + 500, schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[1].status);

    assert_eq!(Time(2100), schedule.flights[2].departure_time);
    assert_eq!(Time(2350), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_first_flight_into_curfew() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);
    airports
        .entry(id("WRO"))
        .and_modify(|x| x.disruptions.push(curfew(1600, 1700)));

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 150);
    let report = schedule
        .last_report
        .unwrap()
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")], report);

    assert_eq!(Time(1350), schedule.flights[0].departure_time);
    assert_eq!(Time(1650), schedule.flights[0].arrival_time);
    assert_eq!(Unscheduled(AirportCurfew), schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

    assert_eq!(Time(2100), schedule.flights[2].departure_time);
    assert_eq!(Time(2350), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_subsequent_flight_into_curfew() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);
    airports
        .entry(id("WRO"))
        .and_modify(|x| x.disruptions.push(curfew(2010, 2100)));

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 500);
    let report = schedule
        .last_report
        .unwrap()
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_2"), id("FLIGHT_3")], report);

    assert_eq!(Time(1200) + 500, schedule.flights[0].departure_time);
    assert_eq!(Time(1500) + 500, schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(AirportCurfew), schedule.flights[1].status);

    assert_eq!(Time(2100), schedule.flights[2].departure_time);
    assert_eq!(Time(2350), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_first_flight_into_max_delay() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 2050);
    let report = schedule
        .last_report
        .unwrap()
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")], report);

    assert_eq!(Time(1200), schedule.flights[0].departure_time);
    assert_eq!(Time(1500), schedule.flights[0].arrival_time);
    assert_eq!(Unscheduled(MaxDelayExceeded), schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[1].status);

    assert_eq!(Time(2100), schedule.flights[2].departure_time);
    assert_eq!(Time(2350), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_subsequent_flight_into_max_delay() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
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
        305,
        500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        600,
        700,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 1999);
    let report = schedule.last_report.unwrap();
    let broken = report
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_2"), id("FLIGHT_3")], *broken);
    assert_eq!(vec![id("FLIGHT_1")], report.affected);

    assert_eq!(Time(200) + 1999, schedule.flights[0].departure_time);
    assert_eq!(Time(300) + 1999, schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(305), schedule.flights[1].departure_time);
    assert_eq!(Time(500), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(MaxDelayExceeded), schedule.flights[1].status);

    assert_eq!(Time(600), schedule.flights[2].departure_time);
    assert_eq!(Time(700), schedule.flights[2].arrival_time);
    assert_eq!(Unscheduled(BrokenChain), schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_no_shift() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 100);
    let report = schedule.last_report.unwrap();
    assert!(report.unscheduled.is_empty());
    assert_eq!(vec![id("FLIGHT_1")], report.affected);

    assert_eq!(Time(1300), schedule.flights[0].departure_time);
    assert_eq!(Time(1600), schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Scheduled, schedule.flights[1].status);

    assert_eq!(Time(2100), schedule.flights[2].departure_time);
    assert_eq!(Time(2350), schedule.flights[2].arrival_time);
    assert_eq!(Scheduled, schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_first_flight_by_overlap() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 500);
    let report = schedule.last_report.unwrap();
    assert!(report.unscheduled.is_empty());
    assert_eq!(
        vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")],
        report.affected
    );

    assert_eq!(Time(1700), schedule.flights[0].departure_time);
    assert_eq!(Time(2000), schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(2030), schedule.flights[1].departure_time);
    assert_eq!(Time(2230), schedule.flights[1].arrival_time);
    assert_eq!(Delayed, schedule.flights[1].status);

    assert_eq!(Time(2260), schedule.flights[2].departure_time);
    assert_eq!(Time(2510), schedule.flights[2].arrival_time);
    assert_eq!(Delayed, schedule.flights[2].status);
}

#[test]
fn test_delay_aircraft_first_flight_by_leapfrog() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(&mut aircraft, "PLANE_1", "KRK", vec![]);

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_3",
        "WAW",
        "GDN",
        2100,
        2350,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.assign();
    schedule.apply_delay(id("FLIGHT_1"), 1000);
    let report = schedule.last_report.unwrap();
    assert!(report.unscheduled.is_empty());
    assert_eq!(
        vec![id("FLIGHT_1"), id("FLIGHT_2"), id("FLIGHT_3")],
        report.affected
    );

    assert_eq!(Time(2200), schedule.flights[0].departure_time);
    assert_eq!(Time(2500), schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(2530), schedule.flights[1].departure_time);
    assert_eq!(Time(2730), schedule.flights[1].arrival_time);
    assert_eq!(Delayed, schedule.flights[1].status);

    assert_eq!(Time(2760), schedule.flights[2].departure_time);
    assert_eq!(Time(3010), schedule.flights[2].arrival_time);
    assert_eq!(Delayed, schedule.flights[2].status);
}

#[test]
fn test_delay_into_spatial_disruption() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(1600, 1650, Some(id("KRK")))],
    );

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.apply_delay(id("FLIGHT_1"), 50);
    let report = schedule.last_report.unwrap();
    let broken = report
        .unscheduled
        .iter()
        .map(|(x, _)| x.clone())
        .collect::<Vec<FlightId>>();
    assert_eq!(vec![id("FLIGHT_2")], broken);
    assert_eq!(vec![id("FLIGHT_1")], report.affected);

    assert_eq!(Time(1250), schedule.flights[0].departure_time);
    assert_eq!(Time(1550), schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Unscheduled(AircraftMaintenance), schedule.flights[1].status);
}

#[test]
fn test_delay_into_valid_base_maintenance() {
    let mut aircraft = HashMap::new();
    let mut airports = HashMap::new();
    let mut flights = Vec::new();

    add_airport(&mut airports, "KRK", 30, vec![]);
    add_airport(&mut airports, "WAW", 30, vec![]);
    add_airport(&mut airports, "GDN", 30, vec![]);
    add_airport(&mut airports, "WRO", 30, vec![]);

    add_aircraft(
        &mut aircraft,
        "PLANE_1",
        "KRK",
        vec![availability(1600, 1650, Some(id("WRO")))],
    );

    add_flight(
        &mut flights,
        "FLIGHT_1",
        "KRK",
        "WRO",
        1200,
        1500,
        Some("PLANE_1"),
        Scheduled,
    );
    add_flight(
        &mut flights,
        "FLIGHT_2",
        "WRO",
        "WAW",
        1800,
        2000,
        Some("PLANE_1"),
        Scheduled,
    );

    let mut schedule = Schedule::new(aircraft, airports, flights);
    schedule.apply_delay(id("FLIGHT_1"), 50);
    let report = schedule.last_report.unwrap();
    assert!(report.unscheduled.is_empty());
    assert_eq!(vec![id("FLIGHT_1")], report.affected);

    assert_eq!(Time(1250), schedule.flights[0].departure_time);
    assert_eq!(Time(1550), schedule.flights[0].arrival_time);
    assert_eq!(Delayed, schedule.flights[0].status);

    assert_eq!(Time(1800), schedule.flights[1].departure_time);
    assert_eq!(Time(2000), schedule.flights[1].arrival_time);
    assert_eq!(Scheduled, schedule.flights[1].status);
}
