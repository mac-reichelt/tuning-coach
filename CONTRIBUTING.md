# Contributing

Thank you for your interest in contributing! This project uses agent-based review workflows to ensure code quality, security, and correctness. Please follow these steps when submitting a pull request:

## 1. Identify Agent Routing

Before making changes, consult the agent routing matrix below to determine which agent files you must read and reference in your PR description. This ensures your changes are reviewed by the appropriate domain experts.

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

**In your PR description, note which agent(s) you consulted:**

```
Consulted: <agent-name> per routing matrix
```

## 2. Automated Review Workflows

The following path-scoped review workflows enforce agent routing on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four workflows use the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## 3. Coding Standards

- Follow the conventions in the agent files you consult.
- Write tests for new public functions, modules, or features.
- Update documentation as needed (README, /docs, ADRs).

## 4. Submitting a PR

- Fork the repository and create a branch.
- Make your changes.
- Run tests and lint checks.
- Update documentation if needed.
- Reference consulted agent(s) in your PR description.
- Submit your pull request.

## 5. Review Process

- Automated agent reviews will run based on the files you changed.
- Address any feedback from agent reviews and maintainers.
- Once all required checks pass, your PR can be merged.

## 6. Additional Resources

- [docs/contributing.md](docs/contributing.md) — extended contributing guide
- [docs/adr/README.md](docs/adr/README.md) — architecture decision records

## License

Contributions are accepted under the project's license. See [LICENSE](LICENSE).
