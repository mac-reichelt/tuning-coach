# Contributing

Thank you for your interest in contributing! This project uses a multi-agent review process to ensure code quality, correctness, and security. Please read this guide carefully before opening a pull request.

## Getting Started

1. **Fork the repository** and clone your fork.
2. **Create a new branch** for your change:
   ```bash
   git checkout -b my-feature
   ```
3. **Make your changes** and commit them with [conventional commit messages](https://www.conventionalcommits.org/en/v1.0.0/).
4. **Push your branch** to your fork:
   ```bash
   git push origin my-feature
   ```
5. **Open a pull request** against the main repository.

## Agent Routing Matrix

Before implementing any changes, you must identify which agents are responsible for reviewing the files you plan to touch. Use the routing matrix below to determine which agent(s) to consult. Read the relevant agent files in `.github/agents/` before writing code, and note them in your PR description as:

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

## Code Style & Documentation

- **Active voice, present tense.**
- **Second person for instructions.**
- **Code-first.** Show the command/snippet; explain in 1–2 lines after.
- **Scannable.** Headings, bullets, tables.
- **Link everything.**
- **No marketing fluff.**
- **Versioned.**

See [docs/contributing.md](docs/contributing.md) for more details.

## Pull Request Checklist

- [ ] I have identified and consulted the correct agent(s) per the routing matrix.
- [ ] I have included a note in my PR description: `Consulted: <agent-name> per routing matrix`.
- [ ] I have run all tests and linters locally.
- [ ] I have updated documentation as needed.

## Questions?

Open an issue or ask in Discussions.
