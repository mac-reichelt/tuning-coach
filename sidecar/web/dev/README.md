# overlay/dev — tuning-coach diagnostic overlay

A developer/QA tool that connects to the running sidecar over WebSocket and
surfaces every event in a readable, interactive view. Use this to validate
Phase 1 (UDP ingest) and Phase 2 (lap validity, pit detection, hotkey
overrides) without reaching for `wscat` or a SQLite browser.

## What it shows

| Panel | Description |
|-------|-------------|
| **Connection** | `connecting / connected / disconnected`, last-message timestamp, WS URL, sidecar schema version |
| **Telemetry Feed** | Rolling 1-second packet rate (Hz), packet variant (`Sled` / `Dash` / `FM2023Dash`), current lap number, speed |
| **Lap State** | Current lap number, validity (`valid / dirty(reason) / pit / reset / out_lap`), dirty reason |
| **Hotkey REST Tester** | Buttons that POST to each `/api/v1/hotkeys/*` endpoint; response shown inline |
| **Event Log** | Scrolling tail of every WebSocket frame — click any row to expand the full JSON payload |

## Prerequisites

- The sidecar must be running (default: `ws://127.0.0.1:7778/ws`).
- Forza Motorsport (or any compatible emitter) must be streaming UDP telemetry
  to the sidecar (default: port `7777`).

## Loading in a browser

Open `overlay/dev/index.html` directly from the file system:

```
# macOS / Linux
open overlay/dev/index.html

# Windows
start overlay/dev/index.html
```

Or serve it with any static file server — the page makes no server-side
requests beyond the local sidecar WebSocket and REST endpoints.

## Changing the sidecar URL

The WS host and port are read from URL query parameters:

| Parameter | Default     | Example                                    |
|-----------|-------------|--------------------------------------------|
| `host`    | `127.0.0.1` | `?host=192.168.1.10`                       |
| `port`    | `7778`      | `?port=7778`                               |

Example — connect to a non-default port:

```
file:///path/to/overlay/dev/index.html?port=38920
```

## Loading in SimHub

1. Copy or symlink the `overlay/dev/` directory into your SimHub
   `DashTemplates` folder.
2. Open SimHub → Dash Studio → add a new overlay using the
   `tuning-coach-dev` template.
3. Make sure the sidecar is running before you start the game.

The overlay uses only the loopback WebSocket and REST endpoints — no internet
access required. SimHub's embedded CEF browser supports all ES2020+ features
used here.

## Filtering the event log

- **Hide telemetry** checkbox (checked by default) — suppresses the noisy
  10 Hz `telemetry` frames so you can focus on lap events.
- **⏸ Pause** — freezes new entries without dropping them.
- **Clear** — empties the log.
- **Auto-scroll** checkbox — keeps the log pinned to the latest entry.
- Click any log row to expand/collapse the full JSON payload.

## Hotkey tester

Each button sends a bare `POST` to the corresponding endpoint. This mirrors
what SimHub Global Hotkeys do, so you can verify the full round-trip (REST
call → sidecar state change → WebSocket event) without configuring SimHub
hotkeys.

| Button | Endpoint |
|--------|----------|
| Mark Lap Dirty | `POST /api/v1/hotkeys/mark-lap-dirty` |
| Mark Lap Clean | `POST /api/v1/hotkeys/mark-lap-clean` |
| Force Pit Start | `POST /api/v1/hotkeys/force-pit-start` |
| Force Pit End | `POST /api/v1/hotkeys/force-pit-end` |
| Force Session Boundary | `POST /api/v1/hotkeys/force-session-boundary` |

The HTTP response (status code + JSON body) is shown below the buttons after
each click.

## Troubleshooting

| Symptom | Likely cause |
|---------|-------------|
| Status stays **Disconnected** | Sidecar not running, or wrong port — check the URL shown in the Connection panel |
| Packet rate shows **0 Hz** | Forza not emitting UDP, or wrong UDP port on the sidecar |
| Lap state stuck on **unknown** | No active session — start a race or free-roam in Forza |
| Hotkey buttons return **503** | No active session in the sidecar |
| Hotkey buttons return **CORS error** | Browser security policy — use `file://` or a local server on the same origin |
