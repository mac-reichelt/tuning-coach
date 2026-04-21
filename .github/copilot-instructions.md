# tuning-coach — Copilot Instructions

A SimHub overlay + Rust sidecar that ingests live Forza Motorsport telemetry,
analyzes driver inputs and chassis behavior, and surfaces specific numeric
tuning recommendations across the full Forza Motorsport tuning surface.

## Audience

These instructions are read by:
- **GitHub Copilot coding agent** (cloud) — when issues are assigned to it
- **Local Copilot CLI sessions** — when working on this repo
- **Vendored team agents** in `.github/agents/` — for shared conventions

The project-specific agents in `.github/agents/` (telemetry-expert, race-engineer)
are domain experts. The 9 vendored team agents (coordinator, producer, architect,
software-engineer, qa-engineer, devops-engineer, tech-writer, code-review,
security-review) handle workflow.

## Stack

| Component | Tech | Path |
|-----------|------|------|
| Sidecar | Rust 2024 edition, tokio async, axum HTTP/WS | `sidecar/` |
| Overlay | Vanilla HTML/CSS/JS (SimHub overlay template) | `overlay/` |
| Storage | SQLite via `rusqlite` (sidecar-owned) | `sidecar/data/` |
| Docs | Markdown → GitHub Pages (Jekyll) | `docs/` |
| CI | GitHub Actions | `.github/workflows/` |
| Releases | release-please monorepo mode | `.github/release-please-config.json` |

## Architecture (one-liner)

`Forza --(UDP)--> tuning-coach (Rust sidecar) <--(WS)--> SimHub overlay`

Sidecar ingests Forza UDP telemetry directly (independent of SimHub), runs
heuristics, persists to SQLite, optionally calls an OpenAI-compatible LLM, and
serves the overlay over WebSocket. SimHub Global Hotkeys POST to a sidecar
webhook for in-session player actions (mark dirty lap, pit, snooze, etc.).

See `docs/adr/` for architecture decisions.

## Conventions

### Versioning + Commits

- **Conventional Commits required** on all PR titles + commits — enforced by
  `pr-title.yml` workflow.
- **release-please** drives SemVer + CHANGELOG + tags from commits.
- **Monorepo mode**: `sidecar` and `overlay` versioned independently
  (`sidecar-vX.Y.Z`, `overlay-vX.Y.Z`).
- **Pre-1.0** (current): `feat`/breaking → minor; `fix`/`perf` → patch.
- Use `feat!:` + `BREAKING CHANGE:` footer for breaking changes.

### Rust (sidecar)

- Edition 2024, MSRV pinned in `Cargo.toml`
- `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings`
- Tests: `cargo test --workspace` — use `insta` for snapshot, `proptest` for
  parsers
- Errors: `thiserror` for library errors, `anyhow` for binary main
- Logging: `tracing` + `tracing-subscriber`; structured (JSON in release)
- Config: `figment` (env > file > defaults), validated at startup
- Async: `tokio` (full features), prefer channels over shared mutex
- HTTP/WS: `axum` + `tokio-tungstenite`
- SQLite: `rusqlite` with bundled feature; migrations in `sidecar/migrations/`

### JS (overlay)

- Vanilla — no build step for the overlay itself
- ES modules, top-level await OK (modern browsers / SimHub uses Chromium)
- Lint: `eslint` (flat config) + `prettier`
- Tests: `vitest` for any non-trivial logic
- DOM-only, no framework — overlay is render-only

### Docs

- `docs/` is the source for the GitHub Pages site
- Active voice, present tense, second person for instructions
- Show real terminal output, not invented examples
- Link every concept on first mention

## Development Workflow

1. **Pick an issue** labeled `ready-for-coding-agent` (cloud agent) or grab any
   open issue (local sessions).
2. **Read the linked ADR** if the issue references one (`docs/adr/`).
3. **Branch**: `<type>/<area>-<short-slug>` (e.g., `feat/sidecar-udp-parser`).
4. **Implement** following the conventions above.
5. **Test locally**: `cargo test` + `npm test` as relevant.
6. **Commit** with conventional commit format.
7. **Open PR** using the template; link the issue (`Closes #N`).
8. CI must pass: `ci`, `codeql`, `dependency-review`, `pr-title`, `agent-review`.
9. Merge via squash; release-please handles the rest.

## Key Files

| File | Purpose |
|------|---------|
| `.github/copilot-setup-steps.yml` | Cloud agent environment (Rust + Tesseract) |
| `.github/agents/MANIFEST.md` | Vendored team agent versions |
| `.github/agents/telemetry-expert.agent.md` | Forza UDP packet schema, sim physics |
| `.github/agents/race-engineer.agent.md` | Real-world tuning knowledge |
| `docs/adr/` | Architecture Decision Records |
| `sidecar/Cargo.toml` | Rust workspace + crate |
| `overlay/index.html` | SimHub overlay entry |
| `.github/release-please-config.json` | release-please monorepo config |

## Anti-Patterns

❌ Adding a build step to the overlay — keep it loadable directly by SimHub.
❌ Shared mutable state in the sidecar — use channels/actors.
❌ Logging telemetry to stdout in release builds — flood + privacy.
❌ Coupling the heuristics engine to the WS layer — keep them separable.
❌ Storing player setups in cleartext if they include account info — out of scope
   today, but design with encryption-at-rest as a future option.
❌ Auto-pushing telemetry to any cloud service without an explicit user opt-in.

## Plan & Roadmap

The full project plan with phased roadmap lives in `docs/PLAN.md` (TBD — see
the producer's open issues for the phase 1 backlog).

## Related

- Project plan and decisions: `docs/PLAN.md`, `docs/adr/`
- Team workflow: `.github/agents/coordinator.agent.md`
- Domain knowledge: `.github/agents/telemetry-expert.agent.md`,
  `.github/agents/race-engineer.agent.md`
