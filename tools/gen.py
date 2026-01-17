import json
import random

def generate_smart_data(num_ap=300, num_ac=500, legs_per_ac=10):
    max_simulation_time = legs_per_ac * 400

    airports = []
    for i in range(num_ap):
        ap_id = f"AP_{i}"
        disruptions = []
        # 10% chance an airport has a curfew
        if random.random() < 0.10:
            start = random.randint(500, max_simulation_time)
            duration = random.randint(120, 480)
            disruptions.append({"from": start, "to": start + duration})

        airports.append({
            "id": ap_id,
            "mtt": 30,
            "disruptions": disruptions
        })

    aircraft = []
    flights = []
    flight_counter = 1

    for i in range(num_ac):
        start_ap = f"AP_{random.randint(0, num_ap-1)}"
        ac_id = f"AC_{i}"

        ac_disruptions = []
        # 30% chance an aircraft has a maintenance disruption
        if random.random() < 0.30:
            d_start = random.randint(500, max_simulation_time)
            d_end = d_start + random.randint(60, 240)
            # 50% chance it's tied to a specific airport (Fixed-Base)
            loc = f"AP_{random.randint(0, num_ap-1)}" if random.random() > 0.5 else None
            ac_disruptions.append({
                "from": d_start,
                "to": d_end,
                "location_id": loc
            })

        aircraft.append({
            "id": ac_id,
            "initial_location_id": start_ap,
            "disruptions": ac_disruptions
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
                "status": {"Unscheduled": "Waiting"}
            })

            flight_counter += 1
            current_loc = dest
            # Add MTT plus a random buffer
            current_time = arrival + 30 + random.randint(30, 120)

    with open("stress_test.json", "w") as f:
        json.dump({
            "airports": airports,
            "aircraft": aircraft,
            "flights": flights
        }, f, indent=2)

if __name__ == "__main__":
    generate_smart_data()