# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps when submitting a pull request:

## Getting Started

1. **Fork and clone** the repository.
2. **Create a new branch** for your changes:
   ```bash
   git checkout -b my-feature
   ```
3. **Make your changes** and commit them with clear, conventional commit messages.

## Agent Routing and Review Checks

Before implementing changes, consult the agent routing matrix to determine which agent files you must read and reference in your PR description. The routing matrix is documented in [`.github/copilot-instructions.md`](.github/copilot-instructions.md) and summarized below:

| Path glob | Agent(s) to consult before editing |
|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` |
| `docs/adr/**` (new files) | `architect` |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` |
| New crates, new public modules, new sidecar tests | `qa-engineer` |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` |

### Automated Review Workflows

The following agent-driven review checks run automatically on every PR:

- **security-review**: Security-sensitive changes
- **qa-review**: Source changes without accompanying tests
- **telemetry-review**: Telemetry schema and expert logic
- **heuristics-review**: Tuning logic and recommendations

Out-of-scope PRs are auto-approved by these checks. In-scope PRs require agent review and approval before merge.

## PR Description Requirements

- **Reference consulted agent files** in your PR description, e.g.:
  ```
  Consulted: race-engineer.agent.md per routing matrix
  ```
- **Describe your changes** and rationale.
- **Link related issues** if applicable.

## Running Tests

Run all tests before submitting:
```bash
cargo test
# For overlay:
npm test
```

## Documentation

If your change affects user-facing behavior, update the relevant docs in `/docs/`, `README.md`, or ADRs as needed.

## Code Style

- Follow Rust and TypeScript formatting conventions.
- Use conventional commit messages.

## License

By submitting code, you agree it can be released under the project license.
