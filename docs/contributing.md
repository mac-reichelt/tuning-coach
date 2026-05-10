# Contributing Guide

This project uses agent-driven review workflows to ensure code quality and domain correctness. Please follow these guidelines when contributing:

## Agent Routing and Review

Before implementing changes, identify which agent(s) match the files you plan to touch using the routing matrix below. Consult the relevant agent files before writing code, and note them in your PR description.

### Routing Matrix

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

### Automated Review Checks

Four path-scoped review workflows enforce this matrix on every PR:

- `security-review.yml` — Security-sensitive files, workflows, or auth logic
- `qa-review.yml` — Source changes without accompanying test changes
- `telemetry-review.yml` — Telemetry schema or expert agent changes
- `heuristics-review.yml` — Heuristics/recommendations logic or race-engineer agent changes

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## Steps to Contribute

1. **Fork and clone the repository**
2. **Create a new branch**
3. **Make your changes**
4. **Consult agent files as per the routing matrix**
5. **Open a pull request**
   - Note consulted agents in your PR description: `Consulted: <agent-name> per routing matrix.`
6. **Wait for agent review checks to complete**

## Running Tests

- **Rust:**
  ```bash
  cargo test
  ```
- **Overlay (JS):**
  ```bash
  npm test
  ```

## License
MIT — see [LICENSE](../LICENSE).
