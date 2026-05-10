# Tuning Coach — AI-powered race engineering for Forza

Tuning Coach automates telemetry-driven tuning advice for Forza, using real-world race engineering heuristics. It integrates with SimHub overlays and supports agent-driven code review workflows.

## Features
- ✅ Real-time telemetry parsing and tuning recommendations
- ✅ SimHub overlay integration
- ✅ Agent-driven review workflows for CI/CD, security, QA, and heuristics

## Quickstart

```bash
git clone https://github.com/<your-org>/tuning-coach.git
cd tuning-coach
# Build and run the sidecar
cargo build --release
./target/release/sidecar
```

## Documentation
- [Getting Started](docs/getting-started.md)
- [Configuration](docs/configuration.md)
- [API Reference](docs/reference/api.md)
- [Contributing](docs/contributing.md)

## Status
| Feature                | Status   |
|------------------------|----------|
| Telemetry parsing      | Stable   |
| Tuning heuristics      | Beta     |
| SimHub overlay         | Stable   |
| Agent review workflows | Beta     |

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/contributing.md](docs/contributing.md) for agent routing and review workflow details.

## License
MIT — see [LICENSE](LICENSE).
