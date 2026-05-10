# Contributing Guide

Welcome! This project uses a multi-agent review system to ensure quality and correctness. Please follow these steps when contributing.

## Workflow

1. **Fork and clone**
2. **Create a branch**
3. **Make changes**
4. **Commit and push**
5. **Open a pull request**

## Agent Review System

Automated agent reviews check your PR for correctness, security, and test coverage. The following agent workflows are used:

- **Devops/CI**: `.github/workflows/devops-review.yml` (devops-engineer agent)
- **Security**: `.github/workflows/security-review.yml` (security-review agent)
- **Telemetry**: `.github/workflows/telemetry-review.yml` (telemetry-expert agent)
- **Heuristics/Tuning Logic**: `.github/workflows/heuristics-review.yml` (race-engineer agent)
- **QA/Test Coverage**: `.github/workflows/qa-review.yml` (qa-engineer agent)

### Agent Routing Matrix

Before making changes, check which agent(s) you need to consult based on the files you plan to edit:

| Path glob | Agent(s) to consult before editing | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` | Overlay test discipline (vitest) |

### Automated Routing

Agent review workflows are triggered automatically based on the files you change. Out-of-scope PRs post a passing check so branch protection is not blocked.

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

## Tests

- Add tests for new public functions or modules.
- QA agent will check for missing tests.

## Documentation

- Update docs for new features or changes.
- Cross-link new concepts.

## Opening a PR

- Fill out the PR template.
- Note which agent(s) you consulted per the routing matrix.
- Ensure all required checks pass.

## License

By contributing, you agree your code will be released under the project's license.
