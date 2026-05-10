# Contributor Guide

Welcome! This guide explains how to contribute code, documentation, and tests to the project.

## Workflow Overview

1. **Fork and branch:**
   - Fork the repository.
   - Create a feature branch for your changes.

2. **Write code and tests:**
   - Follow code conventions and style.
   - Add or update tests for all new public functions, modules, or features.

3. **Update documentation:**
   - Update `README.md` and relevant files in `docs/` for any user-facing changes.
   - Document new features, APIs, or configuration options.

4. **Open a Pull Request:**
   - Fill out the PR template.
   - Note any agent files consulted per the routing matrix (see below).

## Agent Review Routing

Certain files trigger specialized agent review workflows. Before editing these files, consult the relevant agent file and note it in your PR description:

| Path glob | Agent(s) to consult | Review Workflow |
|-----------|---------------------|-----------------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | `telemetry-review` |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer`, `telemetry-expert` | `heuristics-review` |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | (manual/ADR) |
| `docs/adr/**` (new files) | `architect` | (manual/ADR) |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer`, `security-review` | `devops-review`, `security-review` |
| Auth, secrets, OIDC, crypto files | `security-review` | `security-review` |
| New crates, new public modules, new sidecar tests | `qa-engineer` | `qa-review` |
| `overlay/**` (logic changes) | `qa-engineer` | `qa-review` |

See [copilot-instructions.md](../.github/copilot-instructions.md) for full details.

## Agent Review Workflows

Automated agent reviews run on every PR:

- **security-review:** Security-sensitive changes
- **devops-review:** CI/CD and workflow changes
- **telemetry-review:** Telemetry schema and parsing
- **heuristics-review:** Tuning logic and recommendations
- **qa-review:** Source changes without test changes

Out-of-scope PRs are auto-approved. Address agent feedback before requesting human review.

## Test Coverage Discipline

- All new or changed public interfaces must have tests.
- If you change source files under `sidecar/src/` or `overlay/` without updating or adding tests, the `qa-review` check will flag your PR.

## Architecture Decisions

- Schema changes or new ADRs require review by the `architect` agent.

## Security

- Any file involving auth, secrets, OIDC, or crypto triggers the `security-review` check.

## License

Contributions are accepted under the project's license. See [LICENSE](../LICENSE).
