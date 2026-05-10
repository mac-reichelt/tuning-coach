# Contributing Guide

This project uses agent-driven review workflows to ensure quality and correctness. Please follow these steps when contributing:

## Steps

1. **Fork and clone the repository**
2. **Create a feature branch**
3. **Make your changes**
4. **Write conventional commit messages**
5. **Push and open a pull request**

## Agent-Driven Review Workflows

Automated agent reviews cover key areas:

- **Security**: Checks for workflow, shell, and security-sensitive changes
- **DevOps**: Checks for CI/CD and workflow changes
- **QA**: Checks for source changes without corresponding tests
- **Telemetry**: Checks for telemetry schema changes
- **Heuristics**: Checks for tuning logic changes

All workflows use skip-success: out-of-scope PRs post `success` so checks can be required without blocking unrelated work.

### Agent Routing Matrix

Consult the relevant agent(s) before editing files:

| Path glob | Agent(s) to consult | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema must stay in sync |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security |
| Any file with auth, secrets, OIDC, crypto | `security-review` | Security review |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` | Overlay test discipline |

See `.github/copilot-instructions.md` for full details.

## Checklist

- [ ] Conventional commit message
- [ ] Consulted relevant agent(s)
- [ ] Added/updated tests
- [ ] Updated docs if needed

## License
SPDX identifier — see [LICENSE](../LICENSE).
