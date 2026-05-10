# Contributing

Thank you for your interest in improving Tuning Coach! This project uses agent-driven review workflows to ensure quality and correctness. Please follow these steps:

## 1. Agent Routing Matrix

Before making changes, check which agent(s) you need to consult based on the files you plan to edit. See the routing matrix below:

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

## 2. Automated Review Workflows

The following workflows enforce agent review on every PR:

- `.github/workflows/security-review.yml` — Security-sensitive changes
- `.github/workflows/qa-review.yml` — Source changes without test changes
- `.github/workflows/telemetry-review.yml` — Telemetry schema and expert review
- `.github/workflows/heuristics-review.yml` — Tuning logic and heuristics review

Out-of-scope PRs post a passing check so branch protection is not blocked.

## 3. Steps to Contribute

1. **Fork and clone** the repository.
2. **Identify agent(s)** using the routing matrix.
3. **Consult agent files** as required.
4. **Make your changes** and commit.
5. **Open a PR**. In your PR description, note which agent(s) you consulted.
6. **Review workflows** will run automatically and post verdicts.

For more details, see [docs/contributing.md](docs/contributing.md).
