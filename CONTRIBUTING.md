# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps when submitting a pull request:

## Getting Started

1. **Fork the repository**
2. **Clone your fork**
3. **Create a new branch** for your changes

```bash
git clone <your-fork-url>
cd <repo>
git checkout -b <feature-branch>
```

## Agent Routing Matrix

Before making changes, consult the relevant agent files based on the files you plan to edit. The routing matrix below shows which agents to consult:

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

When submitting your PR, note the consulted agents in your PR description:

```
Consulted: <agent-name> per routing matrix
```

## Automated Review Workflows

The following agent-driven review workflows run automatically on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

Out-of-scope PRs post `success` so checks can be required in branch protection without blocking unrelated work.

## Making Changes

- **Follow the agent conventions** in the relevant agent files.
- **Add or update tests** for any new public functions or modules.
- **Document new features** in the appropriate docs files.

## Submitting a Pull Request

1. **Push your branch**
2. **Open a pull request**
3. **Fill out the PR template**
4. **Note consulted agents**
5. **Wait for agent-driven review checks** to complete

## License

Contributions are accepted under the project's license. See [LICENSE](LICENSE).
