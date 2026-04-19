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

Copy or symlink the `overlay/` directory into your SimHub `DashTemplates`
folder, then enable it from SimHub's overlay manager.

## Verify

With Forza on the track and the sidecar running, the overlay should show
"connected" and report incoming telemetry packet rate (~60 Hz).
