# Contributor Guide

This project uses agent-based review workflows to ensure domain correctness and code quality. Before contributing, please read this guide and the agent routing matrix below.

## Agent Routing Matrix

Before making changes, identify which agents match the files you plan to touch. Read those agent files before writing code, and note them in your PR description as:

`Consulted: <agent-name> per routing matrix.`

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

## Contribution Steps

1. **Fork and clone** the repository.
2. **Create a feature branch**.
3. **Consult agent files** per routing matrix.
4. **Make your changes**.
5. **Commit** using conventional commit messages.
6. **Push** and open a pull request.
7. **List consulted agents** in your PR description.
8. **Respond to agent and human reviews**.
9. **Update documentation** as needed.
10. **Merge** after all checks pass.

## Additional Information

- [CONTRIBUTING.md](../CONTRIBUTING.md) — procedural onboarding
- [docs/adr/README.md](adr/README.md) — architecture decisions
- `.github/copilot-instructions.md` — agent conventions and review process

---

For questions, open an issue or consult the agent files referenced above.
