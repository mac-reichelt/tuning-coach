# Getting Started

## Prerequisites

- Rust (latest stable)
- Node.js (for frontend dev/test tooling; not required to run)
- Forza Motorsport or Forza Horizon
- SimHub (optional, for dashboard integration)

## Clone and Build

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
cargo run --release --manifest-path sidecar/Cargo.toml
```

## Running the Sidecar

The sidecar listens on UDP `127.0.0.1:7777` for telemetry and HTTP/WebSocket `127.0.0.1:7778` for the overlay UI and API. The web frontend is embedded in the sidecar binary and served directly — you do **not** need to run a separate static file server or open the HTML from disk.

## Accessing the Overlay UI

- **Browser:** Open [http://127.0.0.1:7778/](http://127.0.0.1:7778/) directly.
- **SimHub:** Add a browser/dash overlay pointing to [http://127.0.0.1:7778/](http://127.0.0.1:7778/).

## SimHub Dashboard Bundle

To use the SimHub dashboard, import the bundle from `simhub/`:

- `tuning-coach.djson`
- `tuning-coach.djson.metadata`
- `tuning-coach.djson.png`

These files provide a SimHub dashboard item that embeds the overlay UI via a browser pointing to the sidecar's HTTP origin.

## Directory Structure

- `sidecar/web/` — Web frontend (HTML/CSS/JS), embedded and served by the sidecar
- `simhub/` — SimHub dashboard bundle (.djson, metadata, PNG)

See [docs/adr/0004-overlay-frontend-relocation.md](adr/0004-overlay-frontend-relocation.md) for architectural details.

## Next Steps

- [Lap Validity Reference](reference/lap-validity.md)
- [API Reference](reference/api.md)
