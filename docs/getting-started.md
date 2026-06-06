# Getting Started

## Prerequisites
- Forza Motorsport or Horizon running
- SimHub (for overlay integration) or a browser
- Rust toolchain (for sidecar)

## Running the Sidecar

Build and run the sidecar:

```bash
cargo run --release --manifest-path sidecar/Cargo.toml
```

The sidecar listens on:
- UDP telemetry: `127.0.0.1:7777`
- HTTP + WebSocket overlay/API: `127.0.0.1:7778`

## Using the Overlay

The sidecar serves the overlay over HTTP — there is no separate static file
server. Do not open `sidecar/web/index.html` from disk; the overlay loads its
assets and opens its WebSocket connection relative to the sidecar origin, so it
only works when served by the sidecar.

With the sidecar running, open the overlay at `http://127.0.0.1:7778/`:

- **Browser:** open `http://127.0.0.1:7778/` directly.
- **SimHub:** add a browser/dash overlay pointing to `http://127.0.0.1:7778/`.

### Overlay Controls

Top-right cluster:
- **HUD** — Toggle the main telemetry HUD
- **Dyno** — Open the guided Dyno panel
- **Raw Data** — Open the Raw Telemetry panel

### Dyno Panel

1. Click **Dyno** to open the panel.
2. Follow the instructions:
   - Stop on a straight, select the target gear, turn off traction control.
   - Hold stopped for 3 seconds to arm.
   - Apply full throttle and hold until the rev limiter.
   - Results (power/torque curves) will appear when complete.
3. Drag the panel by its header to reposition.

### Raw Telemetry Panel

- Click **Raw Data** to open.
- Inspect all Forza packet fields live.
- Drag the panel by its header to reposition.

### Customizing the overlay UI

The HTML/CSS/JS in `sidecar/web/` is a **reference example** that you can edit
or fork. Debug builds (`cargo run`) read files from disk, so edits to
`index.html`, `src/`, or `styles/` are visible on the next browser reload
without rebuilding. Release builds embed the files at compile time.

## Next Steps
- [API Reference](reference/api.md)
- [Lap Validity](reference/lap-validity.md)
