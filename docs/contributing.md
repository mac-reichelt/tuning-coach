# Contributing Guide

This project uses agent-driven review workflows to ensure code quality, correctness, and security. Please follow these guidelines when contributing.

## Agent Routing Matrix

Before making changes, consult the agent routing matrix to determine which agent files you must read and reference. The matrix is documented in [`.github/copilot-instructions.md`](../.github/copilot-instructions.md):

| Path glob | Agent(s) to consult |
|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` |
| `docs/adr/**` (new files) | `architect` |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` |
| New crates, new public modules, new sidecar tests | `qa-engineer` |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` |

## Automated Review Checks

Every pull request triggers agent-driven review workflows:

- **security-review**: Security-sensitive changes
- **qa-review**: Source changes without accompanying tests
- **telemetry-review**: Telemetry schema and expert logic
- **heuristics-review**: Tuning logic and recommendations

If your PR is in scope for any of these checks, agent review is required before merge. Out-of-scope PRs are auto-approved.

## PR Description

- Reference consulted agent files per the routing matrix.
- Describe your changes and rationale.
- Link related issues.

## Testing

Run all tests before submitting:

```bash
cargo test
# Overlay tests:
npm test
```

## Documentation

Update `/docs/`, `README.md`, or ADRs if your change affects user-facing behavior.

## Code Style

- Follow Rust and TypeScript formatting conventions.
- Use conventional commit messages.

## License

By submitting code, you agree it can be released under the project license.
