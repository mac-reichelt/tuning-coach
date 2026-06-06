# tuning-coach overlay

SimHub overlay for `tuning-coach`. The sidecar now serves the overlay UI over
HTTP and WebSocket on the same port (`127.0.0.1:7778` by default), so users run
only `tuning-coach-sidecar`.

## Requirements

| Component | Minimum version |
|-----------|-----------------|
| SimHub | 9.0 |
| tuning-coach sidecar | 0.1.0 |
| Forza Motorsport (PC) | any |

## Install

### 1) Run the sidecar

Download the latest `tuning-coach-sidecar` binary from the
[Releases page](https://github.com/mac-reichelt/tuning-coach/releases), then run
it. Keep it running while you use the overlay.

### 2) Copy SimHub overlay files

Copy these files into your SimHub DashTemplates folder:

- `simhub/tuning-coach.djson`
- `simhub/tuning-coach.djson.metadata`

Default destination:

```text
%ProgramFiles(x86)%\SimHub\DashTemplates\tuning-coach\
```

PowerShell one-liner (from repo root):

```powershell
$dst = Join-Path ${env:ProgramFiles(x86)} 'SimHub\DashTemplates\tuning-coach'; New-Item -ItemType Directory -Force -Path $dst | Out-Null; Copy-Item .\simhub\tuning-coach.djson, .\simhub\tuning-coach.djson.metadata -Destination $dst -Force
```

### 3) Enable overlay in SimHub

1. Open SimHub → **Overlays**.
2. Enable **Tuning Coach**.
3. Position and resize it.

The `.djson` points SimHub to `http://127.0.0.1:7778/`, which serves the HTML,
JS modules, and CSS directly from the sidecar.

## Customizing the overlay

The overlay UI in `sidecar/web/` is a **reference example** — you can edit or
fork the HTML, CSS, and JS to suit your own layout and style.

- **Debug builds** (`cargo run`) read files from `sidecar/web/` on disk, so
  changes to `index.html`, `src/`, or `styles/` show up on the next browser
  reload without rebuilding the sidecar.
- **Release builds** (`cargo build --release`) embed the allowlisted files at
  compile time via `rust_embed`; changes require a rebuild.

## Development

You can override WS target for testing with a query parameter:

```text
http://127.0.0.1:7778/?ws=ws://127.0.0.1:39000/ws
```

For telemetry injection endpoints, use the same sidecar HTTP port:

```sh
curl -s -X POST http://127.0.0.1:7778/test/telemetry \
  -H 'Content-Type: application/json' \
  -d '{"data":{"speed_kph":120,"gear":3}}'
```
