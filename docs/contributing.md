# Contributing

Welcome! This project uses agent-driven review and automated CI checks to ensure code quality and domain correctness. Before you start, please read this guide and the [CONTRIBUTING.md](../CONTRIBUTING.md) file in the repo root.

## Agent Routing Matrix

Before implementing any changes, identify which agents match the files you plan to touch using the routing matrix below. Read those agent files before writing code, and note them in your PR description as:

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

## PR Description Format

Your PR description must include which agent files you consulted, e.g.:

```
Consulted: race-engineer.agent.md, telemetry-expert.agent.md per routing matrix
```

## Procedural Steps

1. **Identify affected files and agents.**
2. **Consult agent files as per routing matrix.**
3. **Write code and tests.**
4. **Document changes as needed.**
5. **Open PR with agent consultation noted.**
6. **Automated review workflows will run and post verdicts.**

## Additional Guidance

- See [CONTRIBUTING.md](../CONTRIBUTING.md) for full onboarding steps, code style, and test requirements.
- For architecture decisions, see [docs/adr/](adr/README.md).
- For API reference, see [docs/reference/api.md](reference/api.md).
