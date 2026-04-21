# Contributing

For full contributor workflow and policy, see the root
[CONTRIBUTING.md](../CONTRIBUTING.md).

## Agent review merge gate

This repository uses `.github/workflows/agent-review.yml` to publish a required
status check named `agent-review` on pull requests.

- Trigger: `pull_request` (`opened`, `synchronize`, `reopened`,
  `ready_for_review`)
- Inputs to review model: PR number, base/head SHA, changed files, full diff,
  and `.github/agents/code-review.agent.md`
- Verdict mapping:
  - `APPROVE` → `success`
  - `REQUEST_CHANGES` → `failure`
  - `COMMENT` → `neutral`
- Visibility: findings are also posted as a PR review comment
- Skip paths: draft PRs, Dependabot PRs, and docs-only (`docs/**`, `*.md`,
  `.github/dependabot.yml`) changes

If branch protection requires `agent-review`, the PR cannot merge until that
check is present and non-failing.
