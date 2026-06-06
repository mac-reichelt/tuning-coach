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
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | Security-sensitive changes |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |

## Process

1. **Consult the agent(s)** for the files you plan to change. Read their agent files.
2. **Note in your PR description**: `Consulted: <agent-name> per routing matrix`.
3. **Follow the agent's conventions** for code, tests, and docs.
4. **Open your PR**. CI will route it to the correct agent(s) for review.

See [CONTRIBUTING.md](CONTRIBUTING.md) for full onboarding.