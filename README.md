# Tuning Coach — Real-time Racing Telemetry & Setup Guidance

Tuning Coach helps you optimize your car setup with live telemetry, actionable recommendations, and seamless SimHub overlay integration.

![Tuning Coach overlay screenshot](docs/assets/overlay-screenshot.png)

## Features
- ✅ Real-time telemetry parsing and analysis
- ✅ SimHub-importable overlay bundle for instant dashboards
- ✅ Actionable setup recommendations

## Quickstart

Get up and running in 5 minutes:

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach/sidecar
cargo build --release
./target/release/tuning-coach-sidecar
```

### Install the SimHub Overlay Bundle

1. Download the latest `tuning-coach-overlay-bundle.zip` from [Releases](https://github.com/mac-reichelt/tuning-coach/releases) or find it in `overlay/`.
2. In SimHub, go to **Overlays > Import Overlay**.
3. Select the zip file and follow the prompts.

See [Getting Started](docs/getting-started.md#install-the-simhub-overlay-bundle-v013) for details.

## Documentation
- [Getting Started](docs/getting-started.md)
- [Configuration](docs/configuration.md)
- [API Reference](docs/reference/api.md)

## Status
| Feature                | Status   |
|------------------------|----------|
| Telemetry parsing      | Stable   |
| SimHub overlay bundle  | Beta     |
| Setup recommendations  | Beta     |

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
MIT — see [LICENSE](LICENSE).
