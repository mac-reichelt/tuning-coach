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
- `TUNING_COACH_OFF_TRACK_WINDOW_MS` (default: `500`)
- `TUNING_COACH_OFF_TRACK_MIN_WHEELS` (default: `2`)
- `TUNING_COACH_SURFACE_RUMBLE_THRESHOLD` (default: `0.35`)
- `TUNING_COACH_SURFACE_RUMBLE_WINDOW_PACKETS` (default: `5`)
- `TUNING_COACH_WALL_CONTACT_G_THRESHOLD` (default: `10.0`)
- `TUNING_COACH_CORNER_CUT_SPEED_KPH_MIN` (default: `30.0`)
- `TUNING_COACH_CORNER_CUT_COMBINED_SLIP_THRESHOLD` (default: `1.0`)
- `TUNING_COACH_CORNER_CUT_MAX_ABS_STEER_NORM` (default: `0.07874016`)
