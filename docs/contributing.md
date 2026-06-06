# Contributing to Tuning Coach

This project uses agent-driven review and strict CI. Before implementing any changes, identify which agents match the files you plan to touch.

## Agent Matrix

| Path | Agent(s) | Purpose |
|------|----------|---------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `sidecar/web/**` | `qa-engineer` | Web frontend test discipline (vitest) |
| `simhub/**` | `qa-engineer` | SimHub dashboard bundle correctness |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security |
| `docs/adr/**` (new files) | `architect` | New ADRs need review |
