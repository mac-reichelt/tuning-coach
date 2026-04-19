# Contributing to tuning-coach

Thanks for considering a contribution! This project showcases an
agent-augmented development workflow, and contributions of any size are
welcome — from typo fixes to entire features.

## Code of Conduct

By participating, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Quick Links

- [Open issues](https://github.com/mac-reichelt/tuning-coach/issues)
- [Discussions](https://github.com/mac-reichelt/tuning-coach/discussions)
- [Plan + Roadmap](docs/PLAN.md)
- [Architecture Decisions](docs/adr/)

## How We Work

We use a **simulated software team** of Copilot agents:

- **producer** — turns requests into user stories with acceptance criteria
- **architect** — owns ADRs and interface contracts
- **software-engineer** — implements stories
- **qa-engineer** — owns test coverage
- **devops-engineer** — owns CI/CD and releases
- **tech-writer** — owns docs
- **code-review** + **security-review** — gate every non-trivial PR
- **coordinator** — dispatches work across the team

Plus project-specific experts:
- **telemetry-expert** — Forza UDP packet schema, sim physics
- **race-engineer** — chassis tuning theory and rule validation

Agent definitions live in [`.github/agents/`](.github/agents/).

You don't need to use the agents to contribute — humans are first-class
contributors. But if you do, the patterns are documented in each agent file.

## Development Setup

### Prerequisites

- Rust stable (latest) — install via [rustup.rs](https://rustup.rs/)
- Node.js 20+ (only if hacking on the overlay tooling)
- A Forza Motorsport (2023) install — for live testing
- [SimHub](https://www.simhubdash.com/) — for overlay rendering
- (Linux dev only) `pkg-config libssl-dev libsqlite3-dev tesseract-ocr libtesseract-dev libleptonica-dev`

### Clone + build

```bash
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach
cd sidecar && cargo build
```

### Run tests + lint locally

```bash
# Rust
cd sidecar
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Overlay (when applicable)
cd overlay
npm ci
npm run lint --if-present
npm test --if-present
```

These match what CI runs.

## Workflow

1. **Find or open an issue.** Use the Issue Forms in
   [.github/ISSUE_TEMPLATE](.github/ISSUE_TEMPLATE) — they're structured to
   feed directly into the producer agent's intake.
2. **Branch.** `git checkout -b <type>/<area>-<short-slug>` (e.g.,
   `feat/sidecar-udp-parser`).
3. **Implement.** Smallest set of changes that satisfy the acceptance criteria.
4. **Test.** Add or update tests for new behavior. Run lint + test locally.
5. **Commit.** Conventional Commits format (see below).
6. **PR.** Use the [PR template](.github/PULL_REQUEST_TEMPLATE.md). Link the
   issue with `Closes #N`.
7. **CI.** All required checks must pass.
8. **Review.** A reviewer (human or agent) will leave feedback or approve.
9. **Merge.** Squash-merge keeps history clean. release-please handles the
   version bump.

## Conventional Commits

PR titles **must** follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <imperative summary>
```

Allowed types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`,
`ci`, `chore`, `revert`.

Examples:
- `feat(sidecar): parse Forza UDP Dash packet`
- `fix(overlay): debounce hotkey acks to prevent UI flicker`
- `docs(readme): add Stream Deck profile import instructions`

Add `!` after the type for breaking changes:
```
feat(api)!: rename /telemetry endpoint to /stream/telemetry
```

…and include a `BREAKING CHANGE:` footer in the body.

This is enforced by the `pr-title` workflow.

## Code Style

### Rust (sidecar)

- `cargo fmt` — formatting (no opinions; use rustfmt defaults)
- `cargo clippy --workspace --all-targets -- -D warnings` — lints
- `thiserror` for library errors, `anyhow` for binary main
- `tracing` for structured logging
- `tokio` for async; prefer channels over shared mutex
- Tests in `#[cfg(test)] mod tests` blocks; integration tests in `tests/`

### JavaScript (overlay)

- ES modules, vanilla — no framework, no build step
- ESLint + Prettier config (TBD)
- `vitest` for tests

## Security

If you find a security issue, **please don't open a public issue**. Use
[private vulnerability reporting](https://github.com/mac-reichelt/tuning-coach/security/advisories/new).
See [SECURITY.md](SECURITY.md).

## License

By contributing, you agree your contribution will be licensed under the [MIT
License](LICENSE).
