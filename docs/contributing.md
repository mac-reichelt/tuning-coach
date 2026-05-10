# Contributing Guide

Welcome! This project uses agent-driven review workflows to maintain quality and correctness. Please read this guide before submitting changes.

## Agent Review Checks

Your pull request will trigger several automated agent reviews:

- **agent-review**: General code review
- **devops-review**: CI/CD and workflow changes
- **security-review**: Security-sensitive files or workflows
- **telemetry-review**: Telemetry schema and related files
- **heuristics-review**: Tuning logic and recommendations
- **qa-review**: Ensures test coverage for source changes

### Routing Matrix

Consult the relevant agent(s) before editing files, as described in [Agent Routing](../.github/copilot-instructions.md#agent-routing).

| Path glob | Agent(s) to consult |
|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | telemetry-expert |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | race-engineer + telemetry-expert |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | architect |
| `docs/adr/**` (new files) | architect |
| `.github/workflows/**`, `.github/actions/**` | devops-engineer + security-review |
| Any file with auth, secrets, OIDC, crypto | security-review |
| New crates, new public modules, new sidecar tests | qa-engineer |
| `overlay/**` (logic changes, not pure CSS) | qa-engineer |

## Steps to Contribute

1. **Fork and clone** the repository.
2. **Create a branch** for your changes.
3. **Make your changes** and commit with conventional messages.
4. **Open a pull request** and describe your changes. Note any agent files consulted per the routing matrix.
5. **Wait for agent review checks** to complete. Address feedback as needed.

## Documentation

Update docs if your changes affect user-facing behavior:
- [README.md](../README.md)
- [docs/](.)

## License

By contributing, you agree to license your code under the project's license.
