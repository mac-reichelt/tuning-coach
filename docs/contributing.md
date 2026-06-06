# Contributing to Tuning Coach

This project uses agent-driven review and strict CI. Before implementing any changes, identify which agents match the files you plan to touch.

## Agent Matrix

| Path | Agent(s) | Purpose |
|------|----------|---------|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |
| `simhub/**` | `qa-engineer` | SimHub dashboard bundle correctness |
| Docs (`docs/**`, `README.md`, etc.) | `tech-writer` | User-facing docs, onboarding, ADR polish |

## Workflow

- **Conventional Commits** required for all PRs and commits.
- **release-please** manages versioning and changelogs.
- **CI** runs lint, tests, and build for Rust and JS frontend.
- **QA review** is triggered for source changes without matching test changes.

## Directory Structure

- `sidecar/` — Rust backend (sidecar)
- `sidecar/web/` — Web frontend served by sidecar (HTML/CSS/JS, tests, dev tooling)
- `simhub/` — SimHub dashboard bundle (`.djson`, metadata, PNG)
- `docs/` — Documentation site

## SimHub Dashboard Bundle

The SimHub dashboard bundle is located in `simhub/`. Import the `.djson` file and associated metadata/PNG into SimHub. The overlay UI is served by the sidecar at `http://127.0.0.1:7778/`.

## Testing

- Rust: `cargo test --workspace`
- JS frontend: `cd sidecar/web && npm install && npm test`

## Releasing

- The sidecar and web frontend are versioned together as `tuning-coach`.
- The SimHub dashboard bundle is released as a zip asset attached to each sidecar release.

## See Also
- [README.md](../README.md)
- [docs/adr/0004-overlay-frontend-relocation.md](adr/0004-overlay-frontend-relocation.md)
