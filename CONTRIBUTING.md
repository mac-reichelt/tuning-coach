# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps to get started:

## Getting Started

1. **Fork the repository**
2. **Clone your fork**

   ```bash
   git clone https://github.com/<your-username>/<repo-name>.git
   cd <repo-name>
   ```
3. **Create a new branch**

   ```bash
   git checkout -b <feature-or-fix-name>
   ```

## Agent Routing Matrix

Before making changes, consult the relevant agent(s) based on the files you plan to edit. The agent routing matrix ensures that subject-matter experts review changes in their domain.

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

**Note:** Mention consulted agents in your PR description as:

```
Consulted: <agent-name> per routing matrix
```

## Automated Review Workflows

The following GitHub Actions enforce agent review checks:

- `.github/workflows/security-review.yml` — Security-sensitive changes
- `.github/workflows/qa-review.yml` — Source changes without test updates
- `.github/workflows/telemetry-review.yml` — Telemetry schema/logic changes
- `.github/workflows/heuristics-review.yml` — Heuristics/tuning logic changes
- `.github/workflows/devops-review.yml` — CI/CD and workflow changes
- `.github/workflows/agent-review.yml` — General agent review

Each workflow posts a verdict (`APPROVE`, `REQUEST_CHANGES`, or `COMMENT`) and must pass for your PR to merge.

## Making a Change

1. **Write code and tests**
2. **Update documentation** if needed
3. **Push your branch**

   ```bash
   git push origin <feature-or-fix-name>
   ```
4. **Open a Pull Request**
   - Fill out the PR template
   - Note which agents you consulted
   - Ensure all required checks pass

## Style Guide

- Use active voice and present tense
- Write instructions in second person
- Show real output and code snippets
- Use headings, bullets, and tables for clarity
- Link to definitions and reference docs

## License

SPDX identifier: [LICENSE](LICENSE)
