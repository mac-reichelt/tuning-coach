# Contributing

This project uses agent-driven review workflows to ensure quality, security, and correctness. Please follow these steps when contributing:

## 1. Agent Routing Matrix

Before making changes, consult the agent routing matrix to determine which agent(s) you need to read and reference in your PR:

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

## 2. Automated Review Workflows

Your PR will trigger automated review workflows based on the files you change:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

Out-of-scope PRs post a passing check so branch protection is not blocked.

## 3. Procedural Steps

- **Fork and clone** the repository.
- **Create a branch** for your changes.
- **Consult agent files** in `.github/agents/` as per the routing matrix.
- **Document agent consultation** in your PR description: `Consulted: <agent-name> per routing matrix`.
- **Add or update tests** for any new or changed public functions/modules.
- **Submit your PR** and respond to agent review feedback.

## 4. Additional Guidelines

- **ADR:** New architecture decisions go in `docs/adr/`.
- **Security:** Changes involving authentication, secrets, or cryptography require security review.
- **CI/CD:** Workflow or action changes require devops and security review.
- **Testing:** All new public interfaces must have corresponding tests.

## Reference
- [docs/contributing.md](docs/contributing.md)
- [Agent files](.github/agents/)
- [Architecture Decision Records](docs/adr/)
- [README.md](README.md)
