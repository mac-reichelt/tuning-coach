# overlay

SimHub overlay for `tuning-coach`. Vanilla HTML/CSS/JS — no build step.

The overlay connects to the tuning-coach sidecar over a localhost WebSocket
and renders a quiet HUD that slides in whenever the coach has a tuning
recommendation.

## Install

### 1 — Download the overlay bundle

Go to the [latest release](https://github.com/mac-reichelt/tuning-coach/releases/latest)
and download **`tuning-coach-overlay.zip`**.

### 2 — Import into SimHub

1. Open SimHub.
2. Navigate to **Overlays** in the left-hand menu.
3. Click **Import overlay** and select the downloaded zip file.
4. SimHub will extract the overlay and make it available in the list.
5. Enable the overlay with the toggle next to **tuning-coach**.

### 3 — Configure the WebSocket URL

The overlay reads its sidecar address from `config.json` inside the extracted
overlay folder. The default value is:

```json
{
  "wsUrl": "ws://127.0.0.1:7778/ws"
}
```

If you changed `ws_listen_port` in the sidecar config, edit `config.json` to
match. The file is located at:

```
%APPDATA%\SimHub\DashTemplates\tuning-coach\config.json
```

(Typically `C:\Users\<YourName>\AppData\Roaming\SimHub\DashTemplates\tuning-coach\config.json`)

Reload the overlay inside SimHub after saving the file.

### 4 — Start the sidecar

Download and run the sidecar binary from the same release page
(`tuning-coach-sidecar.exe` on Windows, `tuning-coach-sidecar` on Linux):

```powershell
# Windows — run from the folder containing the binary
.\tuning-coach-sidecar.exe
```

The sidecar listens for Forza UDP telemetry on port `7777` and exposes the
WebSocket on port `7778` by default.

### 5 — Configure Forza data out

In Forza Motorsport (2023):

1. **Settings → HUD and Gameplay → Data Out**
2. Set **Data Out IP Address** to `127.0.0.1`
3. Set **Data Out Port** to `7777`
4. Enable **Data Out**

### 6 — Launch and verify

1. Start a session in Forza Motorsport.
2. The status bar at the top of the overlay turns **green** when the sidecar
   connection is established.
3. Drive a lap — the sidecar analyses your telemetry and emits recommendations
   when it detects a tuning opportunity. The recommendation panel slides in
   from the right automatically and dismisses after 15 seconds (or tap
   **Dismiss** to hide it immediately).

## Overlay layout

| Element | Description |
|---------|-------------|
| Status bar | Connection state and lap validity (top of screen) |
| Recommendation panel | Category, title, suggested values — slides in from the right |

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| Status dot stays red | Sidecar not running | Start `tuning-coach-sidecar` |
| Status dot stays red | Wrong WS URL in `config.json` | Check port matches sidecar `ws_listen_port` |
| No telemetry / no recommendations | Forza data-out not configured | Follow step 5 above |
| Overlay not visible in SimHub | Overlay not enabled | Toggle on in **Overlays** list |

## Development

The overlay has no build step. Edit the files directly and reload the overlay
in SimHub to see changes.

```
overlay/
├── config.json      ← edit WS URL here
├── index.html       ← overlay entry point
├── manifest.json    ← SimHub metadata
├── overlay.css      ← styles
└── overlay.js       ← WebSocket client + rendering
```
