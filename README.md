# Tuning Coach — Real-time Forza telemetry and tuning assistant

Tuning Coach overlays live telemetry and tuning recommendations on Forza Motorsport and Horizon. Now includes a guided in-session dynamometer (Dyno) panel and a Raw Telemetry viewer for advanced users.

![Overlay screenshot](docs/img/overlay-screenshot.png)

## Features
- ✅ Real-time telemetry HUD — speed, RPM, tire temps, lap status
- ✅ Guided Dyno panel — collect power/torque curves in-session
- ✅ Raw Telemetry panel — inspect all Forza packet fields live
- ✅ Tuning recommendations — actionable setup advice
- ✅ Lap validity badge — see clean/dirty status instantly

## Quickstart

Clone and run the sidecar:

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
cargo run --release --manifest-path sidecar/Cargo.toml
```

The sidecar listens on UDP `127.0.0.1:7777` (telemetry) and HTTP/WebSocket
`127.0.0.1:7778` (overlay + API), and serves the overlay directly — no separate
static file server is needed, and you should not open `sidecar/web/index.html` from
disk.

With the sidecar running, open the overlay at `http://127.0.0.1:7778/`:

- **Browser:** open `http://127.0.0.1:7778/` directly.
- **SimHub:** add a browser/dash overlay pointing to `http://127.0.0.1:7778/`.

## Overlay Controls
- **HUD** — Toggle the main telemetry HUD
- **Dyno** — Open the guided Dyno panel; follow instructions to collect a power/torque curve
- **Raw Data** — Open the Raw Telemetry panel to inspect all fields

## Documentation
- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Lap Validity](docs/reference/lap-validity.md)

## Status
| Feature         | Stability |
|-----------------|----------|
| Telemetry HUD   | Stable   |
| Dyno panel      | Beta     |
| Raw Telemetry   | Beta     |
| Recommendations | Stable   |

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
MIT — see [LICENSE](LICENSE).
