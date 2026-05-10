# Contributing

Thank you for your interest in contributing! This project uses agent-based review workflows to ensure code quality and domain correctness. Please follow these steps when submitting a pull request:

## 1. Fork and Clone

```bash
git clone <repo-url>
cd <repo>
```

## 2. Branch

Create a feature branch:

```bash
git checkout -b <feature-name>
```

## 3. Identify Agent Routing

Before making changes, consult the agent routing matrix below to determine which agent files you need to read and reference. Note the consulted agents in your PR description as:

`Consulted: <agent-name> per routing matrix.`

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

## 4. Review Workflows

Automated review workflows enforce agent routing:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `overlay/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

Out-of-scope PRs post `success` so checks can be required without blocking unrelated work.

## 5. Commit and Push

Follow conventional commit messages:

```bash
git commit -m "feat: <description>"
git push origin <feature-name>
```

## 6. Open a Pull Request

- Fill out the PR template.
- List consulted agents per routing matrix.
- Ensure all required checks pass.

## 7. Respond to Review

- Address agent and human reviewer feedback.
- Update docs as needed.

## 8. Merge

Once all checks pass and reviews are approved, your PR can be merged.

## Additional Resources

- [docs/contributing.md](docs/contributing.md) — extended contributor guide
- [docs/adr/README.md](docs/adr/README.md) — architecture decision records

---

For more details on agent conventions and review process, see `.github/copilot-instructions.md`.
