---
applyTo: '.github/workflows/**'
description: 'Security and reliability patterns for LLM-driven GitHub Actions workflows. Apply when writing or editing any workflow that consumes model output, posts PR/issue comments, or runs against agent-authored content.'
---


# Hardening LLM-driven GitHub Actions workflows

Lessons from `tuning-coach` agent automation pipeline.
See [[tuning-coach]] and [[llm-workflow-payload-limits]].

## Threat model

Any workflow that takes an LLM response and uses it in shell, file paths, or
PR/issue API calls must assume the LLM output is **attacker-controlled bytes**
(prompt injection from PR diffs, comments, or external content). Patterns
that look benign explode when the LLM emits backticks, `$()`, `..`, etc.

## Pattern 1: shell template injection via outputs

**BAD:**
```yaml
- name: Publish review
  run: |
    summary="${{ steps.review.outputs.summary }}"
    body=$(cat <<'EOF'
    🤖 verdict: __SUMMARY__
    EOF
    )
    body="${body/__SUMMARY__/${summary}}"
    gh api ... -f body="$body"
```
LLM emits `` `StorageError::Schema` `` → bash command-substitutes it →
`/tmp/...sh: line 22: StorageError::Schema: command not found`. Step fails,
required check blocks PR. Real incident on PR #84.

**GOOD:**
```yaml
- name: Publish review
  env:
    REVIEW_SUMMARY: ${{ steps.review.outputs.summary }}
  run: |
    {
      echo "verdict: $verdict"
      echo
      echo "$REVIEW_SUMMARY"
    } > /tmp/body.md
    jq -n --rawfile body /tmp/body.md '{event:"COMMENT", body:$body}' \
      | gh api .../pulls/$PR/reviews --input -
```
- Move LLM-controlled values to `env:` block (no `${{ }}` in script body).
- Reference as `"$VAR"` (quoted) — bash does NOT command-substitute inside
  double quotes from env vars, only inside `${{ }}` interpolated literals.
- Build body via `printf` / `echo` to a file, send via `jq --rawfile` so JSON
  encoding handles all special bytes.

## Pattern 2: path traversal in patch appliers

**BAD:** Allowlist via regex prefix
```bash
[[ "$path" =~ ^(docs/|README\.md$|...) ]]   # allows docs/../.git/config
mkdir -p "$(dirname "$path")"
echo "$content" > "$path"
git add "$path"
```
LLM emits `docs/../.git/hooks/pre-commit` → file written → next `git commit`
executes it with PAT exposed via `.git/config` extraheader.

**GOOD:** Realpath validation in Python
```python
ALLOWED = ["docs", "README.md", ...]
real_root = Path.cwd().resolve()
target = (real_root / path).resolve()
target.relative_to(real_root)            # raises if outside
assert ".." not in Path(path).parts
assert not Path(path).is_absolute()
assert any(target.relative_to(real_root).parts[0] == a or
           str(target.relative_to(real_root)) == a for a in ALLOWED)
```
See `apply-tech-writer-patches.py` in tuning-coach.

## Pattern 3: don't use forgeable loop guards

Bot-author email check (`if last commit author == bot-email skip`) is bypassed
by `git commit --author='X <bot@email>'`. Better:
- **Convergence**: Let the LLM see its own output via the next `synchronize`
  event. With low temperature it returns APPROVE on the second pass. No skip.
- Or post `neutral` check status on skip so PR still merges.

## Pattern 4: dependabot-skip on required checks = permanent block

**BAD:** `if: github.event.pull_request.user.login != 'dependabot[bot]'`
on a workflow whose check name is in branch-protection required list.

GitHub branch protection treats "skipped because `if:`" as "never reported"
→ check is forever pending → `BLOCKED` mergeStateStatus.

**GOOD:** Either
- Don't skip dependabot, let the workflow run (LLM will likely APPROVE).
- Or split into two jobs sharing the check name: real job for non-dependabot,
  skip-success job for dependabot that just publishes the check as `success`.
- Or simpler: omit the workflow from the required list (`gh api repos/$O/$R/branches/main/protection/required_status_checks/contexts -X DELETE -f 'contexts[]=NAME'`).

## Pattern 5: GHA expression parsing inside shell comments

