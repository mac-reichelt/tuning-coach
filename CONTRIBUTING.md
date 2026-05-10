# Contributing

Thank you for your interest in contributing! This project uses agent-driven review workflows to ensure code quality, security, and correctness. Please follow these steps when submitting a pull request:

## 1. Identify Agent Routing

Before making changes, consult the agent routing matrix (see [docs/contributing.md](docs/contributing.md)) to determine which agent files you need to read based on the files you plan to edit. Note the consulted agents in your PR description as:

```
Consulted: <agent-name> per routing matrix
```

## 2. Review Workflows

Your PR will be checked by several automated agent review workflows:

- **security-review**: Runs when workflow/action files, shell scripts, or security-sensitive files change.
- **qa-review**: Runs when source files under `sidecar/src/` or `overlay/` change without accompanying test file changes.
- **telemetry-review**: Runs when `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` change.
- **heuristics-review**: Runs when `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` change.

Out-of-scope PRs post a passing check so these can be required without blocking unrelated work.

## 3. Procedural Steps

- **Fork and clone** the repository.
- **Create a branch** for your changes.
- **Make your changes**, consulting agent files as required.
- **Add or update tests** for any new public functions or modules.
- **Document** any new features or API changes in `/docs` and `README.md`.
- **Open a pull request** and fill out the PR template, listing consulted agents.

## 4. Agent Review Outcomes

Agent reviews will post a verdict (`APPROVE`, `REQUEST_CHANGES`, or `COMMENT`) and findings as a PR review and check-run. Address any `REQUEST_CHANGES` before merging.

## 5. Additional Guidelines

- Use active voice and present tense in documentation.
- Link all new concepts to their definitions.
- Do not document features that do not exist yet.

For more details, see [docs/contributing.md](docs/contributing.md).
