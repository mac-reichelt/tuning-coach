# Contributing Guide

Welcome! This project uses agent-driven review workflows to ensure quality and correctness. Please read this guide before submitting changes.

## Agent Routing Matrix

Before you start, identify which agents match the files you plan to touch. Consult the relevant agent(s) and note them in your PR description.

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

## Automated Review Workflows

The following workflows enforce agent review checks:

- `.github/workflows/security-review.yml` — Security-sensitive changes
- `.github/workflows/qa-review.yml` — Source changes without test updates
- `.github/workflows/telemetry-review.yml` — Telemetry schema/logic changes
- `.github/workflows/heuristics-review.yml` — Heuristics/tuning logic changes
- `.github/workflows/devops-review.yml` — CI/CD and workflow changes
- `.github/workflows/agent-review.yml` — General agent review

Each workflow posts a verdict and must pass for your PR to merge.

## How to Contribute

1. Fork and clone the repository
2. Create a new branch
3. Make your changes
4. Update documentation as needed
5. Push your branch
6. Open a Pull Request
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

SPDX identifier: [LICENSE](../LICENSE)
