# Contributing

Thank you for your interest in contributing! This project uses a multi-agent review system to ensure code quality, correctness, and security. Please read this guide before opening a pull request.

## Getting Started

1. **Fork the repository**
2. **Clone your fork**
   ```bash
   git clone <your-fork-url>
   cd <repo>
   ```
3. **Create a new branch**
   ```bash
   git checkout -b <feature-or-fix-name>
   ```
4. **Make your changes**
5. **Commit and push**
   ```bash
   git add .
   git commit -m "<type>: <description>"
   git push origin <branch>
   ```
6. **Open a pull request**

## Agent Review System

This project uses automated agent reviews for key areas:

- **Devops/CI**: `.github/workflows/devops-review.yml` (devops-engineer agent)
- **Security**: `.github/workflows/security-review.yml` (security-review agent)
- **Telemetry**: `.github/workflows/telemetry-review.yml` (telemetry-expert agent)
- **Heuristics/Tuning Logic**: `.github/workflows/heuristics-review.yml` (race-engineer agent)
- **QA/Test Coverage**: `.github/workflows/qa-review.yml` (qa-engineer agent)

Each agent review is triggered based on the files you change. See the [Agent Routing Matrix](docs/contributing.md#agent-routing-matrix) for details.

### Agent Routing Matrix

Before implementing changes, identify which agents match the files you plan to touch. Consult the relevant agent files before writing code, and note them in your PR description as:

```
Consulted: <agent-name> per routing matrix
```

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

### Automated Routing

Four path-scoped review workflows enforce this matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## Commit Message Convention

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation
- `chore:` for maintenance
- `refactor:` for code changes that neither fix a bug nor add a feature

## Code Style

- Follow the existing code style and formatting.
- Run tests before pushing.
- Add tests for new public functions or modules.

## Documentation

- Update relevant documentation in `docs/` and `README.md`.
- Cross-link new concepts.

## Opening a PR

- Fill out the PR template.
- Note which agent(s) you consulted per the routing matrix.
- Ensure all required checks pass.

## License

By contributing, you agree your code will be released under the project's license.
