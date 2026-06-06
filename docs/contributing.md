# Contributing to Tuning Coach

This project uses agent-driven review and strict CI. Before implementing any changes, identify which agents match the files you plan to touch.

## Agent Matrix

| Path | Agent(s) | Purpose |
|------|----------|---------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `sidecar/web/**` | `qa-engineer` | Web frontend test discipline (vitest) |
| `simhub/**` | `qa-engineer` | SimHub dashboard bundle correctness |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |

**Note:**
- The web frontend is now located in `sidecar/web/` and is served directly by the sidecar binary. All UI changes should be made here.
- The SimHub dashboard bundle is located in `simhub/` and contains only the `.djson` import files and preview images.
- The overlay directory no longer exists; all references should be updated to the new structure.

## Workflow

1. **Consult the agent(s)** for the files you plan to change. Read their agent files before writing code.
2. **Note in your PR description** which agents you consulted: `Consulted: <agent-name> per routing matrix`.
3. **Follow conventional commits** for PR titles and commit messages.
4. **Run tests** for any new or changed public functions, especially in `sidecar/web/` (vitest).
5. **Check CI** — all agent-driven workflows must pass before merge.

## Additional Resources
- [README.md](../README.md)
- [Architecture Decision Records](adr/README.md)
- [API Reference](reference/api.md)
