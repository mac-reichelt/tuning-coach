# Contributing

Welcome! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please read this guide before submitting a pull request.

## Getting Started

- **Clone the repo:**
  ```bash
  git clone <repo-url>
  cd <repo-name>
  ```
- **Install dependencies:** Follow [docs/getting-started.md](getting-started.md) for setup instructions.

## Branching & PRs

- **Branch from main:**
  ```bash
  git checkout -b <feature-branch>
  ```
- **Commit messages:** Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
- **Pull requests:** Fill out the PR template. Include which agent files you consulted if your changes match the routing matrix below.

## Agent Review Workflows

This project enforces four automated agent-driven review workflows for every PR:

| Workflow | Scope | Agent | Verdicts |
|---|---|---|---|
| `security-review` | Workflow/action files, shell scripts, or files with security-sensitive names (auth, crypto, secret, oidc, token) | `security-review` | APPROVE, REQUEST_CHANGES, COMMENT |
| `qa-review` | Source files under `sidecar/src/` or `overlay/` without accompanying test changes | `qa-engineer` | APPROVE, REQUEST_CHANGES, COMMENT |
| `telemetry-review` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `.github/agents/telemetry-expert.agent.md` | `telemetry-expert` | APPROVE, REQUEST_CHANGES, COMMENT |
| `heuristics-review` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `.github/agents/race-engineer.agent.md` | `race-engineer` | APPROVE, REQUEST_CHANGES, COMMENT |

All four workflows use a skip-success pattern: if your PR is out of scope for a workflow, it posts a passing check so branch protection is not blocked.

### Agent Routing Matrix

Before making changes, consult the agent(s) listed for the files you plan to edit:

| Path glob | Agent(s) to consult | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` | Overlay test discipline (vitest) |

**Note:** Mention consulted agents in your PR description as:
`Consulted: <agent-name> per routing matrix`.

## Code Style & Tests

- **Rust:** Follow [rustfmt](https://github.com/rust-lang/rustfmt) and [clippy](https://github.com/rust-lang/rust-clippy).
- **JS/TS:** Use [Prettier](https://prettier.io/) and [ESLint](https://eslint.org/).
- **Tests:** Add or update tests for all new public functions, modules, or features. PRs that change source without tests will trigger a QA review.

## Docs

- Update [README.md](../README.md) and [docs/](./) as needed.
- For architecture decisions, add or update files in [docs/adr/](adr/).

## Security

- Never commit secrets or credentials.
- Review [SECURITY.md](../SECURITY.md) for responsible disclosure.

## Release Process

- Releases are managed by [release-please](https://github.com/google-github-actions/release-please-action).
- See [CHANGELOG.md](../CHANGELOG.md) for release notes.

## Questions?

- Open an issue or ask in discussions.
