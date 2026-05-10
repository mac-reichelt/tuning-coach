# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality and domain correctness. Please follow these steps when submitting a pull request:

## Workflow Overview

- **Agent Routing:** Before making changes, identify which agent(s) match the files you plan to touch. Consult the agent files as described below.
- **Automated Review Checks:** Four path-scoped review workflows enforce agent review on every PR:
  - `security-review.yml` — Security-sensitive files, workflows, or auth logic
  - `qa-review.yml` — Source changes without accompanying test changes
  - `telemetry-review.yml` — Telemetry schema or expert agent changes
  - `heuristics-review.yml` — Heuristics/recommendations logic or race-engineer agent changes

All four checks use the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## Agent Routing Matrix

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

## How to Contribute

1. **Fork the repository**
2. **Clone your fork**
3. **Create a new branch**
4. **Make your changes**
5. **Consult agent files as per the routing matrix**
6. **Open a pull request**
   - Note consulted agents in your PR description: `Consulted: <agent-name> per routing matrix.`
7. **Wait for agent review checks to complete**

## Running Tests

- **Rust:**
  ```bash
  cargo test
  ```
- **Overlay (JS):**
  ```bash
  npm test
  ```

## Documentation

See [docs/contributing.md](docs/contributing.md) for more details.

## License

SPDX: MIT — see [LICENSE](LICENSE).
