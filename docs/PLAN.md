# SimHub Tuning Coach — Forza Motorsport (2023)

A SimHub overlay + Rust sidecar that ingests live Forza telemetry, analyzes driver
inputs and chassis behavior, and surfaces specific numeric tuning recommendations
across the full Forza Motorsport tuning surface. Inspired by Tune-It-Yourself but
delivered entirely as a SimHub overlay experience.

## Decisions (locked)

| Area | Choice |
|------|--------|
| Feedback modes | Live coaching + recorded session analysis + post-session report |
| Engineer logic | Heuristics primary; optional LLM layer for style-tuned phrasing/advice |
| Driving style | Auto-detected from throttle/brake/steering/trail-braking patterns |
| MVP scope | **All Forza Motorsport tuning categories** (see Tuning Surface below) |
| Locked parameters | Detected/declared per car; coach skips locked params, suggests upgrade if relevant |
| Storage | SQLite (canonical history) + on-demand JSON export |
| Output format | Specific numeric adjustments (e.g. `front springs 85 → 92 N/mm`) |
| Overlay UX | Minimal HUD — only surfaces when coach has something to say |
| Player input (out of session) | HTML inputs in overlay (car upgrades, locked params, preferences) |
| Player input (in session) | SimHub Global Hotkeys (works with keyboard or Stream Deck) |
| Setup ingestion (stretch) | Rust sidecar OCR's the in-game tuning screen via Windows screen capture |
| Lap validity | Coach detects dirty laps, pit stops, lap resets/restarts |
| LLM provider | Generic OpenAI-compatible API (works for Lemonade local + cloud) |
| Car/track ID | Import SimHub's built-in car/track list; auto-identify via telemetry |
| Repo | Private GitHub under `mac-reichelt`, open-source later |
| Stack | Vanilla HTML/CSS/JS overlay + Rust sidecar |
| Telemetry path | Sidecar ingests Forza UDP directly; overlay reads from sidecar via WS |

## Tuning Surface (Forza Motorsport 2023)

All categories are MVP scope. Each parameter is tagged `locked` if the car lacks
the prerequisite upgrade (race transmission, race diff, adjustable suspension, etc.).

| Category    | Parameters |
|-------------|------------|
| Tires       | Pressure F, Pressure R |
| Gearing     | Final Drive, Gear 1–7 (as available) |
| Alignment   | Camber F, Camber R, Toe F, Toe R, Caster |
| Anti-Roll   | ARB F, ARB R |
| Springs     | Spring Rate F, Spring Rate R, Ride Height F, Ride Height R |
| Damping     | Bump F, Bump R, Rebound F, Rebound R |
| Aero        | Downforce F, Downforce R |
| Brakes      | Balance, Pressure |
| Differential| Accel Lock F/R/C, Decel Lock F/R/C, AWD Split |

## Architecture

```
┌──────────────────┐  UDP   ┌──────────────────────┐   WS    ┌────────────────┐
│ Forza Motorsport │ ─────▶ │  tuning-coach (Rust) │ ◀─────▶ │ SimHub overlay │
└──────────────────┘        │  - UDP listener      │         │  (HTML/JS)     │
                            │  - Heuristics engine │         │  - HUD         │
┌──────────────────┐ HTTP/  │  - SQLite history    │ HTTP    │  - Input forms │
│ SimHub Hotkeys   │──────▶ │  - Hotkey REST API   │ ◀────── │    (out-of-    │
│ (kbd/StreamDeck) │ webhook│  - LLM proxy         │         │     session)   │
└──────────────────┘        │  - Screen-capture OCR│         └────────────────┘
                            │    (stretch)         │
                            └──────────────────────┘
                                       │
                                       ▼
                               ┌───────────────┐
                               │ OpenAI-compat │
                               │ LLM endpoint  │
                               └───────────────┘
```

### Sidecar responsibilities
- UDP listener on configurable port, parses Forza telemetry packets: primary 331-byte FM 2023 Dash, plus legacy 311-byte Dash and 232-byte Sled-only variants
- Session state machine: idle → in-session → between-laps → post-session
- **Lap-validity detection:** dirty laps (off-track wheels, contact spikes,
  unrealistic grip jumps), pit-stop entry/exit, lap reset/rewind detection
- Real-time heuristics pipeline across full tuning surface
- Driving-style classifier (auto-detected profile)
- Car/track identification (telemetry fingerprint vs imported SimHub list)
- Per-car setup model: known params, locked params, current values, history
- SQLite persistence (sessions, laps, telemetry snapshots, recommendations,
  car_setups, user_preferences)
- Post-session report generator (markdown/JSON)
- Optional LLM proxy: structured ctx → OpenAI-compat endpoint
- WebSocket + REST API for overlay
- REST endpoint hit by SimHub global hotkeys (mark dirty / pit / snooze / etc.)
- **Stretch:** Windows Desktop Duplication API capture of tuning screen +
  Tesseract OCR to auto-populate current setup values and locked flags

