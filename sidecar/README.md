# sidecar

Rust sidecar for `tuning-coach`. Ingests Forza Motorsport UDP telemetry, runs
the heuristics engine, persists to SQLite, and serves the SimHub overlay over
WebSocket.

See [docs/PLAN.md](../docs/PLAN.md) for the roadmap.

## Build

```bash
cargo build --release
./target/release/tuning-coach-sidecar
```

## Lap validity detection defaults

- `TUNING_COACH_REWIND_BACKWARD_JUMP_M` (default: `50.0`)
- `TUNING_COACH_SESSION_RESET_RACE_TIME_WINDOW_S` (default: `2.0`)
