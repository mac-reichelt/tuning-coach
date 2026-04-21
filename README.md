# tuning-coach

> **A real-time race engineer for sim racing.** Live telemetry → specific tuning recommendations.

[![CI](https://github.com/mac-reichelt/tuning-coach/actions/workflows/ci.yml/badge.svg)](https://github.com/mac-reichelt/tuning-coach/actions/workflows/ci.yml)
[![CodeQL](https://github.com/mac-reichelt/tuning-coach/actions/workflows/codeql.yml/badge.svg)](https://github.com/mac-reichelt/tuning-coach/actions/workflows/codeql.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`tuning-coach` is a [SimHub](https://www.simhubdash.com/) overlay backed by a
Rust sidecar. It ingests live telemetry from Forza Motorsport (2023), watches
how the car behaves on track, and surfaces specific numeric tuning
recommendations — with the rationale a real race engineer would give you.

> ⚠️ **Pre-1.0 / heavy development.** Nothing is stable yet — APIs, schemas,
> recommendations all subject to change. Star the repo if you want to follow
> along.

## Features (planned MVP)

- ✅ **Live coaching** — minimal HUD that surfaces only when the coach has something to say
- ✅ **Recorded session analysis** — replay any session through the heuristics pipeline
- ✅ **Post-session report** — markdown + JSON summary with per-lap deltas
- ✅ **Specific numeric recommendations** — *"front spring rate 85 → 92 N/mm"*, not *"stiffen front"*
- ✅ **Full Forza tuning surface** — tires, gearing, alignment, ARB, springs, damping, aero, brakes, diff
- ✅ **Locked-parameter aware** — only suggests adjustments your car can actually make
- ✅ **Driving-style auto-detect** — tunes recommendations to your inputs
- ✅ **Lap validity tracking** — detects dirty laps, pit stops, lap resets
- ✅ **Hotkey overrides** — keyboard or Stream Deck for in-session player input
- ✅ **Optional LLM coaching** — bring your own OpenAI-compatible endpoint (local or cloud)

See [docs/PLAN.md](docs/PLAN.md) for the phased roadmap.

## Architecture

```
┌──────────────────┐  UDP   ┌──────────────────────┐   WS    ┌────────────────┐
│ Forza Motorsport │ ─────▶ │  tuning-coach (Rust) │ ◀─────▶ │ SimHub overlay │
└──────────────────┘        │  - UDP listener      │         │  (HTML/JS)     │
                            │  - Heuristics engine │         │  - HUD         │
┌──────────────────┐ HTTP/  │  - SQLite history    │ HTTP    │  - Setup form  │
│ SimHub Hotkeys   │──────▶ │  - Hotkey REST API   │ ◀────── │    (paused)    │
│ (kbd/StreamDeck) │ webhook│  - LLM proxy (opt.)  │         └────────────────┘
└──────────────────┘        └──────────────────────┘
```

- The **Rust sidecar** ingests Forza UDP telemetry directly (no SimHub
  dependency for telemetry), runs heuristics, persists to SQLite, and serves
  the overlay over WebSocket.
- The **SimHub overlay** is a vanilla HTML/JS page — no build step. It connects
  to the sidecar over localhost.
- **In-session input** uses SimHub Global Hotkeys (works with keyboard or
  Stream Deck) that POST to a sidecar webhook.
- **Out-of-session input** uses HTML forms in the overlay (car upgrades, locked
  parameters, preferences).
- **LLM coaching** is optional and OpenAI-compatible — point it at a local
  Lemonade instance or a cloud provider.

## Quickstart

> Prerequisites: Forza Motorsport (2023), [SimHub](https://www.simhubdash.com/),
> Rust toolchain (stable).

```bash
# Clone
git clone https://github.com/mac-reichelt/tuning-coach.git
cd tuning-coach

# Build the sidecar
cd sidecar
cargo build --release

# Run the sidecar (default UDP port 7777, default WS port 7778)
./target/release/tuning-coach-sidecar
```

```bash
# Install the overlay in SimHub
# (Symlink or copy overlay/ into your SimHub DashTemplates folder)
```

In Forza:
1. **Settings → HUD and Gameplay → Data Out**
2. Set **Data Out IP Address**: `127.0.0.1`
3. Set **Data Out Port**: `7777`
4. Enable **Data Out**

Full setup: [docs/getting-started.md](docs/getting-started.md).

## Documentation

- [Getting Started](docs/getting-started.md) — install + first-run
- [Plan + Roadmap](docs/PLAN.md) — phased delivery
- [Architecture Decisions](docs/adr/) — ADRs
- [API Reference](docs/reference/api.md) — sidecar HTTP/WS API (TBD)
- Telemetry Schema — Forza Dash packet (TBD)

## Heuristics Reference

- [Lap Validity Heuristics + Thresholds](docs/reference/lap-validity.md)

## Status

| Component | Status | Version |
|-----------|--------|---------|
| Sidecar — UDP parser | 🚧 in design | 0.1.0 |
| Sidecar — heuristics engine | 🔜 planned | — |
| Sidecar — WS API | 🚧 in design | 0.1.0 |
| Overlay — HUD | 🔜 planned | 0.1.0 |
| Overlay — setup form | 🔜 planned | — |
| LLM integration | 🔜 planned (optional) | — |
| OCR for in-game tuning screen | 🔜 stretch | — |

## Showcase

This repo intentionally exercises a wide swath of GitHub features:

- **Copilot coding agent** — issues labeled `ready-for-coding-agent` are
  auto-assigned for autonomous PR development
- **Copilot setup steps** — `.github/copilot-setup-steps.yml` preinstalls Rust
  + Tesseract for the cloud agent
- **Vendored agent roster** — full software-team simulation in
  `.github/agents/` (coordinator, producer, architect, software-engineer,
  qa-engineer, devops-engineer, tech-writer, code-review, security-review,
  plus project-specific telemetry-expert and race-engineer)
- **Conventional Commits + release-please** — fully automated SemVer +
  CHANGELOG + GitHub Releases
- **Monorepo releases** — sidecar and overlay versioned independently
- **GitHub Pages** — `/docs` deploys automatically
- **Issue Forms** — structured intake (feature, bug, security, tuning rule)
- **CodeQL + Dependabot + Dependency Review + Secret Scanning** — security
  baseline
- **Auto-merge** — `automerge` label + green checks → squash merge
- **Auto-labeler** — area labels from changed paths

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The team-of-agents workflow is documented
in [.github/agents/MANIFEST.md](.github/agents/MANIFEST.md) and each agent's own
`.agent.md` file.

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

- Inspired by the [Tune-It-Yourself](https://play.google.com/store/search?q=tune-it-yourself)
  Android app's data-driven tuning approach.
- Built on [SimHub](https://www.simhubdash.com/) for overlay rendering.
- Forza UDP telemetry schema documented by the racing sim community.
