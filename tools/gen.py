import json
import random

def generate_smart_data(num_ap=300, num_ac=500, legs_per_ac=10):
    airports = [{"id": f"AP_{i}", "mtt": 30, "disruptions": []} for i in range(num_ap)]

    aircraft = []
    flights = []
    flight_counter = 1

    for i in range(num_ac):
        start_ap = f"AP_{random.randint(0, num_ap-1)}"
        ac_id = f"AC_{i}"
        aircraft.append({
            "id": ac_id,
            "initial_location_id": start_ap,
            "disruptions": []
        })

        current_loc = start_ap
        current_time = random.randint(60, 300)

        for _ in range(legs_per_ac):
            dest = f"AP_{random.randint(0, num_ap-1)}"
            while dest == current_loc:
                dest = f"AP_{random.randint(0, num_ap-1)}"

            duration = random.randint(60, 180)
            arrival = current_time + duration

            flights.append({
                "id": f"FL_{flight_counter}",
                "origin_id": current_loc,
                "destination_id": dest,
                "departure_time": current_time,
                "arrival_time": arrival,
                "aircraft_id": None,
                "status": "Unscheduled"
            })

            flight_counter += 1
            current_loc = dest
            current_time = arrival + 30 + random.randint(30, 120)

    with open("stress_test.json", "w") as f:
        json.dump({"airports": airports, "aircraft": aircraft, "flights": flights}, f, indent=2)

generate_smart_data()