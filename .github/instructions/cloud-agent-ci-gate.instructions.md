---
applyTo: '**'
description: 'Playbook for unblocking cloud Copilot agent PRs whose workflows are stuck in action_required state. Apply when a copilot-swe-agent or release-please PR has empty statusCheckRollup or BLOCKED merge state with no failing checks.'
---

# Cloud Agent CI Gate (PR Checks `action_required`)

When the cloud `copilot-swe-agent` pushes to a PR branch in this repo, the
workflows can land in status `action_required` instead of running. Auto-merge
gate stalls until a human (or this script) approves.

Root cause: GitHub treats first-party-pushes-from-bot-actor like fork PRs and
requires explicit approval before workflows run. The repo setting
`fork-pr-contributor-approval = first_time_contributors_new_to_github`
(see `llm-workflow-hardening.instructions.md` Pattern 9) eliminates this for
**known** bot accounts, but new bot accounts and release-please runs can still
trigger it.

## Detect

```bash
BR=copilot/feat-whatever
gh run list -b "$BR" --json status,conclusion,databaseId \
  --jq '[.[] | select(.status=="completed" and .conclusion=="action_required")][0].databaseId'
```

## Approve + rerun

```bash
RUN_ID=$(gh run list -b "$BR" --json status,conclusion,databaseId \
  --jq '[.[] | select(.status=="completed" and .conclusion=="action_required")][0].databaseId')
gh run rerun "$RUN_ID"
```

`gh run rerun` on an `action_required` run implicitly approves it. After this,
PR Checks runs normally.

## Drain script (for active PR sweeps)

```bash
for pr in $(gh pr list --state open --json number,headRefName --jq '.[].number'); do
  br=$(gh pr view "$pr" --json headRefName --jq .headRefName)
  rid=$(gh run list -b "$br" --json status,conclusion,databaseId \
    --jq '[.[] | select(.status=="completed" and .conclusion=="action_required")][0].databaseId')
  [ -n "$rid" ] && { echo "PR #$pr → rerunning $rid"; gh run rerun "$rid"; }
done
```

## When `gh run rerun` is not enough

Some PRs (especially first-time-contributor + release-please) start with NO
workflows triggered at all — `statusCheckRollup: []`. Symptoms:

- `gh pr view N --json statusCheckRollup` → `checks: []`
- `gh run list --branch <copilot-branch>` → empty or all `action_required`
- `gh api .../actions/runs/$id/approve` → 403 "not from a fork pull request"

**Fix:** close + reopen the PR as a maintainer.

```bash
gh pr close N && sleep 2 && gh pr reopen N
# CI fires within seconds, runs normally, no manual approve needed
```

This is the canonical workaround for release-please's "chore: release main"
PRs as well — those are opened by `github-actions[bot]` which suppresses
workflow triggers by design (anti-recursion).

## Other gotchas

### `gh run approve` does not exist

Use the API (and only on actual fork PRs):

```bash
gh api -X POST repos/$REPO/actions/runs/$RUN_ID/approve   # only fork PRs
gh api -X POST repos/$REPO/actions/runs/$RUN_ID/rerun     # works for action_required from cloud-swe-agent
```

The cloud-agent's `action_required` runs are NOT fork PRs (they're same-repo
branches), so `/approve` returns HTTP 403 "This run is not from a fork pull
request". Use `/rerun` instead.

### Squash-merge sometimes drops `Closes #N`

When the agent's PR body uses bullet-list form (`- Closes #N`) instead of a
leading sentence, the linked issue may not auto-close. Manually
`gh issue close N` after merge if needed.

### `gh pr merge --auto` quirk

Returns silently on success but `gh pr view N --json autoMergeRequest` may
show null briefly. If state stays NULL after CI green, re-run the command.

### Bot assignment via REST is flaky

`gh issue edit N --add-assignee Copilot` intermittently 404s. Prefer GraphQL:

```bash
gh api graphql -f query='mutation($i:ID!,$a:[ID!]!){replaceActorsForAssignable(input:{assignableId:$i,actorIds:$a}){clientMutationId}}' \
  -f i="$ISSUE_NODE_ID" -f a='["BOT_kgDOC9w8XQ","MDQ6VXNlcjg0NDA3NDEz"]'
```
(Copilot bot + maintainer as co-assignees.)

### `GITHUB_TOKEN` push doesn't trigger downstream workflows

Any event created by `GITHUB_TOKEN` does not trigger another workflow
(anti-recursion). For downstream triggering use `secrets.AUTOMATION_PAT`. See
`llm-workflow-hardening.instructions.md` Pattern 7.

### `gh pr update-branch` from a maintainer bypasses `action_required`

When a sibling PR merges and your branch goes BEHIND, run
`gh pr update-branch <N> --rebase` — the resulting push is from the
maintainer's identity, so CI runs immediately without needing close+reopen.
This is what `update-pr-branches.yml` automates.

### Job-name collisions across workflow files create silent merge blocks

GitHub treats same-named jobs across different workflows as the SAME check
context for branch protection purposes. Always give workflow jobs unique
`name:` values, especially when adding new workflows to a repo with existing
required checks.

### GitHub Models API needs `models: read` permission

Workflows that call `https://models.inference.ai.azure.com/...` (or the new
`gh models` API) get `curl: (22) error: 401` without it. Add to the workflow's
`permissions:` block.