### Overlay responsibilities
- WebSocket client to sidecar; pure render layer
- **Live HUD:** hidden by default, slide-in panel when sidecar emits a recommendation
- **Setup form (out-of-session):** HTML inputs for car upgrades installed,
  manually-flagged locked parameters, current setup values when OCR isn't used,
  and player preferences (style override, notification frequency)
- Dismiss/snooze/history-toggle controls
- Status indicator: lap valid / dirty / pit / reset

### Input model
- **Out-of-session (paused, garage, between sessions):** click into overlay,
  use HTML form (car/setup data, preferences)
- **In-session (driving):** SimHub Global Hotkeys mapped to sidecar webhooks:
  - `Mark current lap dirty` (in case auto-detect missed it)
  - `Pit in / pit out` (override)
  - `Snooze coach` (mute notifications until next session)
  - `Request feedback now` (force a recommendation summary)
  - `Reset session` (start fresh)
- Hotkeys work identically from keyboard or Stream Deck (Stream Deck just sends
  the bound hotkey).

## Phased Roadmap

> **Per-phase work lives in [GitHub milestones + project](https://github.com/users/mac-reichelt/projects/2).**
> This document is the *architectural* source of truth (decisions, contracts, philosophy).
> The roadmap below is an index — for current scope, status, and open issues per
> phase, click into the milestone.

Roadmap reordered 2026-04-26 to optimize for hands-on user validation at every
phase, minimal blocking deps, and earliest risk burndown.

| # | Phase | Milestone | Notes |
|---|-------|-----------|-------|
| 1 | Sidecar foundation ✅ | [Phase 1](https://github.com/mac-reichelt/tuning-coach/milestone/1) | shipped `sidecar-v0.1.1` |
| 2 | Lap validity + session state ✅ | [Phase 2](https://github.com/mac-reichelt/tuning-coach/milestone/2) | shipped `overlay-v0.1.1` |
| 3 | Overlay shell (product skeleton) | [Phase 3](https://github.com/mac-reichelt/tuning-coach/milestone/3) | first user-visible phase; unblocks everything visible after |
| 4 | Setup form + car/track ID | [Phase 4](https://github.com/mac-reichelt/tuning-coach/milestone/4) | `CarOrdinal`/`TrackOrdinal` are documented packet fields ([ref](https://forums.forza.net/t/data-out-feature-in-forza-motorsport/651333)) |
| 5 | OCR feasibility spike | [Phase 5](https://github.com/mac-reichelt/tuning-coach/milestone/5) | bounded; ≥70% per-field accuracy bar; gates Phase 13 |
| 6 | In-session hotkeys + Stream Deck | [Phase 6](https://github.com/mac-reichelt/tuning-coach/milestone/6) | small, self-contained; another tactile surface post-overlay |
| 7 | Heuristics: suspension + ride height + ARB + damping | [Phase 7](https://github.com/mac-reichelt/tuning-coach/milestone/7) | first heuristic; pays trailblazer cost (segmentation, engine wiring, locked-param fallback) |
| 8 | Heuristics: remaining categories (8a/8b/8c) | [Phase 8](https://github.com/mac-reichelt/tuning-coach/milestone/8) | 8a brakes+tires / 8b gearing+diff / 8c alignment+aero |
| 9 | Post-session report | [Phase 9](https://github.com/mac-reichelt/tuning-coach/milestone/9) | markdown + JSON; cross-session deltas |
| 10 | Driving-style auto-detect | [Phase 10](https://github.com/mac-reichelt/tuning-coach/milestone/10) | classifier over throttle/brake/steering |
| 11 | Recorded session replay | [Phase 11](https://github.com/mac-reichelt/tuning-coach/milestone/11) | replay saved telemetry through heuristics + scrub UI |
| 12 | LLM phrasing layer | [Phase 12](https://github.com/mac-reichelt/tuning-coach/milestone/12) | OpenAI-compat; feature-flagged; heuristics always primary |
| 13 | OCR setup capture (gated by Phase 5) | [Phase 13](https://github.com/mac-reichelt/tuning-coach/milestone/13) | only if Phase 5 clears accuracy bar |
| 14 | Multi-game abstraction | [Phase 14](https://github.com/mac-reichelt/tuning-coach/milestone/14) | iRacing or ACC as second game |

### Future / post-MVP backlog (not yet scheduled)

- **Track corner database** — auto-derive named corner index per track from a
  reference lap (curvature peaks, distance markers); replaces anonymous
  `(distance, lat_g)` indexing introduced in Phase 7. Recommended slot:
  between Phase 8 and Phase 9 once heuristics expose how often "same corner"
  ambiguity matters.

## Cross-cutting open questions (deferrable)

- Report delivery: file on disk vs sidecar HTTP vs both (defer to Phase 9)
- Installer vs zipped release (defer to first user release)
- Telemetry retention policy (per session / rolling window / configurable)
- Whether to ship a sample SimHub overlay theme separately

Phase-specific design questions are tracked as `[DECISIONS]` issues against
their milestones.
