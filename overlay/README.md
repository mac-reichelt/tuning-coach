# tuning-coach overlay

SimHub overlay for `tuning-coach`. Vanilla HTML/CSS/JS — no build step.

The overlay connects to the tuning-coach sidecar over a localhost WebSocket
and renders:

- **Telemetry HUD** — speed, gear, RPM, throttle/brake bars, steering indicator, lap clock (hidden by default; click the left-edge tab to toggle).
- **Lap-status badge** — `valid` / `dirty` / `pit` / `reset` / `out_lap`.
- **Recommendation slot** — placeholder card that slides in from the right; will surface real tuning advice once Phase 7 heuristics land.
- **Connection-status bar** — shows "Reconnecting…" or "Sidecar offline" whenever the sidecar is unreachable; automatically disappears when connected.

## Requirements

| Component | Minimum version |
|-----------|----------------|
| SimHub    | 9.0            |
| tuning-coach sidecar | 0.1.0 |
| Forza Motorsport (PC) | any |

The sidecar must be running **before** you load the overlay. It binds to
`ws://127.0.0.1:38920/ws` by default; the overlay will keep retrying until
it connects.

## Install

### 1 — Install and run the sidecar

Download the latest `tuning-coach-sidecar` binary from the
[Releases page](https://github.com/mac-reichelt/tuning-coach/releases) and
follow the [sidecar README](../sidecar/README.md).

### 2 — Copy the overlay into SimHub

Copy (or symlink) this entire `overlay/` directory into your SimHub
`DashTemplates` folder. The default location is:

```
%USERPROFILE%\Documents\SimHub\DashTemplates\tuning-coach\
```

The final directory tree should look like:

```
DashTemplates\
└── tuning-coach\
    ├── manifest.json
    ├── index.html
    ├── src\
    │   ├── ws-client.js
    │   ├── telemetry-hud.js
    │   ├── lap-status.js
    │   └── recommendation-slot.js
    └── styles\
        └── overlay.css
```

### 3 — Enable in SimHub

1. Open SimHub → **Overlays** tab.
2. Click **Add overlay** and select `tuning-coach` from the list.
3. Position and resize the overlay on your screen as desired.
4. Start a Forza Motorsport session — the overlay will connect automatically.

## Usage

| Element | Behaviour |
|---------|-----------|
| **Connection bar** (top-centre) | Hidden when connected. Shows "Reconnecting…" in amber or "Sidecar offline" in red. |
| **Lap-status badge** (top-right) | Colour-coded: green = valid, amber = dirty, purple = pit, blue = reset, grey = out lap. |
| **HUD toggle tab** (left edge, bottom) | Click to show/hide the telemetry HUD. |
| **Telemetry HUD** (bottom-left) | Gear, speed, RPM bar, throttle/brake bars, steering dot, lap time + delta. |
| **Recommendation slot** (right side) | Slides in when the coach has advice. Dismiss with ✕; snooze hides for the session; History shows past advice. |

## Speed unit

The HUD defaults to **km/h**. The sidecar will expose a `user_preferences`
field in the `hello` message in a future release; the overlay is wired to
pick this up automatically.

## Architecture

```
Forza (UDP) ──► sidecar (Rust)
                    │  ws://127.0.0.1:38920/ws
                    ▼
              overlay (this directory)
                    │
              ┌─────┼─────────────────┐
              │     │                 │
         ws-client  telemetry-hud  lap-status
                    │
              recommendation-slot
```

See [ADR-0002](../docs/adr/0002-ws-api-contract.md) for the full WS API
contract and [docs/reference/api.md](../docs/reference/api.md) for a
human-readable summary.

## File layout

```
overlay/
├── README.md
├── CHANGELOG.md
├── manifest.json          # SimHub overlay manifest
├── index.html             # Entry point
├── src/
│   ├── ws-client.js       # WS client with reconnect-with-backoff
│   ├── telemetry-hud.js   # Speed / gear / bars / lap clock
│   ├── lap-status.js      # Lap-status badge
│   └── recommendation-slot.js  # Placeholder recommendation panel
└── styles/
    └── overlay.css
```

## Development

No build step — open `index.html` directly in a browser and point it at a
running sidecar. The `ws-client.js` module will keep retrying until the
sidecar is available.

For a quick local test without Forza running, use the sidecar's
[test-inject endpoints](../docs/reference/api.md#test-hooks-developmentintegration-tests):

```sh
# Inject a telemetry event
curl -s -X POST http://127.0.0.1:38920/test/telemetry \
  -H 'Content-Type: application/json' \
  -d '{"data":{"speed_kph":120,"gear":3,"rpm":6500,"rpm_max":9000,"throttle":0.8,"brake":0,"steer":0.1,"lap_status":"valid","lap":{"number":2,"current_s":45.3,"best_s":88.9,"last_s":89.2}}}'

# Inject a recommendation
curl -s -X POST http://127.0.0.1:38920/test/recommendation \
  -H 'Content-Type: application/json' \
  -d '{"data":{"id":"01test","session_id":"01sess","lap_number":2,"category":"springs","title":"Front bottoming out","detected":"Front suspension >95% travel on corners.","confidence":"high","adjustment":{"summary":"Front spring rate 85 → 92 N/mm"}}}'
```

## Changelog

See [CHANGELOG.md](./CHANGELOG.md).
