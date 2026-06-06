# Contributing

Welcome! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Before you start, please read this guide and follow the agent routing matrix below.

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
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |

## Automated Review Workflows

Four path-scoped review workflows enforce this matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `sidecar/web/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## How to Contribute

1. **Identify affected agents:** Use the routing matrix above to determine which agent(s) you need to consult based on the files you plan to change.
2. **Read agent files:** Review the relevant agent files in `.github/agents/` before making changes.
3. **Document agent consultation:** In your PR description, note which agents you consulted, e.g., `Consulted: race-engineer per routing matrix`.
4. **Follow review workflow:** Your PR will trigger the appropriate automated review workflows. Address any feedback from agent reviews.
5. **Add or update tests:** If you change source files under `sidecar/src/` or `sidecar/web/`, ensure you add or update corresponding test files. Otherwise, the QA review workflow may request changes.
6. **Submit your PR:** Follow the [CONTRIBUTING.md](../CONTRIBUTING.md) for procedural details.

## Additional Guidelines

- **Architecture Decisions:** New ADRs go in `docs/adr/` and require architect review.
- **Security:** Any changes involving authentication, secrets, or cryptography require security review.
- **CI/CD:** Changes to workflows or actions require devops and security review.
- **Testing:** All new public functions or modules must have corresponding tests.

## Reference
- [Agent files](../.github/agents/)
- [Architecture Decision Records](adr/)
- [README](../README.md)
