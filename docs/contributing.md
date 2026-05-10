# Contributing Guide

This project uses agent-based review workflows to ensure quality, security, and correctness. Follow these steps to contribute effectively:

## Agent Routing Matrix

Before you start, identify which agent(s) you must consult based on the files you plan to change. Read the relevant agent files and reference them in your PR description.

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

**Example PR description note:**

```
Consulted: race-engineer, telemetry-expert per routing matrix
```

## Automated Review Workflows

The following review workflows run automatically based on the files you change:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

Out-of-scope PRs post a passing check so branch protection is not blocked.

## Coding Standards

- Follow agent conventions for your area.
- Write tests for new public interfaces.
- Update documentation as needed.

## Submitting a PR

1. Fork and branch.
2. Make your changes.
3. Run tests and lint.
4. Update docs if needed.
5. Reference consulted agent(s) in your PR description.
6. Submit your PR.

## Review Process

- Automated agent reviews will run.
- Address agent and maintainer feedback.
- Merge once all checks pass.

## More Information

- [CONTRIBUTING.md](../CONTRIBUTING.md) — quickstart
- [docs/adr/README.md](adr/README.md) — architecture decisions

## License

See [LICENSE](../LICENSE).
