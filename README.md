# Tuning Coach — Real-time Forza tuning overlay

Tuning Coach is a real-time tuning overlay for Forza Motorsport and Forza Horizon, powered by a Rust sidecar and a web frontend. It analyzes live telemetry and recommends setup changes to optimize lap times. The SimHub dashboard bundle lets you import the overlay into SimHub for seamless integration.

## Features
- ✅ Real-time telemetry analysis — parses Forza UDP packets and computes tuning recommendations
- ✅ Web overlay UI — drag-and-drop panels, live lap status, dyno graph, and telemetry inspector
- ✅ SimHub dashboard bundle — importable .djson for easy setup
- ✅ Embedded frontend — no static file server needed; the sidecar serves the overlay UI

## Quickstart

**Clone and build:**

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
cargo run --release --manifest-path sidecar/Cargo.toml
```

The sidecar listens on UDP `127.0.0.1:7777` (telemetry) and HTTP/WebSocket `127.0.0.1:7778` (overlay + API), and serves the overlay directly — no separate static file server is needed, and you should not open `sidecar/web/index.html` from disk.

With the sidecar running, open the overlay at `http://127.0.0.1:7778/`:

![Overlay screenshot](docs/img/overlay-screenshot.png)

**SimHub users:** Import the dashboard bundle from `simhub/`:
- `tuning-coach.djson`
- `tuning-coach.djson.metadata`
- `tuning-coach.djson.png`

## Documentation
- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Lap Validity](docs/reference/lap-validity.md)

## Status
- Sidecar + web frontend: **beta** — stable API, frequent improvements
- SimHub dashboard bundle: **stable** — changes rarely

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
MIT — see [LICENSE](LICENSE).
