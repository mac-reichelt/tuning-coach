# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps when submitting a pull request:

## Getting Started

1. **Fork the repository** and clone your fork.
2. **Create a new branch** for your changes:
   ```bash
   git checkout -b my-feature
   ```
3. **Make your changes** and commit them with clear, conventional commit messages.

## Agent Review Checks

Every pull request triggers automated agent reviews based on the files you change. These checks are enforced by GitHub Actions and must pass before your PR can be merged:

- **agent-review**: General code review.
- **devops-review**: CI/CD and workflow changes.
- **security-review**: Security-sensitive files or workflows.
- **telemetry-review**: Telemetry schema and related files.
- **heuristics-review**: Tuning logic and recommendations.
- **qa-review**: Ensures test coverage for source changes.

### Routing Matrix

Before implementing changes, consult the relevant agent(s) based on the files you plan to touch. See [Agent Routing](.github/copilot-instructions.md#agent-routing) for details.

| Path glob | Agent(s) to consult before editing |
|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | telemetry-expert |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | race-engineer + telemetry-expert |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | architect |
| `docs/adr/**` (new files) | architect |
| `.github/workflows/**`, `.github/actions/**` | devops-engineer + security-review |
| Any file with auth, secrets, OIDC, crypto | security-review |
| New crates, new public modules, new sidecar tests | qa-engineer |
| `overlay/**` (logic changes, not pure CSS) | qa-engineer |

### Automated Checks

The following workflows enforce the routing matrix:

- `.github/workflows/security-review.yml`
- `.github/workflows/qa-review.yml`
- `.github/workflows/telemetry-review.yml`
- `.github/workflows/heuristics-review.yml`

Out-of-scope PRs post a passing check so unrelated work is not blocked.

## Submitting a Pull Request

1. **Push your branch** to your fork.
2. **Open a pull request** against the main repository.
3. **Describe your changes** clearly. If you consulted agent files per the routing matrix, note them in your PR description (e.g., `Consulted: race-engineer per routing matrix`).
4. **Wait for agent review checks** to complete. Address any feedback.

## Documentation

If your changes affect user-facing behavior, update the relevant documentation:

- [README.md](README.md)
- [docs/](docs/)
- [docs/contributing.md](docs/contributing.md)

## License

By contributing, you agree that your code will be released under the project's license.
