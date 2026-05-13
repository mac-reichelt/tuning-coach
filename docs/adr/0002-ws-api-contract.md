---
title: WebSocket API contract
date: 2023-11-10
status: accepted
---

# ADR-0002: WebSocket API contract

## Context

The sidecar needs to expose a real-time API for overlays and external tools. SimHub overlays require a WebSocket endpoint for live telemetry and recommendations. The API must be stable, versioned, and easy to consume from vanilla JS.

The `axum` + `tokio::sync::broadcast` stack already chosen for the sidecar.

## Decision

We will expose a single WebSocket endpoint at `ws://127.0.0.1:<port>/ws`
(default port `7778`, configurable). The server binds to `127.0.0.1` by
default and only binds to `0.0.0.0` when an explicit opt-in config flag is
set, in which case it logs a warning at startup. There is no authentication
at v1 — the API surface is local-only.

### Endpoint summary

| Property            | Value                                                   |
|---------------------|---------------------------------------------------------|
| Scheme              | `ws://` (no TLS — local loopback)                       |
| Default bind        | `127.0.0.1:7778`                                        |
| Bind override       | config key `ws_listen_port` (env: `TUNING_COACH_WS_LISTEN_PORT`) |
| Path                | `/ws`                                                   |
| Subprotocol         | `tuning-coach.v1` (echoed when requested)               |
| Frame type          | Text frames only; binary frames rejected                |
| Encoding            | UTF-8 JSON, one envelope per frame                      |
| Max frame size      | 64 KiB (server-enforced; closes with 1009 if exceeded)  |
| Versioning          | `schema_version` field in hello frame                   |
| API docs            | [reference/api.md](../reference/api.md)                 |

### Versioning

- `schema_version` is an integer monotonically increasing per breaking change.
- The subprotocol string (`tuning-coach.v1`) carries the *major* version.
  Clients may connect without subprotocol negotiation, but when a client sends
  `Sec-WebSocket-Protocol: tuning-coach.v1`, the server echoes that value in
  the upgrade response.
- Once upgraded, if the client's first non-`hello` frame carries a different
  `schema_version`, the server closes with code **`4001`** and reason
  `"schema_version mismatch: server=N client=M"`.

...

### Overlay serving

The sidecar serves the overlay UI (HTML, JS, CSS) directly on the same HTTP port as the WebSocket endpoint (`127.0.0.1:7778`). SimHub overlays point to `http://127.0.0.1:7778/` and do not require a separate static file server or manifest.

### Notes

- Loopback bind is the default; `0.0.0.0` requires explicit opt-in and logs
  a warning. Most users will never touch this.
- The default port `7778` is arbitrary; configurable. If it conflicts with
  another tool, the user changes it in config.
- Tire-temp unit conversion happens at the WS boundary, not in the parser.
  Parser stays faithful to the source spec; presentation does the conversion.
