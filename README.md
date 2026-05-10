# Tuning Coach — Real-time Racing Telemetry & Recommendations

Tuning Coach analyzes racing telemetry and provides actionable tuning recommendations for Forza and other racing sims. Get real-time feedback on your car setup, driving style, and lap performance.

## Features
- ✅ Real-time telemetry parsing and analysis
- ✅ Automated tuning recommendations based on chassis engineering heuristics
- ✅ SimHub overlay for live feedback
- ✅ Agent-driven review workflows for code quality and security

## Quickstart

Clone and build:

```bash
git clone https://github.com/<your-org>/tuning-coach.git
cd tuning-coach
cargo build --release
# For overlay:
cd overlay
npm install
npm run build
```

## Documentation
- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Lap Validity](docs/reference/lap-validity.md)
- [Contributing](docs/contributing.md)

## Status
| Feature                | Status |
|------------------------|--------|
| Telemetry parsing      | Stable |
| Tuning recommendations | Beta   |
| Overlay UI             | Beta   |
| Agent review workflows | Stable |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for onboarding. All PRs are subject to agent-driven review checks (security, QA, telemetry, heuristics) as described in the [agent routing matrix](.github/copilot-instructions.md).

## License
MIT — see [LICENSE](LICENSE).
