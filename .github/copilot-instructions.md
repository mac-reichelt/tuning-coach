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
| Sidecar | Rust 2021 edition (MSRV 1.80), tokio async, axum HTTP/WS | `sidecar/` |
| SimHub dashboard | SimHub-importable `.djson` bundle | `simhub/` |
| Web frontend | Vanilla HTML/CSS/JS (served by sidecar) | `sidecar/web/` |
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
- **Single package**: `tuning-coach` versioned as `vX.Y.Z`. Each release ships
  the compiled sidecar binaries **and** the `simhub/` dashboard bundle; the web
  frontend is embedded in the sidecar binary.
- **Pre-1.0** (current): `feat`/breaking → minor; `fix`/`perf` → patch.
- Use `feat!:` + `BREAKING CHANGE:` footer for breaking changes.

### Rust (sidecar)

- Edition 2021, `rust-version = "1.80"` pinned in `sidecar/Cargo.toml` (CI uses
  stable toolchain). Run all `cargo` commands from `sidecar/`.
- `cargo fmt --all --check` + `cargo clippy --workspace --all-targets -- -D warnings`
- Tests: `cargo test --workspace --all-features` (matches CI). Run a **single
  test** with `cargo test --workspace <test_name>` or a module with
  `cargo test telemetry::tests`. Use `insta` for snapshot (review with
  `cargo insta review`), `proptest` for parsers.
- On Linux, the build needs system deps: `pkg-config libssl-dev libsqlite3-dev
  tesseract-ocr libtesseract-dev libleptonica-dev`.
- Errors: `thiserror` for library errors, `anyhow` for binary main
- Logging: `tracing` + `tracing-subscriber`; structured (JSON in release)
- Config: `figment` (env > file > defaults), validated at startup
- Async: `tokio` (full features), prefer channels over shared mutex
- HTTP/WS: `axum` + `tokio-tungstenite`
- SQLite: `rusqlite` with bundled feature; migrations in `sidecar/migrations/`

### JS (web frontend)

- Vanilla — no build step for the web frontend itself. Source lives in
  `sidecar/web/src/` (ES modules); tests are `*.test.js` colocated in
  `sidecar/web/`.
- ES modules, top-level await OK (modern browsers / SimHub uses Chromium)
- Tests: `vitest` (jsdom). From `sidecar/web/`: `npm ci` then `npm test`
  (= `vitest run`). Single file: `npx vitest run dyno-graph.test.js`; single
  test: `npx vitest run -t "<name>"`. Coverage: `npm run coverage`.
- CI runs `npm run lint --if-present` — there is **no** eslint/prettier config
  today, so lint is currently a no-op. Add the tooling before relying on it.
- DOM-only, no framework — frontend is render-only

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
| `.github/instructions/MANIFEST.md` | Vendored copilot instructions (security, workflow hardening, cloud-agent playbooks) |
| `docs/adr/` | Architecture Decision Records |
| `sidecar/Cargo.toml` | Rust workspace + crate |
| `sidecar/web/index.html` | Web frontend entry |
| `.github/release-please-config.json` | release-please monorepo config |

## Agent Routing

> **Note on paths:** the sidecar is currently flat — `sidecar/src/` holds
> `telemetry.rs`, `recommendation.rs`, `storage.rs`, `session_state.rs`,
> `lap_validity.rs`, `hotkeys.rs`, `overlay.rs`, `main.rs`. The globs below
> (e.g. `heuristics/**`, `recommendations/**`, `forza_*.rs`) include
> not-yet-created paths; match against the closest existing file
> (`telemetry.rs` for Forza packets, `recommendation.rs` for tuning logic).

Before implementing any changes, identify which agents match the files you
plan to touch using the routing matrix below. Read those agent files before
writing code, and note them in your PR description as:
`Consulted: <agent-name> per routing matrix`.

| Path glob | Agent(s) to consult before editing | Rationale |
|---|---|---|
| `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs` | `telemetry-expert` | Packet schema is the source of truth; agent file must stay in sync with code |
| `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**` | `race-engineer` + `telemetry-expert` | Tuning logic must reflect real-world practice + correct telemetry semantics |
| `sidecar/src/storage*.rs`, `sidecar/migrations/**` | `architect` | Schema migrations need ADR consideration |
| `docs/adr/**` (new files) | `architect` | New ADRs need review against existing decisions |
| `.github/workflows/**`, `.github/actions/**` | `devops-engineer` + `security-review` | CI/CD correctness + security (covered by devops-review.yml and security-review.yml) |
| Any file with auth, secrets, OIDC, crypto in name or context | `security-review` | OWASP / Zero Trust pass |
| New crates, new public modules, new sidecar tests | `qa-engineer` | Test strategy + coverage |
| `sidecar/web/**` (logic changes, not pure CSS) | `qa-engineer` | Web frontend test discipline (vitest) |

### Automated routing

Four path-scoped review workflows enforce this matrix on every PR:

| Workflow | Check name | In-scope when |
|---|---|---|
| `.github/workflows/security-review.yml` | `security-review verdict` | Workflow/action files, shell scripts, or security-sensitive file names change |
| `.github/workflows/qa-review.yml` | `qa-review verdict` | `sidecar/src/**` or `sidecar/web/**` changes without accompanying test-file changes |
| `.github/workflows/telemetry-review.yml` | `telemetry-review verdict` | `sidecar/src/telemetry.rs`, `sidecar/src/forza_*.rs`, or `telemetry-expert.agent.md` changes |
| `.github/workflows/heuristics-review.yml` | `heuristics-review verdict` | `sidecar/src/heuristics/**`, `sidecar/src/recommendations/**`, or `race-engineer.agent.md` changes |

All four follow the skip-success pattern: out-of-scope PRs post `success` so
the checks can be required in branch protection without blocking unrelated work.

## Anti-Patterns

❌ Adding a build step to the web frontend — keep it loadable directly by the sidecar.
❌ Shared mutable state in the sidecar — use channels/actors.
❌ Logging telemetry to stdout in release builds — flood + privacy.
❌ Coupling the heuristics engine to the WS layer — keep them separable.
❌ Storing player setups in cleartext if they include account info — out of scope
   today, but design with encryption-at-rest as a future option.
❌ Auto-pushing telemetry to any cloud service without an explicit user opt-in.

## Plan & Roadmap

The full project plan with phased roadmap lives in `docs/PLAN.md`.

## Related

- Project plan and decisions: `docs/PLAN.md`, `docs/adr/`
- Team workflow: `.github/agents/coordinator.agent.md`
- Domain knowledge: `.github/agents/telemetry-expert.agent.md`,
  `.github/agents/race-engineer.agent.md`
- Workflow / agent gotchas: `.github/instructions/llm-workflow-hardening.instructions.md`,
  `.github/instructions/cloud-agent-ci-gate.instructions.md`,
  `.github/instructions/cloud-agent-dirty-pr.instructions.md`
