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

## Using the Overlay

Open `overlay/index.html` in SimHub as a browser overlay, or directly in a browser.

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

## Next Steps
- [API Reference](reference/api.md)
- [Lap Validity](reference/lap-validity.md)
