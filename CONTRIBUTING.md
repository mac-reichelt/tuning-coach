# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please read this guide before opening a pull request.

## Getting Started

1. **Fork the repository**
2. **Clone your fork**
   ```bash
   git clone https://github.com/<your-username>/<repo-name>.git
   cd <repo-name>
   ```
3. **Create a new branch**
   ```bash
   git checkout -b <feature-or-bugfix-name>
   ```
4. **Make your changes**
5. **Commit with a conventional commit message**
   ```bash
   git commit -m "feat: add new telemetry parser"
   ```
6. **Push and open a pull request**

## Agent-Driven Review Workflows

This project uses automated agent reviews for key areas:

- **Security**: `.github/workflows/security-review.yml` — triggers on changes to workflow/action files, shell scripts, or security-sensitive files.
- **DevOps**: `.github/workflows/devops-review.yml` — triggers on CI/CD and workflow changes.
- **QA**: `.github/workflows/qa-review.yml` — triggers when source files change without corresponding test changes.
- **Telemetry**: `.github/workflows/telemetry-review.yml` — triggers on telemetry schema or expert agent file changes.
- **Heuristics**: `.github/workflows/heuristics-review.yml` — triggers on tuning logic or race-engineer agent file changes.

All workflows use the skip-success pattern: out-of-scope PRs post `success` so checks can be required without blocking unrelated work.

### Agent Routing Matrix

Before implementing changes, consult the relevant agent(s) based on the files you plan to touch:

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

See `.github/copilot-instructions.md` for full details.

## PR Checklist

- [ ] Conventional commit message
- [ ] Consulted relevant agent(s) per routing matrix
- [ ] Added/updated tests for new public functions
- [ ] Updated documentation if needed

## Documentation

- [Getting Started](docs/getting-started.md)
- [API Reference](docs/reference/api.md)
- [Contributing](docs/contributing.md)

## License

SPDX identifier — see [LICENSE](LICENSE).
