# Contributing

Welcome! This project uses agent-driven review and a routing matrix to ensure every change gets the right expert eyes. Please follow these steps when contributing:

## Agent Routing Matrix

Before you make changes, identify which agents match the files you plan to touch. Consult the agent files listed below and note them in your PR description as:

```
Consulted: <agent-name> per routing matrix
```

| Path glob | Agent(s) to consult before editing | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` | Overlay test discipline (vitest) |

## Automated Review Workflows

Four path-scoped review workflows enforce this matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## How to Contribute

1. **Fork and clone** the repo.
2. **Create a branch** for your change.
3. **Identify agent(s)** per the routing matrix above.
4. **Consult agent files** as needed.
5. **Make your changes** and commit.
6. **Open a PR**. In your PR description, list consulted agents.
7. **Review checks** will run automatically. Address any feedback from agents.

## Code Style & Tests

- Follow the code style in existing files.
- Add tests for new public functions, modules, or features.
- If you change source files under `sidecar/src/` or `overlay/` without updating tests, the `qa-review` workflow will flag missing coverage.

## Docs & ADRs

- Update documentation in `/docs` as needed.
- New ADRs go in `docs/adr/` and require architect review.

## CI & Security

- Changes to workflows or security-sensitive files trigger `devops-review` and `security-review` checks.

## Questions?

Open an issue or ask in discussions.
