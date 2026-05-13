# Tuning Coach — Live telemetry HUD + tuning recommendations for Forza Motorsport

Tuning Coach connects to Forza Motorsport (PC) and surfaces actionable tuning advice, live telemetry, and lap validity. The SimHub overlay is now served directly from the sidecar HTTP endpoint—no separate static file server or overlay bundle required.

![Overlay screenshot](docs/img/overlay-screenshot.png)

## Features
- ✅ Live telemetry HUD — speed, gear, RPM, throttle/brake, steering, lap clock
- ✅ Lap-status badge — valid/dirty/pit/reset/out lap
- ✅ Recommendation slot — tuning advice (Phase 7+)
- ✅ SimHub overlay — served directly from sidecar HTTP endpoint

## Quickstart

**Prerequisites:**
- [Rust](https://rustup.rs/) 1.80+
- SimHub 9.0+
- Forza Motorsport (PC)

**Build and run the sidecar:**

```bash
cargo build --release
./target/release/tuning-coach-sidecar
```

The sidecar listens on:
- UDP telemetry: `127.0.0.1:7777`
- HTTP + WebSocket overlay/API: `127.0.0.1:7778`

**Install the SimHub overlay:**

Copy `overlay/tuning-coach.djson` and `overlay/tuning-coach.djson.metadata` into:

```text
%ProgramFiles(x86)%\SimHub\DashTemplates\tuning-coach\
```

PowerShell one-liner (from repo root):

```powershell
$dst = Join-Path ${env:ProgramFiles(x86)} 'SimHub\DashTemplates\tuning-coach'; New-Item -ItemType Directory -Force -Path $dst | Out-Null; Copy-Item .\overlay\tuning-coach.djson, .\overlay\tuning-coach.djson.metadata -Destination $dst -Force
```

Then in SimHub:
1. Open **Overlays**.
2. Enable **Tuning Coach**.
3. Position/resize as desired.

The SimHub overlay points to `http://127.0.0.1:7778/`, so no external static file server is required.

## Documentation
- [Getting Started](docs/getting-started.md)
- [Configuration](docs/configuration.md)
- [API Reference](docs/reference/api.md)

## Status
| Feature | Status |
|---------|--------|
| Telemetry HUD | Stable |
| Lap-status badge | Stable |
| Recommendation slot | Beta (Phase 7+) |
| SimHub overlay serving | Stable |

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
MIT — see [LICENSE](LICENSE).
