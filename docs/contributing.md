# Contributing

Before implementing any changes, identify which agents match the files you plan to modify. This ensures your PR triggers the correct automated review workflows.

| Path | Agent | Review Focus |
|------|-------|-------------|
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |

## Automated Review Workflows

Four path-scoped review workflows enforce this matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `sidecar/web/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so the checks can be required in branch protection without blocking unrelated work.

## Contribution Process

1. **Read agent files:** Review the relevant agent files in `.github/agents/` before making changes.
2. **Document agent consultation:** In your PR description, note which agents you consulted, e.g., `Consulted: race-engineer per routing matrix`.
3. **Follow review workflow:** Your PR will trigger the appropriate automated review workflows. Address any feedback from agent reviews.
4. **Add or update tests:** If you change source files under `sidecar/src/` or `sidecar/web/`, ensure you add or update corresponding test files. Otherwise, the QA review workflow may request changes.
5. **Submit your PR:** Follow the [CONTRIBUTING.md](../CONTRIBUTING.md) for procedural details.

## Additional Guidelines
- Use [Conventional Commits](https://www.conventionalcommits.org/) for PR titles and commit messages.
- Document any architectural decisions in `docs/adr/`.
- Update documentation and tests as needed.
