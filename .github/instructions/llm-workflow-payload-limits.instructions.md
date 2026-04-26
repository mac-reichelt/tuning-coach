---
applyTo: '.github/workflows/**'
description: 'GitHub Models API payload-size guidance for workflows. Apply when adding or editing a workflow that posts to models.inference.ai.azure.com or the gh models API.'
---

# GitHub Models API payload limits in workflows

## Symptom

`curl: (22) The requested URL returned error: 413` from
`https://models.inference.ai.azure.com/chat/completions` even with payloads
that look small in raw bytes (~125KB).

## Root cause

Models endpoint rejects payloads well below stated token limits when the
*encoded* JSON is large. Newline-heavy `--rawfile` content explodes via JSON
escaping (`\n`), doubling effective size. Saw rejection at:
- 60KB diff + 80KB doc corpus + 6KB agent prompt
- Worked at 60KB diff + 0KB corpus + 6KB agent prompt

## Pattern that works (mirrors `agent-review.yml`)

Send only:
1. PR diff (cap with `head -c`)
2. Changed-files list
3. Agent prompt file

Do NOT include the full repo file corpus. If LLM needs to update an existing
file, it emits FULL NEW content blind (acceptable trade-off for v1).

## Reference

- `.github/workflows/tech-writer.yml`
- `.github/workflows/agent-review.yml` (canonical pattern)
