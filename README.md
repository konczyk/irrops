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
- Aircraft availability disruptions with an optional location constraint
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

## Sample TUI session

```shell
cargo run -r -- -s data/stress_test.json 

Tower online. Loaded flights from data/stress_test.json
>> ?

Available Commands:
  ls [status]         - List all flights in a table or filter by status: u - unscheduled, s - scheduled, d - delayed
  delay <id> <m>      - Inject <m> minutes of delay into flight <id>
  curfew <id> <m> <m> - Inject a curfew from <m> to <m> minutes into airport <id>
  recover             - Re-run assignment to repair unscheduled flights
  stats               - Display summary statistics
  help / ?            - Show this help menu
  exit / quit         - Exit the simulator
  
>> stats

Fleet Utilization Summary:
---------------------------
Scheduled:                          4724 (94.5%)
Delayed:                            0 (0.0%)
Unscheduled (Waiting):              276 (5.5%)
Unscheduled (Max Delay Exceeded):   0 (0.0%)
Unscheduled (Airport Curfew):       0 (0.0%)
Unscheduled (Aircraft Maintenance): 0 (0.0%)
Unscheduled (Broken Chain):         0 (0.0%)
---------------------------
Total Flights: 5000

>> delay FL_1922 1000
Applied delay.
Flights delayed: 2
Flights unscheduled: 7

>> curfew AP_75 1000 1500
Applied airport curfew.
Flights unscheduled: 40

>> stats

Fleet Utilization Summary:
---------------------------
Scheduled:                          4675 (93.5%)
Delayed:                            2 (0.0%)
Unscheduled (Waiting):              276 (5.5%)
Unscheduled (Max Delay Exceeded):   0 (0.0%)
Unscheduled (Airport Curfew):       8 (0.2%)
Unscheduled (Aircraft Maintenance): 0 (0.0%)
Unscheduled (Broken Chain):         39 (0.8%)
---------------------------
Total Flights: 5000

>> recover
Recovery cycle complete.

>> stats

Fleet Utilization Summary:
---------------------------
Scheduled:                          4720 (94.4%)
Delayed:                            2 (0.0%)
Unscheduled (Waiting):              254 (5.1%)
Unscheduled (Max Delay Exceeded):   0 (0.0%)
Unscheduled (Airport Curfew):       4 (0.1%)
Unscheduled (Aircraft Maintenance): 0 (0.0%)
Unscheduled (Broken Chain):         20 (0.4%)
---------------------------
Total Flights: 5000

```