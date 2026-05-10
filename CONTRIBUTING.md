# Contributing

Thank you for your interest in improving this project!

## How to Contribute

- **Fork** the repository and create a feature branch.
- **Write code** and tests. Follow the [coding conventions](docs/contributing.md).
- **Document** any new features or changes. Update relevant files in `docs/` and `README.md`.
- **Open a Pull Request**. Fill out the PR template completely.

## Agent Review Workflows

This project uses agent-driven review checks for critical areas. When you change files matching certain paths, your PR will trigger specialized review workflows:

| Workflow | Scope | Agent File | Purpose |
|----------|-------|------------|---------|
| `security-review` | Security-sensitive files, workflows, actions | `.github/agents/security-review.agent.md` | Security correctness |
| `devops-review` | CI/CD workflows, actions | `.github/agents/devops-engineer.agent.md` | DevOps correctness |
| `telemetry-review` | Telemetry schema, packet parsing | `.github/agents/telemetry-expert.agent.md` | Telemetry schema correctness |
| `heuristics-review` | Tuning logic, heuristics, recommendations | `.github/agents/race-engineer.agent.md` | Engineering correctness |
| `qa-review` | Source changes without test changes | `.github/agents/qa-engineer.agent.md` | Test coverage discipline |

**Before editing files in these areas:**
- Consult the relevant agent file (see the routing matrix in [copilot-instructions.md](.github/copilot-instructions.md)).
- Note the consulted agent(s) in your PR description: `Consulted: <agent-name> per routing matrix`.

## Procedural Requirements

- **Tests:** All new public functions, modules, or features must have accompanying tests. If you change source files under `sidecar/src/` or `overlay/` without updating or adding tests, the `qa-review` check will flag your PR.
- **Architecture Decisions:** Schema changes or new ADRs require review by the `architect` agent.
- **Security:** Any file involving auth, secrets, OIDC, or crypto triggers the `security-review` check.

## Review Process

- Automated agent reviews run on every PR. Out-of-scope PRs are auto-approved by the agents.
- Address any agent feedback before requesting human review.

## Docs

- Update `docs/contributing.md` for any changes to the contribution process.
- See [copilot-instructions.md](.github/copilot-instructions.md) for agent routing details.

## License

Contributions are accepted under the project's license. See [LICENSE](LICENSE).
