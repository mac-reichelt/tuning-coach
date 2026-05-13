# Getting Started

Welcome to Tuning Coach! This guide walks you through building and running the
sidecar and enabling the SimHub overlay.

## Prerequisites
- [Rust](https://rustup.rs/) 1.80 or newer
- SimHub 9.0+
- Forza Motorsport (PC)

## Build and Run the Sidecar

```bash
cargo build --release
./target/release/tuning-coach-sidecar
```

The sidecar listens on:
- UDP telemetry: `127.0.0.1:7777`
- HTTP + WebSocket overlay/API: `127.0.0.1:7778`

## Install the SimHub Overlay

Copy `overlay/tuning-coach.djson` and `overlay/tuning-coach.djson.metadata` into:

```text
%ProgramFiles(x86)%\SimHub\DashTemplates\tuning-coach\
```

PowerShell (from repo root):

```powershell
$dst = Join-Path ${env:ProgramFiles(x86)} 'SimHub\DashTemplates\tuning-coach'; New-Item -ItemType Directory -Force -Path $dst | Out-Null; Copy-Item .\overlay\tuning-coach.djson, .\overlay\tuning-coach.djson.metadata -Destination $dst -Force
```

Then in SimHub:
1. Open **Overlays**.
2. Enable **Tuning Coach**.
3. Position/resize as desired.

The SimHub overlay points to `http://127.0.0.1:7778/`, so no external static
file server is required.

## Next Steps
- [Configuration](configuration.md)
