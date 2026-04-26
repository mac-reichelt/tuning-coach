---
applyTo: '**'
description: 'Manual rescue playbook for cloud Copilot agent PRs that go DIRTY (merge conflicts) when sibling PRs merge first. Apply when gh pr view N --json mergeStateStatus returns DIRTY and the agent has failed to self-rebase.'
---

# Cloud Copilot agent — DIRTY PR rescue playbook

When the Copilot coding agent (cloud, `copilot-swe-agent`) has multiple PRs
in flight and one lands, siblings go `mergeable_state: DIRTY` with conflicts.
The agent **can** rebase itself if pinged, but often rebases against **stale
main** and stays DIRTY. Faster to do it yourself via worktrees + docker.

## Symptoms

- `gh pr view N --json mergeStateStatus` → `DIRTY`
- Commenting "please rebase" → agent pushes a merge commit against stale main
- Still DIRTY after its attempt

## Manual rescue (per PR)

```bash
# 1. Worktree per branch (one worktree dir per branch keeps WIP isolated)
BR=copilot/feat-whatever
git -C ~/repos/tuning-coach worktree add ~/repos/.worktrees/tuning-coach/$BR $BR
cd ~/repos/.worktrees/tuning-coach/$BR

# 2. Reset and merge fresh main (prefer merge over rebase when the branch
#    already has its OWN merge commits — rebase will re-apply commits already
#    upstreamed and confuse patch application)
git fetch origin --quiet
git reset --hard origin/$BR --quiet
git merge origin/main --no-edit

# 3. Resolve conflicts. Typical pattern in this Rust repo:
#    - Cargo.toml: keep BOTH dep additions
#    - Cargo.lock: checkout --ours then regenerate (see below)
#    - src/main.rs: keep both 'mod X;' declarations

# 4. Regenerate Cargo.lock + verify
git checkout --ours sidecar/Cargo.lock
docker run --rm --network host -u $(id -u):$(id -g) \
  -v "$PWD:/work" -w /work/sidecar \
  -e CARGO_HOME=/work/.cargo-home \
  rust:1.88-slim bash -c "cargo generate-lockfile && cargo check --all-targets"

# 5. Stage, commit
git add -A
GIT_EDITOR=true git commit --no-edit    # accept auto-generated merge msg

# 6. Push (use force-with-lease if rebased; plain push if merged)
git push origin HEAD:$BR

# 7. Rerun action_required workflows (Copilot bot push gate)
for id in $(gh run list --branch $BR --limit 20 \
  --json databaseId,conclusion \
  -q '.[] | select(.conclusion=="action_required") | .databaseId'); do
  gh run rerun $id
done

# 8. Re-arm auto-merge (push clears it)
gh pr merge $N --auto --squash
```

## Why rebase fails and merge succeeds

When the branch tip contains its own merge-from-main commit (`Merge branch
'main' into ...`), a linear `git rebase origin/main` tries to replay commits
that are already upstreamed → apply fails → "dropping ... patch contents
already upstream" → **but the unique branch changes get lost too**.

Merge preserves the branch's state and only needs you to resolve the delta
against today's main. Force-push not needed.

## Resolving blocking review threads

`required_conversation_resolution: true` + Copilot's self-review threads =
BLOCKED even with all-green checks. Outdated threads don't auto-resolve.

```bash
# Find unresolved
gh api graphql -f query='{repository(owner:"OWNER",name:"REPO"){
  pullRequest(number:N){reviewThreads(first:50){
    nodes{id isResolved isOutdated path}}}}}' \
  --jq '.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved==false) | .id'

# Resolve each
for id in $(...); do
  gh api graphql -f query="mutation{resolveReviewThread(input:{threadId:\"$id\"}){thread{isResolved}}}"
done
```

## rustfmt ping-pong

Workflow runs `cargo fmt --all --check`. When you change a call (e.g. drop a
type conversion), the line length changes and rustfmt's wrap decision flips.
Each commit can trigger the opposite decision. Faster: run rustfmt locally
first via `cargo fmt --all` in a docker container that has rustfmt
(`rust:1.88-slim` does **NOT** — `rustfmt` component is missing). Use
`rust:1.88` (full image, ~1.5GB) or apply `rustup component add rustfmt`
in the script. Otherwise: read the diff hunks from CI and apply line-by-line
— but be ready for 2–3 round trips as wrap decisions cascade.

## dependency-review-action scans `.cargo-home/`

If you set `CARGO_HOME=/work/.cargo-home` in your docker rust build and don't
gitignore `.cargo-home/`, every `git add -A` sweeps in 10000+ cached registry
files. The `actions/dependency-review-action` then scans those cached crates'
internal `Cargo.lock` files (NOT just the repo's lockfile!) and flags moderate
vulns in deps that aren't even in the build graph. **Always add `.cargo-home/`
to `.gitignore` before first commit.**

## Additive merge regex (use with care)

For purely additive conflicts (e.g. two PRs each add fields to the same
struct), this Python one-liner resolves all conflict markers by concatenating
both sides:

```python
import re, sys
p = sys.argv[1]
s = open(p).read()
s = re.sub(
    r'<<<<<<< HEAD\n(.*?)=======\n(.*?)>>>>>>> [^\n]+\n',
    lambda m: m.group(1) + m.group(2),
    s, flags=re.DOTALL)
open(p, 'w').write(s)
```

**Mandatory post-checks** (the regex silently produces buggy code otherwise):

```bash
python3 -c "import ast; ast.parse(open('path/to/file.py').read())"  # syntax check
grep -nE '^\s*(def|async def) [a-z_]+' path/to/file.py | sort -k2 | uniq -d  # dup methods
grep -nE '^\s+[a-z_]+: [A-Z]' path/to/config.py | awk '{print $1}' | sort | uniq -d  # dup fields
```

Python silently accepts duplicate `def` and duplicate dataclass fields — the
**last** definition wins, shadowing earlier ones. Anti-pattern: `with`
statements. Python's `with (a, b)` tuple form vs `with a, \\ b`
line-continuation form is NOT mergeable by this regex — needs hand-resolution.
