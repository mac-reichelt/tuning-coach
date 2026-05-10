# Tuning Coach — AI-powered racing telemetry and tuning

Tuning Coach analyzes racing telemetry and recommends setup changes using real-world engineering heuristics. It integrates with SimHub overlays and supports Forza, Assetto Corsa, and more.

## Features
- ✅ Real-time telemetry parsing for Forza and other sims
- ✅ AI-driven tuning recommendations based on chassis engineering
- ✅ SimHub overlay for live feedback
- ✅ Automated agent review workflows for code, security, QA, and tuning logic

## Quickstart

Clone and build:

```bash
git clone https://github.com/<your-org>/tuning-coach.git
cd tuning-coach
cargo build --release
```

Run the sidecar and overlay:

```bash
./target/release/sidecar
# Open overlay/index.html in SimHub
```

## Documentation
- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Lap Validity](docs/reference/lap-validity.md)
- [Contributing](docs/contributing.md)

## Status
| Feature                | Status   |
|-----------------------|----------|
| Forza telemetry       | Stable   |
| Assetto Corsa support | Beta     |
| SimHub overlay        | Stable   |
| AI tuning heuristics  | Beta     |
| Agent review workflows| Stable   |

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

### Agent Review Workflows

Every PR is checked by automated agent reviews for:
- Security-sensitive changes
- QA/test coverage
- Telemetry schema updates
- Tuning heuristics logic

See [docs/contributing.md](docs/contributing.md) for the agent routing matrix and review details.

## License
MIT — see [LICENSE](LICENSE).
