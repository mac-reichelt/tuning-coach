# Getting Started

> 🚧 This page is a placeholder. Detailed setup will land alongside the Phase 1
> sidecar implementation. See [PLAN.md](PLAN.md) for the roadmap.

## Prerequisites

- Forza Motorsport (2023)
- [SimHub](https://www.simhubdash.com/)
- Rust stable toolchain (for building the sidecar)

## Install

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach/sidecar
cargo build --release
./target/release/tuning-coach-sidecar
```

## Configure Forza data out

1. Settings → HUD and Gameplay → Data Out
2. Set Data Out IP Address: `127.0.0.1`
3. Set Data Out Port: `7777`
4. Enable Data Out

## Install the overlay

1. Download **`tuning-coach-overlay.zip`** from the
   [latest release](https://github.com/mac-reichelt/tuning-coach/releases/latest).
2. In SimHub open **Overlays → Import overlay** and select the zip.
3. Enable **tuning-coach** in the overlays list.
4. If you changed the sidecar `ws_listen_port`, append `?ws=ws://127.0.0.1:<port>/ws`
   to the overlay URL — see [overlay/README.md](../overlay/README.md) for details.

## Verify

With Forza on the track and the sidecar running, the overlay should show
"connected" and report incoming telemetry packet rate (~60 Hz).

## Back up your local data

The sidecar stores SQLite data at `<data_dir>/tuning-coach.db` (default:
`./data/tuning-coach.db` when started from `sidecar/`).

To make a backup, stop the sidecar, then copy `tuning-coach.db` to a safe
location. If you back up while the sidecar is running, copy
`tuning-coach.db`, `tuning-coach.db-wal`, and `tuning-coach.db-shm` together.

## Runtime config defaults

You can override config values with `TUNING_COACH_*` environment variables.
Session-state defaults:

- `TUNING_COACH_PAUSE_DEBOUNCE_MS=2000`
- `TUNING_COACH_PACKET_TIMEOUT_MS=10000`
