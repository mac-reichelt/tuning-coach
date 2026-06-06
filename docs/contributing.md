# Contributing to Tuning Coach

This project uses agent-driven review and strict CI. Before implementing any changes, identify which agents match the files you plan to touch.

## Agent Matrix

| Path | Agent(s) | Purpose |
|------|----------|---------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |
| `simhub/**` | `qa-engineer` | Dashboard bundle correctness |

## Workflow

1. **Consult the agent(s)** for your target files. Read their agent files before writing code.
2. **Follow Conventional Commits** for PR titles and commit messages.
3. **Run tests:**
   - Rust: `cargo test --workspace`
   - Web frontend: `cd sidecar/web && npm install && npm test`
4. **Open a PR:**
   - Note consulted agents in your PR description: `Consulted: <agent-name> per routing matrix`
   - CI will enforce agent review and test coverage.

## Directory Structure

- `sidecar/` — Rust sidecar (server, telemetry, API)
- `sidecar/web/` — Web frontend (served by sidecar, dev/test tooling)
- `simhub/` — SimHub dashboard bundle (`*.djson`, PNGs)
- `docs/` — Documentation

## Release & Versioning

- The web frontend is versioned and released with the sidecar.
- The SimHub dashboard bundle is shipped as a release asset attached to the sidecar release.

## Additional Resources
- [README.md](../README.md)
- [Architecture Decision Records](adr/README.md)
