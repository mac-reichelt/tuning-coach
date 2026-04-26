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
- UDP listener on configurable port, parses Forza "Dash" packet (331 bytes)
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

Roadmap reordered (2026-04-26) to optimize for hands-on user validation at every
phase, minimal blocking deps, and earliest risk burndown. See
[ADR-0003](adr/) (TBD) for rationale. Phases 1 and 2 shipped as
`sidecar-v0.1.1` / `overlay-v0.1.1`.

### Phase 1 — Sidecar foundation ✅ shipped
- Rust project scaffold, CI, basic logging
- Forza UDP packet parser ("Dash" format, full 331-byte schema)
- SQLite schema (sessions / laps / telemetry_snapshots / recommendations /
  car_setups / user_preferences / hotkey_events)
- WebSocket server + REST API (telemetry stream + hotkey webhook endpoints)
- CLI to start/stop and inspect

### Phase 2 — Lap validity + session state machine ✅ shipped
- Dirty-lap detection (off-track wheel count, contact, grip discontinuity)
- Pit-stop detection (speed in pit area, stationary state)
- Lap reset / rewind detection (position jump, lap-time anomaly)
- Manual override hotkey endpoints
- Session state machine + persistence

### Phase 3 — Overlay shell (product skeleton)
- SimHub overlay template + WebSocket client to sidecar
- Telemetry HUD primitives (speed, gear, throttle/brake bars, lap clock)
- Lap-status indicator (valid / dirty / pit / reset) — visual proof Phase 2
  detection works
- Recommendation slot (placeholder until Phase 7 heuristics land)
- Dismiss / snooze / history-toggle controls (UI only, payload TBD)
- Supersedes / closes #49 (diagnostic dev overlay)
- **Why first:** unblocks every later visible feature; first phase where the
  user can *see* the product, not just `tail -f` it.

### Phase 4 — Setup form + car/track ID
- HTML setup form (car upgrades installed, locked params, current setup
  values, preferences) → persisted via sidecar REST to `car_setups` /
  `user_preferences`
- Car/track identification via documented `CarOrdinal` / `TrackOrdinal`
  packet fields + community lookup tables
  ([forum reference](https://forums.forza.net/t/data-out-feature-in-forza-motorsport/651333))
- Per-car setup remembered between sessions
- **Why merged:** ordinals make per-car persistence trivial; no point shipping
  setup form without the key it hangs on.

### Phase 5 — OCR feasibility spike
- Bounded spike (timeboxed): Windows Desktop Duplication API capture of
  tuning menu + Tesseract OCR
- Acceptance bar: ≥70% per-field accuracy on a reference car (e.g. Civic
  Type R) tuning screen
- Output: go / no-go decision + recommendation for Phase 13 scope
- **Why early:** kills or confirms the riskiest stretch goal before we build
  features that depend on it.

### Phase 6 — In-session hotkeys + Stream Deck
- SimHub global-shortcut config doc + sample import
- Sidecar webhook endpoints for each action (mark dirty / pit / snooze /
  request feedback / reset)
- Stream Deck profile (optional)
- **Why before suspension heuristics:** small, self-contained, gives the user
  another tactile control surface immediately after the overlay lands.

### Phase 7 — Heuristics: suspension + ride height + ARB + damping
- Lap segmentation (anonymous corner indexing pre-corner-DB:
  `(distance_m_at_entry bucketed 25 m, sign(peak_lat_g))`)
- Suspension travel analysis (compression, rebound, bottoming)
- Ride height baseline
- Roll/pitch analysis → ARB and damping recommendations
- Dirty/pit/reset laps excluded from analysis
- First heuristic — pays the trailblazer cost (segmentation, engine wiring,
  locked-param fallback `from: null, needs_setup_form: true`) for the rest
  of Phase 8
- Unit tests with recorded replays
- See `~/.copilot/session-state/<id>/files/tuning-coach-phase-3-breakdown.md`
  for per-issue scope (P3-0 through P3-7).

### Phase 8 — Heuristics: remaining categories
Sliced for incremental delivery. Each slice ships independently.
- **8a — Brakes + tires:** lockup detection per axle, stopping distance,
  pressure heuristics from temp/pressure delta (or setup-form-driven if not
  exposed)
- **8b — Gearing + differential:** top-speed-on-straight vs rev limit, shift
  points, bog detection, corner-exit wheelspin per axle, lift-off rotation
- **8c — Alignment + aero:** tire load distribution at corner apex,
  understeer/oversteer split, high-speed corner stability vs straight-line
  drag

### Phase 9 — Post-session report
- Markdown + JSON output
- Per-lap summary, aggregate recommendations, delta vs prior session on same
  car/track

### Phase 10 — Driving-style auto-detect
- Classifier over throttle/brake/steering traces
- Style profiles (smooth/aggressive, early/late braker, trail braker, etc.)
- Tune recommendations adjusted per detected style

### Phase 11 — Recorded session replay
- Replay saved telemetry through pipeline
- Viewer UI for scrubbing

### Phase 12 — LLM phrasing layer (optional)
- OpenAI-compatible client (configurable base_url + api_key)
- Feature-flagged; heuristics always primary
- LLM consumes structured recommendation + driver context, returns
  style-matched explanation
- Caching + rate limiting

### Phase 13 — OCR setup capture (gated by Phase 5)
- Only proceeds if Phase 5 spike clears the accuracy bar
- Windows Desktop Duplication API screen capture
- Tesseract OCR of tuning menu screens
- Auto-populate `car_setups` (current values + locked flags); setup form
  remains as manual override
- Hotkey to trigger capture from in-game menu

### Phase 14 — Additional games
- Generic telemetry abstraction layer
- Add second game (likely iRacing or ACC)

### Future / post-MVP backlog
- **Track corner database:** auto-derive named corner index per track from
  reference lap (curvature peaks, distance markers); replaces anonymous
  `(distance, lat_g)` indexing introduced in Phase 7. Recommended slot:
  between Phase 8 and Phase 9 once heuristics expose how often "same corner"
  ambiguity matters.

## Open questions (deferrable)
- Report delivery: file on disk vs sidecar HTTP vs both
- Installer vs zipped release
- Telemetry retention policy (per session / rolling window / configurable)
- Whether to ship a sample SimHub overlay theme separately

## Next steps after plan approval
1. Create private GitHub repo `tuning-coach` under `mac-reichelt`
2. Scaffold Rust sidecar + SimHub overlay template, prove end-to-end with a
   no-op recommendation
3. Lock in SQLite schema (including car_setups + locked-param model)
4. Phase 2: lap-validity + session state machine
