use crate::schedule::schedule::Schedule;
use crate::schedule::tests::utils::{add_aircraft, add_airport, arb_flight, arb_id};
use proptest::prelude::*;
use proptest::proptest;
use std::collections::HashMap;

proptest! {
    #[test]
    fn test_time_and_location_invariants(
        aircraft_data in prop::collection::vec((arb_id("AC"), arb_id("AP")), 1..5),
        flights in prop::collection::vec(arb_flight(), 1..30)
    ) {
        let mut aircraft_map = HashMap::new();
        let mut airports_map = HashMap::new();
        for (ac_id, loc_id) in aircraft_data {
            add_aircraft(&mut aircraft_map, ac_id.as_ref(), loc_id.as_ref(), vec![]);
        }
        add_airport(&mut airports_map, "AP_1", 20, vec![]);
        add_airport(&mut airports_map, "AP_2", 45, vec![]);
        add_airport(&mut airports_map, "AP_3", 60, vec![]);
        let mut schedule = Schedule::new(aircraft_map, airports_map, flights);

        schedule.assign();

        for ac_id in schedule.aircraft.keys() {
            let mut assigned: Vec<_> = schedule.flights.iter()
                .filter(|f| f.aircraft_id.as_ref() == Some(ac_id))
                .collect();

            assigned.sort_by_key(|f| f.departure_time);

            for pair in assigned.windows(2) {
                let first = &pair[0];
                let second = &pair[1];

                let mtt = schedule.airports
                    .get(&first.destination_id)
                    .unwrap()
                    .mtt;
                let ready_at = first.arrival_time + mtt;

                prop_assert!(
                    second.departure_time >= ready_at,
                    "\nOverlap on {}:\nFlight {} (ends {}+{}m MTT) vs Flight {} (starts {})",
                    ac_id, first.id, first.arrival_time, mtt, second.id, second.departure_time
                );

                prop_assert!(
                    first.destination_id == second.origin_id,
                    "\nWrong airport:\nFlight {} lands at {} vs Flight {} (takes off at {})",
                    first.id, first.destination_id, second.id, second.origin_id
                );
            }

            if let Some(first_flight) = assigned.first() {
                if let Some(ac) = schedule.aircraft.get(&first_flight.aircraft_id.clone().unwrap()) {
                    prop_assert!(
                        first_flight.origin_id == ac.initial_location_id,
                        "\nWrong airport:\nAircraft {} originates at {} but Flight {} (takes off at {})",
                        ac.id, ac.initial_location_id, first_flight.id, first_flight.origin_id
                    );
                }
            }
        }
    }
}
