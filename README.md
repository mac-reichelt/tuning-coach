# Tuning Coach — Real-time Racing Telemetry & Recommendations

Tuning Coach analyzes racing telemetry and provides actionable tuning recommendations for Forza and other racing sims. It uses agent-driven review workflows to ensure correctness, security, and test coverage.

## Features
- ✅ Real-time telemetry parsing and analysis
- ✅ Automated tuning recommendations based on chassis engineering heuristics
- ✅ Agent-driven review workflows for code, security, and test coverage
- ✅ Easy integration with SimHub overlays

## Quickstart

Clone and build:

```bash
git clone <repo-url>
cd tuning-coach
cargo build
```

## Documentation
- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Contributing](docs/contributing.md)

## Status
| Feature | Status |
|---------|-------|
| Telemetry parsing | Stable |
| Tuning recommendations | Beta |
| Agent review workflows | Stable |
| Overlay integration | Beta |

## Agent Review Workflows

Every pull request triggers automated agent reviews:

- **agent-review**: General code review
- **devops-review**: CI/CD and workflow changes
- **security-review**: Security-sensitive files or workflows
- **telemetry-review**: Telemetry schema and related files
- **heuristics-review**: Tuning logic and recommendations
- **qa-review**: Ensures test coverage for source changes

See [CONTRIBUTING.md](CONTRIBUTING.md) and [Agent Routing](.github/copilot-instructions.md#agent-routing) for details.

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
MIT — see [LICENSE](LICENSE).