Workflow validation FAILS on a literal `${{ }}` substring even inside a `run:`
block's `# comment`. Symptom: `unexpected end of input while parsing variable
access`. Workaround: rephrase comments; never include literal `${{ }}` syntax
outside of intended expressions.

Co-symptom: when a workflow fails YAML/expression validation, `gh api`
sometimes shows the run with `name: .github/workflows/X.yml` (path instead of
declared `name:`) AND lists jobs from sibling workflows under it. Misleading
— check actionlint or the raw workflow log.

## Pattern 6: CodeQL actions-queries crash on emoji+printf

`codeql/actions-queries v0.6.25` crashes with `String ... is not valid
Unicode` when parsing `printf '🤖 ... %s\n' "$x"` (emoji + format spec).
Tooling bug, not a real finding. Workaround: replace `printf` with sequential
`echo` lines, or strip the emoji.

## Pattern 7: AUTOMATION_PAT for downstream triggering

Commits pushed by `${{ secrets.GITHUB_TOKEN }}` and events triggered by it
(label add, ready_for_review, etc.) are SUPPRESSED by GitHub anti-recursion
rules — downstream workflows do not fire.

Solution: use a fine-grained PAT in `secrets.AUTOMATION_PAT`. Scopes:
contents R/W, PRs R/W, issues R/W, actions R/W, metadata R. Cannot manage
user-owned Projects v2 (PAT limitation).

## Pattern 8: `pull_request` vs `pull_request_target`

- `pull_request`: PR's head code, NO secrets. Safe for `actions/checkout`.
- `pull_request_target`: base-repo context WITH secrets. Only safe for
  metadata-only API workflows. NEVER `actions/checkout` in this mode.

`copilot-finalize.yml` uses `pull_request_target` (needs to mark draft PRs
ready) but does no checkout. `tech-writer.yml` uses `pull_request` (checks
out PR head, runs LLM analysis on it).

## Pattern 9b: fork-PR contributor approval gate

Symptom: Copilot/bot PRs sit forever with workflows stuck in `action_required`.
You cannot `gh api .../actions/runs/$R/approve` non-fork runs (returns 403
"not from a fork pull request"). Closing+reopening the PR retriggers them.

Real fix is the repo-level setting:

```bash
gh api repos/$O/$R/actions/permissions/fork-pr-contributor-approval \
  -X PUT -f approval_policy=first_time_contributors_new_to_github
```

Values:
- `first_time_contributors` — default; gates all bots that haven't merged yet
- `first_time_contributors_new_to_github` — only gates brand-new GH accounts;
  Copilot/Dependabot/etc. bot accounts are fine. **Recommended for agent repos.**
- `all_external_contributors` — paranoid mode

## Pattern 10: bot-only PR auto-approval

Goal: bots merge unimpeded, humans get reviewed.

1. Branch protection: `required_approving_review_count = 1`,
   `dismiss_stale_reviews = true`.
2. Add `.github/workflows/auto-approve-bots.yml` on `pull_request_target`
   triggers (`opened, reopened, synchronize, ready_for_review`). Job-level
   `if:` allowlists author logins; shell step re-validates author with
   `case`/`esac` defense-in-depth and validates `PR_NUMBER` is integer.
3. Use `secrets.GITHUB_TOKEN` (not AUTOMATION_PAT) — `github-actions[bot]`
   approving counts as a review by a different account than the PR author.
4. Workflow CANNOT auto-approve its own introducing PR (chicken-and-egg);
   admin-merge that one bootstrap PR.

Allowlist examples (case patterns escape brackets):
```bash
case "$PR_AUTHOR" in
  dependabot\[bot\]|copilot-swe-agent\[bot\]|github-actions\[bot\]) ;;
  *) echo "not allowlisted" >&2; exit 1 ;;
