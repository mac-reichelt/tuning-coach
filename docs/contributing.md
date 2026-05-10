# Contributor Guide

Welcome! This project uses a multi-agent review process and automated checks to ensure high-quality, maintainable code. This guide explains how to contribute effectively.

## Agent Routing Matrix

Before you start coding, determine which agent(s) are responsible for reviewing your changes. The agent routing matrix below shows which files are reviewed by which agents. **You must consult the relevant agent file(s) in `.github/agents/` before making changes.**

| Path glob | Agent(s) to consult before editing | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `overlay/**` (logic changes, not pure CSS) | `qa-engineer` | Overlay test discipline (vitest) |

**In your PR description, include:**

```
Consulted: <agent-name> per routing matrix
```

## Automated Review Workflows

Every pull request is checked by four automated workflows that enforce the agent routing matrix:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

If your PR is out of scope for a workflow, it will post a passing check (skip-success) and not block your merge.

## How to Contribute

1. **Fork and clone** the repository.
2. **Create a feature branch**:
   ```bash
   git checkout -b my-feature
   ```
3. **Identify agent(s)** for your changes using the routing matrix above.
4. **Consult the agent file(s)** in `.github/agents/` for conventions and requirements.
5. **Make your changes** and commit with a [conventional commit message](https://www.conventionalcommits.org/en/v1.0.0/).
6. **Push your branch**:
   ```bash
   git push origin my-feature
   ```
7. **Open a pull request**. In the PR description, note which agent(s) you consulted.
8. **Ensure all required checks pass** before merging.

## Code Style & Documentation

- Use **active voice** and **present tense**.
- Write instructions in **second person**.
- Prefer **code-first** explanations.
- Use **headings, bullets, and tables** for clarity.
- **Link** to definitions and related docs.
- Avoid marketing language.
- Indicate minimum compatible version where relevant.

See [README.md](../README.md) and [docs/](./) for more style guidance.

## Questions?

Open an issue or start a discussion if you need help.
