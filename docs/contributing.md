# Contributor Guide

This project uses agent-driven review workflows to maintain quality, security, and correctness. Follow these steps to contribute:

## Workflow Overview

1. **Fork** and clone the repository.
2. **Create** a branch for your changes.
3. **Identify** which agent(s) to consult based on the files you plan to edit (see routing matrix below).
4. **Consult** the relevant agent files before writing code.
5. **Note** consulted agents in your PR description: `Consulted: <agent-name> per routing matrix.`
6. **Commit** changes with [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
7. **Push** and open a PR.

## Agent Routing Matrix

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

## Commit Messages

- Use Conventional Commits for all changes.
- Example: `fix: correct lap validity calculation`

## Documentation

- Update `/docs` and `README.md` as needed.
- Follow [tech-writer conventions](docs/contributing.md).

## License

By submitting a PR, you agree your contributions are licensed under the project's SPDX license.
