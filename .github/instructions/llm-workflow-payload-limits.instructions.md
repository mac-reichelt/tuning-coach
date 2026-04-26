---
applyTo: '.github/workflows/**'
description: 'GitHub Models API payload-size guidance for workflows. Apply when adding or editing a workflow that posts to models.inference.ai.azure.com or the gh models API.'
---


# GitHub Models API payload limits in workflows

## Symptom
`curl: (22) The requested URL returned error: 413` from
`https://models.inference.ai.azure.com/chat/completions` even with payloads
that look small in raw bytes.

## Root cause
GitHub Models free tier has an **~8K-token input limit** (~32KB of UTF-8
text). Above that, the endpoint hard-rejects with 413. Newline-heavy
`--rawfile` content explodes via JSON escaping (`\n`), so the encoded body
is ~30% bigger than the raw file. Empirical thresholds:

| Diff size | Result |
|-----------|--------|
| 25 KB diff + 6 KB agent prompt + 2 KB system prompt | OK |
| 60 KB diff + 6 KB agent prompt | 413 |
| 60 KB diff + 80 KB doc corpus + 6 KB agent prompt | 413 (massively over) |

## Pattern that works
1. **Cap MAX_DIFF_BYTES at 25000** (not 60000 as initially set)
2. Send only: diff (capped), changed-files list, agent prompt file
3. Do NOT include the full repo file corpus
4. Add a belt-and-suspenders pre-curl check on `wc -c < /tmp/req.json`:
   if assembled body > 90 KB, skip the LLM call and emit an APPROVE-equivalent
   verdict telling humans to verify manually.

```bash
REQ_BYTES=$(wc -c < /tmp/req.json)
if [ "$REQ_BYTES" -gt 90000 ]; then
  echo "::warning::Request body ${REQ_BYTES} bytes exceeds LLM budget; defaulting to APPROVE"
  jq -n --argjson n "$REQ_BYTES" '{verdict:"APPROVE",summary:("PR diff plus context (" + ($n|tostring) + " bytes) exceeds LLM budget. Skipping inline review; please verify manually."),patches:[]}' > /tmp/verdict.json
  # ... emit outputs and exit 0
fi
```

## Reference
- See [[tuning-coach]] `.github/workflows/tech-writer.yml` (PR #100)
- The same fix needs propagation to `agent-review.yml`, `devops-review.yml`,
  `security-review.yml` — they all hit the same endpoint with similar prompts.

## Real incident
2026-04-26: rolling out the agent pipeline to 4 sister repos via greenfield
bootstrap PRs (~5600 LOC each, 230 KB diffs). Every LLM review workflow
returned 413 simultaneously. Bootstrap PRs were admin-merged because they're
inherently too big for LLM review. Filed `tuning-coach#101` to propagate the
budget fix to the other review workflows.
