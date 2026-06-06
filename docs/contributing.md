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
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |

**Before you start:**
- Read the agent file(s) for your area.
- Note in your PR description: `Consulted: <agent-name> per routing matrix`.
- Follow the [conventional commit](https://www.conventionalcommits.org/en/v1.0.0/) style for PR titles and commits.

## Directory Structure

- `sidecar/` — Rust workspace and server
- `sidecar/web/` — Web frontend served by the sidecar (HTML/CSS/JS, tests, dev tooling)
- `simhub/` — SimHub dashboard bundle (`.djson`, `.metadata`, `.png`)
- `docs/` — Documentation site

## Testing
- Web frontend logic (`sidecar/web/`) must be covered by `vitest` tests.
- SimHub dashboard bundle changes should be tested in SimHub before PR.

## Release Process
- The web frontend versions and releases with the sidecar.
- The SimHub dashboard bundle ships as a release asset attached to the sidecar release.

## Questions?
Open an issue or ask in discussions.