# IRROPs

A deterministic, incremental aircraft scheduling engine.

## Overview

IRROPs assigns aircraft to flights while enforcing airport continuity, minimum turn times (MTT), and aircraft availability.
It supports incremental delay injection and local schedule repair without rebuilding the entire plan.  
Includes a simple interactive TUI for exploring schedules and simulating disruptions.

## Features
- Deterministic aircraft assignment
- Airport continuity and minimum turn times (MTT)
- Absolute-time scheduling (multi-day support)
- Aircraft availability disruptions
- Incremental delay propagation
- Partial schedule repair via reassignment
- No global re-optimization
- Interactive terminal UI (REPL-style)
- Load scenarios from JSON
- Human-readable multi-day time display

## Testing

```bash
cargo test 
```

## TUI Usage

```bash
cargo run -- --scenario data/default.json
```

Commands:

- `ls` - list all flights and their current status
- `delay <flight_id> <minutes>` - inject a delay into a flight
- `recover` - re-run assignment to repair unscheduled flights
- `exit` / `quit` - leave the simulator

## Sample TUI session

```shell
cargo run --
Tower online. Loaded flights from data/default.json

> ls
╭────────┬─────────────┬────────┬─────────────┬────────────────┬──────────────┬───────────╮
│ id     │ aircraft_id │ origin │ destination │ departure_time │ arrival_time │ status    │
├────────┼─────────────┼────────┼─────────────┼────────────────┼──────────────┼───────────┤
│ FL-101 │ ALPHA       │ WAW    │ KRK         │ DAY1 01:40     │ DAY1 03:20   │ Scheduled │
│ FL-102 │ ALPHA       │ KRK    │ GDN         │ DAY1 08:20     │ DAY1 12:30   │ Scheduled │
│ FL-201 │ ALPHA       │ GDN    │ WAW         │ DAY1 15:00     │ DAY1 17:30   │ Scheduled │
╰────────┴─────────────┴────────┴─────────────┴────────────────┴──────────────┴───────────╯

> delay FL-101 200
Applied delay. 3 flights became unscheduled.

> ls
╭────────┬─────────────┬────────┬─────────────┬────────────────┬──────────────┬─────────────╮
│ id     │ aircraft_id │ origin │ destination │ departure_time │ arrival_time │ status      │
├────────┼─────────────┼────────┼─────────────┼────────────────┼──────────────┼─────────────┤
│ FL-101 │ ---         │ WAW    │ KRK         │ DAY1 05:00     │ DAY1 06:40   │ Unscheduled │
│ FL-102 │ ---         │ KRK    │ GDN         │ DAY1 08:20     │ DAY1 12:30   │ Unscheduled │
│ FL-201 │ ---         │ GDN    │ WAW         │ DAY1 15:00     │ DAY1 17:30   │ Unscheduled │
╰────────┴─────────────┴────────┴─────────────┴────────────────┴──────────────┴─────────────╯

> recover 
Recovery cycle complete.

> ls
╭────────┬─────────────┬────────┬─────────────┬────────────────┬──────────────┬───────────╮
│ id     │ aircraft_id │ origin │ destination │ departure_time │ arrival_time │ status    │
├────────┼─────────────┼────────┼─────────────┼────────────────┼──────────────┼───────────┤
│ FL-101 │ BRAVO       │ WAW    │ KRK         │ DAY1 05:00     │ DAY1 06:40   │ Scheduled │
│ FL-102 │ BRAVO       │ KRK    │ GDN         │ DAY1 08:20     │ DAY1 12:30   │ Scheduled │
│ FL-201 │ BRAVO       │ GDN    │ WAW         │ DAY1 15:00     │ DAY1 17:30   │ Scheduled │
╰────────┴─────────────┴────────┴─────────────┴────────────────┴──────────────┴───────────╯

> 
```