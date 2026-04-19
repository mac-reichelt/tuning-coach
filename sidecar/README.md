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
