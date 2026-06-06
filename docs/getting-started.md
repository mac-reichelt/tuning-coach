# Getting Started

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [SimHub](https://www.simhubdash.com/) (optional, for dashboard integration)

## Clone and Build

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
cargo run --release --manifest-path sidecar/Cargo.toml
```

The sidecar listens on UDP `127.0.0.1:7777` for telemetry and HTTP/WebSocket `127.0.0.1:7778` for the overlay UI and API. The web frontend is served directly by the sidecar—do not open `sidecar/web/index.html` from disk.

## Using the Overlay

- **Browser:** Open [http://127.0.0.1:7778/](http://127.0.0.1:7778/) in your browser.
- **SimHub:**
  - Import the dashboard bundle from the `simhub/` directory:
    - `tuning-coach.djson`
    - `tuning-coach.djson.metadata`
    - `tuning-coach.djson.png`
  - Add a browser/dash overlay pointing to `http://127.0.0.1:7778/`.

## Overlay Controls

- **HUD** — Toggle the main telemetry HUD
- **Dyno** — Open the guided Dyno panel; follow instructions to collect a power/torque curve
- **Raw Data** — Open the Raw Telemetry panel to inspect all fields

## Next Steps

- [Lap Validity Reference](reference/lap-validity.md)
- [API Reference](reference/api.md)
