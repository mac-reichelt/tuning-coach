# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps when submitting a pull request:

## Getting Started

1. **Fork and clone the repository.**
2. **Create a new branch** for your changes:
   ```bash
   git checkout -b my-feature
   ```
3. **Make your changes.**
4. **Run tests** and verify your changes locally.

## Agent Routing Matrix

Before implementing any changes, identify which agents match the files you plan to touch using the routing matrix below. Read those agent files before writing code, and note them in your PR description as:

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

## Automated Agent Review Workflows

The following path-scoped review workflows enforce the agent routing matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## PR Checklist

- [ ] Consulted relevant agent(s) per routing matrix
- [ ] Added/updated tests for new public functions/modules
- [ ] Updated documentation as needed
- [ ] PR description includes consulted agent(s)

## Submitting a Pull Request

1. **Push your branch** and open a pull request.
2. **Fill out the PR template** and list consulted agents.
3. **Wait for agent review checks** to complete. Address any feedback from agent reviewers.
4. **Respond to comments** and update your PR as needed.

## Additional Resources

- [docs/contributing.md](docs/contributing.md) — extended contributor guide
- [docs/adr/](docs/adr/) — architecture decision records

## License

Contributions are accepted under the project's license. See [LICENSE](LICENSE).
