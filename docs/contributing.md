# Contributing

Welcome! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps when contributing:

## Agent Routing Matrix

Before implementing any changes, identify which agents match the files you plan to touch using the routing matrix below. Read those agent files before writing code, and note them in your PR description as:

`Consulted: <agent-name> per routing matrix`

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

## PR Description Requirements

- List which agent files you consulted, per the routing matrix.
- If you add or change logic in a path covered by an agent, summarize how your changes align with the agent's conventions.
- If you add new public functions or modules, ensure you add or update tests. Otherwise, the QA review will request changes.

## Procedural Steps

1. **Fork and clone** the repository.
2. **Create a branch** for your changes.
3. **Consult agent files** as required by the routing matrix.
4. **Make your changes** and add tests as needed.
5. **Commit** with a conventional commit message.
6. **Push** your branch and open a PR.
7. **Fill out the PR template** and note consulted agents.
8. **Address agent review feedback** if any checks request changes.

## Additional Guidance

- See [README.md](../README.md) for project overview.
- See [docs/adr/](adr/README.md) for architecture decisions.
- See [docs/reference/api.md](reference/api.md) for API details.

## Anti-Patterns

- Do not document features that don't exist yet.
- Do not skip agent consultation for in-scope files.
- Do not merge source changes without tests.
