# Getting Started

## Installation

**Clone the repo:**

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
```

**Build and run the sidecar:**

```bash
cargo run --release --manifest-path sidecar/Cargo.toml
```

The sidecar listens on:
- UDP `127.0.0.1:7777` — Forza telemetry
- HTTP/WebSocket `127.0.0.1:7778` — overlay UI + API

## Using the Overlay

The sidecar serves the overlay over HTTP — there is no separate static file server. Do not open `sidecar/web/index.html` from disk; the overlay loads its assets and opens its WebSocket connection relative to the sidecar origin, so it only works when served by the sidecar.

**Open the overlay:**

```
http://127.0.0.1:7778/
```

**SimHub users:** Import the dashboard bundle from `simhub/`:
- `tuning-coach.djson`
- `tuning-coach.djson.metadata`
- `tuning-coach.djson.png`

## Overlay Features

Top-left cluster:
- Live lap status (valid/invalid, sector times)
- Tuning recommendations (suspension, gearing, aero, etc.)
- Dyno graph (resettable)

Top-right cluster:
- Inspect all Forza packet fields live.
- Drag the panel by its header to reposition.

### Customizing the overlay UI

The HTML/CSS/JS in `sidecar/web/` is a **reference example** that you can edit or fork. Debug builds (`cargo run`) read files from disk, so edits to `index.html`, `src/`, or `styles/` are visible on the next browser reload without rebuilding. Release builds embed the files at compile time.

## Next Steps
- [API Reference](reference/api.md)
- [Lap Validity](reference/lap-validity.md)
