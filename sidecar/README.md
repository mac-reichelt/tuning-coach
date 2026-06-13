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

## Replaying a packet capture

Instead of listening for live UDP, the sidecar can replay a recorded Forza
telemetry capture (`.pcapng` or legacy `.pcap`) into the same pipeline that
drives the overlay, heuristics, and WebSocket API. This is useful for
developing and validating the web view without a running game, and for
analyzing recorded sessions.

```bash
# Play a capture once, in real time
tuning-coach-sidecar --replay tests/fixtures/lap_validity/fm-training.pcapng

# Loop forever at 4x speed
tuning-coach-sidecar --replay capture.pcapng --replay-loop --replay-speed 4
```

While replaying, the live UDP listener is disabled. The parser keeps every
UDP payload that decodes as a valid Forza Sled/Dash packet (any unrelated
traffic in the capture is ignored) and paces playback using the capture's own
timestamps. Equivalent config keys / env vars: `replay_file`
(`TUNING_COACH_REPLAY_FILE`), `replay_loop` (`TUNING_COACH_REPLAY_LOOP`),
`replay_speed` (`TUNING_COACH_REPLAY_SPEED`); CLI flags take precedence.

## Lap validity detection defaults

- `TUNING_COACH_REWIND_BACKWARD_JUMP_M` (default: `50.0`)
- `TUNING_COACH_SESSION_RESET_RACE_TIME_WINDOW_S` (default: `2.0`)
- `TUNING_COACH_PIT_ENTRY_SPEED_THRESHOLD_KPH` (default: `20.0`)
- `TUNING_COACH_PIT_ENTRY_DWELL_S` (default: `3.0`)
- `TUNING_COACH_PIT_EXIT_SPEED_THRESHOLD_KPH` (default: `40.0`)
- `TUNING_COACH_PIT_EXIT_DWELL_S` (default: `1.0`)
- `TUNING_COACH_OFF_TRACK_WINDOW_MS` (default: `500`)
- `TUNING_COACH_OFF_TRACK_MIN_WHEELS` (default: `2`)
- `TUNING_COACH_SURFACE_RUMBLE_THRESHOLD` (default: `0.35`)
- `TUNING_COACH_SURFACE_RUMBLE_WINDOW_PACKETS` (default: `5`)
- `TUNING_COACH_WALL_CONTACT_G_THRESHOLD` (default: `10.0`)
- `TUNING_COACH_CORNER_CUT_SPEED_KPH_MIN` (default: `30.0`)
- `TUNING_COACH_CORNER_CUT_COMBINED_SLIP_THRESHOLD` (default: `1.0`)
- `TUNING_COACH_CORNER_CUT_MAX_ABS_STEER_NORM` (default: `0.07874016`)
