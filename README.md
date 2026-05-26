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
git clone https://github.com/<your-org>/tuning-coach.git
cd tuning-coach
cargo run --release --manifest-path sidecar/Cargo.toml
```

Open the overlay in SimHub or a browser:

```bash
# SimHub: add overlay/index.html as a browser overlay
# Browser: open overlay/index.html
```

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
