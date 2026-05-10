# Contributing

Welcome! This project uses agent-driven review workflows to ensure every change is correct, secure, and well-tested. Before you start, please read this guide and follow the agent routing matrix below.

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

## How to Contribute

1. **Fork and clone** the repo.
2. **Create a branch** for your change.
3. **Consult the agent(s)** per the routing matrix above.
4. **Make your changes.**
5. **Add or update tests** for any new public functions or modules.
6. **Document your change** if it affects user-facing behavior.
7. **Open a PR**. In your PR description, list the consulted agent(s) and affected files.
8. **Wait for agent reviews**. The automated workflows will post review verdicts as PR comments and check runs.
9. **Address feedback** from agents and maintainers.

## PR Description Template

Include this in your PR description:

```
Consulted: <agent-name> per routing matrix
Affected files: <list>
Summary: <what/why>
```

## Testing

- All new public functions must have tests.
- Overlay logic changes must have vitest coverage.
- Sidecar changes must have cargo test coverage.

## Docs

- Update `docs/` for any user-facing changes.
- Update ADRs for architectural decisions.

## Security

- Any change involving auth, secrets, or crypto must be reviewed by `security-review`.

## Questions?

Open an issue or ask in discussions.

---

See [CONTRIBUTING.md](../CONTRIBUTING.md) for the procedural checklist.
