# Contributing to Tuning Coach

This project uses agent-driven review and strict CI. Before implementing any changes, identify which agents match the files you plan to touch.

## Agent Matrix

| Path | Agent(s) | Purpose |
|------|----------|---------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |
| `simhub/**` | `qa-engineer` | SimHub dashboard bundle correctness |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |

## Process

1. **Consult the agent(s)** for the files you plan to change. Read their agent files before writing code.
2. **Note consulted agents** in your PR description: `Consulted: <agent-name> per routing matrix`.
3. **Follow the agent's conventions** for code, tests, and docs.
4. **Run all tests** and ensure CI passes before submitting a PR.

## Coding Standards
- Use Conventional Commits for PR titles and commit messages.
- Add or update tests for every new public function or module.
- Document new features in the README and /docs site.

## Docs
- [README.md](../README.md) — project overview and quickstart
- [docs/adr/](adr/README.md) — architecture decisions
- [docs/getting-started.md](getting-started.md) — install and first run
- [docs/reference/api.md](reference/api.md) — API reference

## License
MIT — see [LICENSE](../LICENSE).
