# Tuning Coach — Real-time Forza tuning recommendations

Tuning Coach analyzes telemetry from Forza Motorsport and Forza Horizon to recommend chassis setup changes in real time. It integrates with SimHub overlays and supports custom tuning logic.

## Features
- ✅ Real-time telemetry parsing from Forza Motorsport/Horizon
- ✅ Automated tuning recommendations based on race engineering heuristics
- ✅ SimHub overlay integration for in-game display
- ✅ Modular agent review workflows for CI/CD, security, QA, and tuning logic

## Quickstart

Clone and build:

```bash
git clone https://github.com/<your-org>/tuning-coach.git
cd tuning-coach
cargo build --release
```

Run the sidecar:

```bash
./target/release/tuning-coach-sidecar
```

## Documentation
- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Lap Validity](docs/reference/lap-validity.md)
- [Contributing](docs/contributing.md)

## Status
| Feature                | Status   |
|------------------------|----------|
| Telemetry parsing      | Stable   |
| Tuning recommendations | Beta     |
| Overlay integration    | Stable   |
| Agent review workflows | Stable   |

## Agent Routing & Automated Review Workflows

Before implementing changes, identify which agents match the files you plan to touch using the [agent routing matrix](.github/copilot-instructions.md#agent-routing). Four automated review workflows enforce this matrix:

- `security-review`: Security-sensitive files and workflows
- `qa-review`: Source changes without accompanying tests
- `telemetry-review`: Telemetry schema and expert logic
- `heuristics-review`: Tuning logic and race engineering heuristics

These checks run on every PR and are required for merge.

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
MIT — see [LICENSE](LICENSE).
