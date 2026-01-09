# IRROPs

A deterministic, incremental aircraft scheduling engine.

## Overview

IRROPs assigns aircraft to flights while enforcing airport continuity, minimum turn times (MTT), and aircraft availability.
It supports incremental delay injection and local schedule repair without rebuilding the entire plan.

## Features
- Deterministic aircraft assignment
- Airport continuity and minimum turn times (MTT)
- Absolute-time scheduling (multi-day support)
- Aircraft availability disruptions
- Incremental delay propagation
- Partial schedule repair via reassignment
- No global re-optimization

## Testing

```bash
cargo test 
```