# Contributing Guide

This guide covers how to contribute to Tuning Coach, including agent routing and automated review workflows.

## Agent Routing Matrix

Before making changes, consult the [agent routing matrix](../.github/copilot-instructions.md#agent-routing) to determine which agent(s) must review your changes. The matrix maps file paths to responsible agents:

| Path glob | Agent(s) to consult | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security |
| Any file with auth, secrets, OIDC, crypto | `security-review` | Security review required |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `overlay/**` (logic changes) | `qa-engineer` | Overlay test discipline |

## Automated Review Workflows

Four automated review workflows enforce the agent routing matrix:

| Workflow | Scope |
|---|---|
| `security-review` | Security-sensitive files and workflows |
| `qa-review` | Source changes without accompanying tests |
| `telemetry-review` | Telemetry schema and expert logic |
| `heuristics-review` | Tuning logic and race engineering heuristics |

These checks run on every PR and must pass for merge. Out-of-scope PRs are auto-approved.

## PR Process

1. **Consult agent routing matrix** before editing.
2. **Note consulted agents** in your PR description.
3. **Open a PR** and fill out the template.
4. **Wait for agent review checks** to complete.

## Testing

- Add tests for new public functions/modules.
- Run all tests before opening a PR:
  ```bash
  cargo test
  npm test
  ```

## Docs Style

- Use active voice, second person, and scannable formatting.
- Link all new concepts to their definitions.

## License
MIT — see [LICENSE](../LICENSE).
