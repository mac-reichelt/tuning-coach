# Contributing

Thank you for your interest in contributing! This project welcomes pull requests and issues from the community.

## Getting Started

1. **Fork the repository** and clone it locally:
   ```bash
   git clone https://github.com/<your-username>/tuning-coach.git
   cd tuning-coach
   ```
2. **Install dependencies** as described in [docs/getting-started.md](docs/getting-started.md).
3. **Create a new branch** for your change:
   ```bash
   git checkout -b my-feature
   ```

## Agent Routing Matrix & Review Workflows

Before making changes, **identify which agent(s) you must consult** based on the files you plan to edit. This is enforced by automated review workflows. See the table below:

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

**You must read the relevant agent file(s) before writing code, and note them in your PR description as:**

```
Consulted: <agent-name> per routing matrix
```

### Automated Review Workflows

Four path-scoped review workflows enforce this matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## Making Changes

- **Write code and tests**. Follow the style and conventions in the codebase.
- **Update documentation** if your change affects the user interface, API, or behavior.
- **Run tests** before submitting:
  ```bash
  cargo test
  # or for overlay
  npm test
  ```

## Submitting a Pull Request

1. **Push your branch** to your fork:
   ```bash
   git push origin my-feature
   ```
2. **Open a pull request** against the main repository.
3. **Fill out the PR template**. Include which agent(s) you consulted per the routing matrix.
4. **Address automated review feedback** from the required workflows.

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/). Be respectful and constructive.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