esac
```

devops-review LLM may flag env-var interpolation as injection — false
positive when vars are in `"$VAR"` (env vars are not re-evaluated by shell
inside double quotes). Add the shell-side allowlist anyway to satisfy it.

## Pattern 11: GitHub Models payload limits

See [[llm-workflow-payload-limits]]. TL;DR: cap diff at **25 KB** (not 60 KB),
do not include the file corpus, and add a pre-curl byte check that bails to
APPROVE if the assembled request exceeds 90 KB.

## Pattern 12: SIGPIPE on `gh api | head -c`

`gh api ... | head -c "$N"` returns exit code 141 (SIGPIPE) under
`set -euo pipefail` because `head` closes its stdin once the byte cap is
reached, sending SIGPIPE to `gh`. The fix: write the full output to a tempfile
first, then run `head -c` against the file (no pipe → no SIGPIPE):

```bash
# WRONG: trips SIGPIPE under pipefail
gh api "repos/$REPO/pulls/$PR" -H 'Accept: application/vnd.github.v3.diff' \
  | head -c "$MAX_DIFF_BYTES" > /tmp/diff.txt

# RIGHT: tempfile, then truncate
gh api "repos/$REPO/pulls/$PR" -H "Accept: application/vnd.github.v3.diff" > /tmp/diff-full.txt
head -c "$MAX_DIFF_BYTES" /tmp/diff-full.txt > /tmp/diff.txt
```

## Pattern 13: Copilot SWE agent's actual login is `Copilot`

The `pull_request.user.login` for the GitHub Copilot SWE agent's PRs is
literally **`Copilot`** (Bot type, id 198982749). NOT `copilot-swe-agent[bot]`,
NOT `app/copilot-swe-agent`. The `gh pr list` UI may display
`app/copilot-swe-agent` as the author column but that's a render-time alias.

Allowlist gates that check `user.login` against the expected bot identities
must include all three forms for safety:

```yaml
if: >-
  github.event.pull_request.user.login == 'dependabot[bot]' ||
  github.event.pull_request.user.login == 'copilot-swe-agent[bot]' ||
  github.event.pull_request.user.login == 'app/copilot-swe-agent' ||
  github.event.pull_request.user.login == 'Copilot' ||
  github.event.pull_request.user.login == 'github-actions[bot]'
```

Symptom of the bug: the `auto-approve-bots` workflow logs as `completed/skipped`
on every Copilot PR, no error, no approval.

For assigning issues to the bot, the login is `copilot-swe-agent` (no brackets):
`gh issue edit N --add-assignee copilot-swe-agent`. Yes, all three forms are
real and contextually different.

## Reference workflow files (canonical, in tuning-coach)

| File | Purpose |
|------|---------|
| `agent-review.yml` | LLM code reviewer — env-var pattern, jq --rawfile body |
| `tech-writer.yml` | LLM doc auditor with patch commits — path validator |
| `devops-review.yml` | LLM workflow auditor (audit-only) |
| `copilot-finalize.yml` | Auto-mark Copilot PRs ready + label automerge |
| `auto-assign-copilot.yml` | Assign Copilot+owner on `ready-for-coding-agent` label |
| `update-pr-branches.yml` | Keep open PRs current with main on push |
| `auto-approve-bots.yml` | Auto-approve PRs from trusted bot allowlist |

All require `secrets.AUTOMATION_PAT` for downstream triggering.

## Real incidents (chronological)

- 2026-04-26: tech-writer HTTP 413 → dropped doc corpus ([[llm-workflow-payload-limits]])
- 2026-04-26: agent-review.yml shell injection via `` `StorageError::Schema` `` → env-var hardening (PR #89)
- 2026-04-26: CodeQL crash on `printf '🤖 ... %s\n'` → swapped to plain echo
- 2026-04-26: dependabot-skip on `tech-writer-review` + `conventional` required checks → dependabot PRs permanently BLOCKED → dropped `if:` skips (PR #93)
- 2026-04-26: Copilot PRs stuck on `action_required` → flipped fork-pr-contributor-approval to `first_time_contributors_new_to_github`
- 2026-04-26: bot-only PR policy → auto-approve-bots.yml + required_approving_review_count=1 (PR #94)
- 2026-04-26: rolled out agent pipeline to 4 sister repos via greenfield bootstraps; admin-merged because 230 KB diffs exceeded LLM budget; filed `tuning-coach#101` for durable budget-bail in agent/devops/security review workflows
- 2026-04-26: tech-writer SIGPIPE on `gh api | head -c` under `set -euo pipefail` → tempfile-then-truncate (PR #100)
- 2026-04-26: discovered Copilot SWE agent PR author login is `Copilot` (not `copilot-swe-agent[bot]`); auto-approve-bots silently skipped every Copilot PR; added to allowlist (PR #100)
