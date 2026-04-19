# ADR 0002: Sidecar WebSocket API contract

- Status: proposed
- Date: 2025-01-27
- Deciders: @mac-reichelt

## Context

The sidecar (Rust) ingests Forza UDP telemetry at ~60 Hz, runs a heuristics
engine, and needs to push two streams of data to the SimHub overlay (HTML/JS):

1. **Live telemetry** — a downsampled view of the parsed Forza Dash packet,
   used by the HUD to render speed/RPM/gear/etc. and a lap-status indicator.
2. **Recommendations** — discrete events from the race-engineer heuristics
   pipeline (see `.github/agents/race-engineer.agent.md`). These are bursty,
   not continuous.

Phase 1 of `docs/PLAN.md` blocks on this contract being locked before
implementation (issue #9). The contract must also be open enough to support
additional consumers later — Stream Deck plugins, custom dashboards, a future
native viewer — without a rewrite.

Constraints:

- **Local-only** at v1: sidecar runs on the same Windows box as Forza and
  SimHub. No remote scenario yet.
- **Browser consumer**: the SimHub overlay is HTML/JS running in SimHub's
  embedded webview. That rules out non-HTTP transports unless we ship a
  bridge.
- **Bursty + streaming mix**: telemetry is high-rate streaming; recommendations
  are low-rate events; both must arrive on a single connection so the overlay
  stays in sync with the session.
- **Bidirectional**: the overlay needs to talk back — at minimum, to set its
  preferred telemetry rate and to subscribe selectively. Future consumers will
  want to request snapshots or replay state.
- **Slow consumer hostile**: a janky overlay must never back-pressure the UDP
  ingest path. Telemetry can be dropped; the producer cannot block.

We considered Server-Sent Events, a Unix/named-pipe transport, and gRPC. WS
won (see Alternatives) because it is the only option that is bidirectional,
universally supported by browser/webview clients with zero shim, and matches
the `axum` + `tokio::sync::broadcast` stack already chosen for the sidecar.

## Decision

We will expose a single WebSocket endpoint at `ws://127.0.0.1:<port>/ws`
(default port `38920`, configurable). The server binds to `127.0.0.1` by
default and only binds to `0.0.0.0` when an explicit opt-in config flag is
set, in which case it logs a warning at startup. There is no authentication
at v1 — the API surface is local-only.

All frames are **text frames** carrying a single JSON object that conforms to
a fixed envelope. The envelope, the set of event types, the heartbeat policy,
the slow-consumer policy, and the version-negotiation rules are all specified
below and frozen at `schema_version: 1`. Incompatible changes bump
`schema_version` and require a new ADR.

### Transport

| Property            | Value                                                   |
|---------------------|---------------------------------------------------------|
| Scheme              | `ws://` (no TLS — local loopback)                       |
| Default bind        | `127.0.0.1:38920`                                       |
| Bind override       | config key `ws.bind` (env: `TUNING_COACH_WS_BIND`)      |
| Path                | `/ws`                                                   |
| Subprotocol         | `tuning-coach.v1` (sent in `Sec-WebSocket-Protocol`)    |
| Frame type          | Text frames only; binary frames rejected                |
| Encoding            | UTF-8 JSON, one envelope per frame                      |
| Max frame size      | 64 KiB (server-enforced; closes with 1009 if exceeded)  |
| Compression         | `permessage-deflate` advertised, accepted if offered    |

CBOR is **deferred**. If JSON serialization shows up in profiling as a hot
path, a follow-up ADR can introduce a `tuning-coach.v1+cbor` subprotocol; the
envelope stays identical.

### Envelope

Every message — server→client and client→server — is a JSON object with these
fields:

```json
{
  "type": "telemetry",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": { /* type-specific */ }
}
```

| Field            | JSON type | Required | Notes                                                              |
|------------------|-----------|----------|--------------------------------------------------------------------|
| `type`           | string    | yes      | One of the event types below. Unknown types are rejected.          |
| `schema_version` | integer   | yes      | Currently `1`. Mismatched clients are closed with code `4001`.     |
| `t_ms`           | integer   | yes      | Unix epoch ms (server wall clock for server-sent; client for sent).|
| `data`           | object    | yes      | Type-specific payload. May be empty `{}` but must be present.      |

Forward-compatibility rule: **clients must ignore unknown fields** inside
`data`. Adding fields to a `data` payload is a non-breaking change. Removing
or retyping a field is breaking and bumps `schema_version`.

### Server → client event types

| `type`             | Cardinality                  | Purpose                                     |
|--------------------|------------------------------|---------------------------------------------|
| `hello`            | exactly 1, first frame       | Handshake; advertises versions and limits   |
| `session_started`  | per session                  | Forza `IsRaceOn` went 0→1                   |
| `session_ended`    | per session                  | `IsRaceOn` went 1→0 or pause threshold hit  |
| `telemetry`        | streaming (default 10 Hz)    | Downsampled, curated telemetry snapshot     |
| `lap_completed`    | per lap boundary             | Final lap time + validity classification    |
| `recommendation`   | bursty, low rate             | Race-engineer output                        |
| `pong`             | reply to client `ping`       | Echoes client `t_ms`                        |
| `error`            | as-needed                    | Soft error; connection stays open           |

### Client → server event types

| `type`              | Purpose                                                             |
|---------------------|---------------------------------------------------------------------|
| `ping`              | Liveness probe; server replies with `pong`                          |
| `subscribe`         | Filter the set of event types this client wants                     |
| `set_rate`          | Change telemetry downsample rate for *this client*                  |
| `request_snapshot`  | Ask the server to immediately emit the current telemetry frame      |

Any unknown `type`, missing required field, or wrong `schema_version` causes
the server to send an `error` envelope and — if the offence is the envelope
itself or the version — close the connection (see Error model).

### Heartbeat

- Server sends a WebSocket-level **PING** control frame every **30 s**.
- If no **PONG** (or any client frame) arrives within **60 s** of the last
  client activity, the server closes with code `1011` (server-side timeout)
  and reason `"idle timeout"`.
- Application-level `ping`/`pong` envelopes are also supported for clients
  whose WebSocket library hides control frames; either path resets the idle
  timer.

### Slow-consumer policy

Each client gets its own bounded `tokio::sync::broadcast` receiver. Capacity:
**256 frames** (≈ 25 s @ 10 Hz). When a client lags:

- Telemetry frames: **drop oldest** silently (count is exposed via metrics,
  not the WS). The producer never blocks — the broadcast `send` call
  cannot await.
- `recommendation`, `lap_completed`, `session_*`: **never dropped**. If the
  per-client queue is full of stale telemetry, the server purges telemetry
  from that client's lane to make room. If non-telemetry frames still cannot
  be delivered within 5 s, the server closes the client with code `1008`
  (policy violation) and reason `"slow consumer"`.

Implementation hint (non-normative): a single `broadcast::Sender<Frame>` for
telemetry and a per-client `mpsc::Sender<Frame>` for events solves this
cleanly without coupling the two queues.

### Versioning

- `schema_version` is an integer monotonically increasing per breaking change.
- The subprotocol string (`tuning-coach.v1`) carries the *major* version. The
  server only speaks `tuning-coach.v1`; if the upgrade completes without
  negotiating that subprotocol (e.g. the client requested `tuning-coach.v2`),
  the server immediately closes with code **`4002`** and reason
  `"unsupported subprotocol"`. The client must reconnect with the correct
  subprotocol. Note: `4002` is an application close frame sent *after* the
  HTTP 101 upgrade — it is not an HTTP-level rejection.
- Once upgraded, if the client's first non-`hello` frame carries a different
  `schema_version`, the server closes with code **`4001`** and reason
  `"schema_version mismatch: server=N client=M"`.
- Additive changes (new event types, new fields inside `data`) do **not**
  bump `schema_version`. Clients tolerate them by ignoring unknowns.

### Multi-client fan-out

Any number of clients may connect simultaneously. Each gets:

- Its own `hello` (with a unique `client_id`).
- Its own subscription filter and downsample rate.
- Its own slow-consumer accounting.

Recommendations and lap events are fanned out to every subscribed client.
Telemetry is generated once per UDP packet, downsampled per-client, and
delivered independently.

### Authentication

None at v1. The server binds to loopback by default; the `0.0.0.0` opt-in
emits a `WARN level=audit msg="ws bound to non-loopback; no auth"` log line
on every accepted upgrade until a future ADR introduces auth.

### Observability

Mandatory per architect cross-cutting standards:

- Structured log on connect/disconnect (`client_id`, `peer_addr`, `subprotocol`,
  `close_code`, `reason`).
- Metrics: `ws_clients_connected` (gauge), `ws_frames_sent_total{type}`
  (counter), `ws_frames_dropped_total{type,reason}` (counter),
  `ws_send_lag_seconds` (histogram per-client p50/p95).
- Never log envelope `data` payloads at `INFO` (they're large and high-rate);
  `DEBUG` only, sampled.

## Event payload schemas

### `hello` (server → client)

```json
{
  "type": "hello",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "sidecar_version": "0.1.0",
    "client_id": "01HQ7K8YV3...",
    "server_time_ms": 1738012345678,
    "telemetry_default_hz": 10,
    "telemetry_max_hz": 60,
    "supported_event_types": [
      "telemetry", "recommendation", "lap_completed",
      "session_started", "session_ended", "pong", "error"
    ]
  }
}
```

> **Note:** `"hello"` is intentionally absent from `supported_event_types`.
> It is sent exactly once at connect-time before any other frame; it is not a
> subscribeable event type and clients always receive it unconditionally.

### `telemetry` (server → client)

Curated subset of the Forza Dash packet. Units are SI / display-friendly;
**tire temperatures are converted from °F to °C at this boundary** (Forza
emits °F per the source spec — see `.github/agents/telemetry-expert.agent.md`).
Steering is normalized to `[-1.0, 1.0]`. Throttle/brake/clutch/handbrake are
normalized to `[0.0, 1.0]`. Speed is in km/h for HUD use.

```json
{
  "type": "telemetry",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "is_race_on": true,
    "session_t_ms": 142318,
    "speed_kph": 187.4,
    "rpm": 8120.0,
    "rpm_max": 9200.0,
    "gear": 4,
    "throttle": 0.92,
    "brake": 0.0,
    "clutch": 0.0,
    "handbrake": 0.0,
    "steer": -0.18,
    "drivetrain": "AWD",
    "lap": {
      "number": 3,
      "current_s": 42.318,
      "last_s": 89.114,
      "best_s": 88.902,
      "position": 4,
      "distance_m": 4821.6
    },
    "tire_temp_c": { "fl": 88.3, "fr": 92.1, "rl": 79.5, "rr": 81.0 },
    "tire_slip_ratio": { "fl": 0.02, "fr": 0.03, "rl": 0.05, "rr": 0.06 },
    "tire_slip_angle_rad": { "fl": 0.04, "fr": 0.05, "rl": 0.03, "rr": 0.04 },
    "suspension_travel_norm": { "fl": 0.41, "fr": 0.43, "rl": 0.55, "rr": 0.57 },
    "fuel_frac": 0.62,
    "boost_bar": 0.8,
    "accel_g": { "x": 0.12, "y": -0.04, "z": 1.21 },
    "lap_status": "valid"
  }
}
```

`lap_status` is one of `"valid" | "dirty" | "pit" | "reset" | "out_lap"`.
Tire wear is **not** in the Dash packet and is therefore omitted; if a future
parser exposes it (e.g. a different game) it will be added under
`tire_wear_frac`.

Rationale for the field selection: every field is needed by either the
heuristics engine (per `.github/agents/race-engineer.agent.md`) or a typical
HUD. Raw vectors that aren't directly displayed (velocity, angular velocity,
position) are intentionally excluded from `telemetry`; they remain available
through `request_snapshot` if a debugging consumer needs them in future
(via an additional `data.raw` block, gated by an opt-in subscribe filter).

### `lap_completed` (server → client)

```json
{
  "type": "lap_completed",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "lap_number": 3,
    "lap_time_s": 88.902,
    "is_personal_best": true,
    "validity": "valid",
    "invalid_reasons": []
  }
}
```

`validity ∈ {"valid","dirty","pit","reset"}`. `invalid_reasons` is an array
of short tags (`"off_track"`, `"contact"`, `"rewind"`, `"pit_in"`).

### `session_started` / `session_ended` (server → client)

```json
{
  "type": "session_started",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "session_id": "01HQ7K8YV3...",
    "car_ordinal": 3456,
    "car_class": "S",
    "car_pi": 750,
    "drivetrain": "AWD"
  }
}
```

```json
{
  "type": "session_ended",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "session_id": "01HQ7K8YV3...",
    "duration_s": 1842.3,
    "lap_count": 12
  }
}
```

### `recommendation` (server → client)

Mirrors the race-engineer recommendation format one-to-one. Each field maps
to a line in the engineer's template (`Detected / Likely cause / Recommended
adjustment / Expected outcome / Confidence / Caveats`).

```json
{
  "type": "recommendation",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "id": "01HQ7K8YV3...",
    "session_id": "01HQ7K8YV3...",
    "lap_number": 3,
    "category": "springs",
    "title": "Front bottoming out",
    "detected": "Front suspension >95% travel on 3 of 4 corners (T1, T3, T7).",
    "cause": "Insufficient front spring rate / ride height for downforce + load.",
    "adjustment": {
      "summary": "Front spring rate 85 → 92 N/mm",
      "parameter": "spring_rate_front",
      "from": 85.0,
      "to": 92.0,
      "unit": "N/mm",
      "step": 1.0
    },
    "expected_outcome": "Eliminates bottoming on T1/T3; slight loss of mechanical grip mid-corner.",
    "confidence": "high",
    "caveats": [
      "Assumes smooth driving style",
      "Re-check after 3 clean laps",
      "If Race Springs not installed, raise ride height +2mm instead"
    ],
    "alternatives": [
      { "summary": "Ride height F +2mm", "parameter": "ride_height_front", "from": 110, "to": 112, "unit": "mm" }
    ],
    "driving_style_assumed": "smooth",
    "locked_fallback_used": false
  }
}
```

`category ∈ {"tires","gearing","alignment","anti_roll","springs","damping","aero","brakes","differential"}`
matching the tuning surface in `docs/PLAN.md`. `confidence ∈ {"high","medium","low"}`
matching the engineer agent's confidence rules.

### `error` (server → client)

```json
{
  "type": "error",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "code": "bad_request",
    "message": "set_rate: hz must be in [1, 60]",
    "ref": null
  }
}
```

`code` is a fixed enum: `bad_request | unknown_type | schema_mismatch | rate_limited | internal`.
`ref` echoes the offending client envelope's `t_ms` if known, for client-side
correlation.

### `ping` / `pong`

```json
{ "type": "ping", "schema_version": 1, "t_ms": 1738012345678, "data": {} }
```

Server reply:

```json
{ "type": "pong", "schema_version": 1, "t_ms": 1738012345701, "data": { "echo_t_ms": 1738012345678 } }
```

### `subscribe` (client → server)

```json
{
  "type": "subscribe",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "events": ["telemetry", "recommendation", "lap_completed",
               "session_started", "session_ended"]
  }
}
```

`events` is a complete list, not a delta. An empty list disables all
server-pushed events except `error`, `pong`, and `hello`. Default on connect
is *all event types*.

### `set_rate` (client → server)

```json
{ "type": "set_rate", "schema_version": 1, "t_ms": 1738012345678, "data": { "hz": 10 } }
```

Valid range: `1..=60`. Out-of-range values produce an `error` with
`code: "bad_request"`; the previous rate is retained.

### `request_snapshot` (client → server)

```json
{ "type": "request_snapshot", "schema_version": 1, "t_ms": 1738012345678, "data": {} }
```

Server responds within 100 ms with a single `telemetry` frame containing the
most recent parsed packet, regardless of the client's downsample rate.

### Close codes

| Code   | Meaning                              | Sender |
|--------|--------------------------------------|--------|
| `1000` | Normal closure                       | either |
| `1008` | Policy violation (slow consumer)     | server |
| `1009` | Frame too large                      | server |
| `1011` | Server error / idle timeout          | server |
| `4001` | `schema_version` mismatch            | server |
| `4002` | Unsupported subprotocol              | server |

`4000`-range codes are application-defined per RFC 6455.

## Consequences

### Positive

- **One contract, many consumers.** The overlay is the v1 client, but Stream
  Deck plugins, custom dashboards, and a future native viewer can all attach
  with zero protocol changes.
- **Producer is unblockable.** UDP ingest cannot be back-pressured by a
  hung overlay; slow consumers degrade themselves, not the system.
- **Per-client downsampling.** A debug dashboard can run at 60 Hz while the
  HUD stays at 10 Hz, on the same sidecar instance.
- **Clear breaking-change protocol.** `schema_version` + close code 4001 +
  subprotocol major version give us three independent ways to reject a stale
  client cleanly.
- **Boring stack.** `axum` WS + `tokio::sync::broadcast` + JSON. All
  well-trodden.

### Negative

- **JSON cost.** Serializing a 25-field telemetry struct 10 times/sec is
  cheap, but 60 Hz × N clients eventually shows up. Mitigation: CBOR
  subprotocol is left open as a future ADR; envelope stays the same.
- **Two queues per client.** The split between broadcast (telemetry) and
  per-client mpsc (events) is more code than a single channel. Worth it for
  the "events never dropped" guarantee.
- **No auth.** A malicious local process can connect. Acceptable for v1
  because anything that runs locally already has more dangerous attack
  surface (UDP ingest, SQLite file).
- **Field selection bias.** The curated `telemetry` payload reflects today's
  HUD + heuristics needs. Adding fields later is non-breaking, but removing
  or renaming is breaking — pick carefully.

### Neutral

- Loopback bind is the default; `0.0.0.0` requires explicit opt-in and logs
  a warning. Most users will never touch this.
- The default port `38920` is arbitrary; configurable. If it conflicts with
  another tool, the user changes it in config.
- Tire-temp unit conversion happens at the WS boundary, not in the parser.
  Parser stays faithful to the source spec; presentation does the conversion.

## Alternatives Considered

### Server-Sent Events (SSE)

- Pros: Simpler than WS; pure HTTP; auto-reconnects in browsers.
- Cons: Strictly server→client. Client→server (subscribe filters, set_rate,
  snapshot requests) would need a separate `POST /control` endpoint, doubling
  the surface area and de-syncing the two channels (a control change has no
  ordering guarantee with the event stream).
- Why not chosen: bidirectionality matters, and a split control channel
  bleeds complexity into every consumer.

### Native pipe / Unix domain socket / Windows named pipe

- Pros: Lowest latency; no HTTP framing overhead; trivial to secure.
- Cons: SimHub's HTML overlay runs in an embedded webview that speaks WS/HTTP
  natively. Adding a pipe bridge means shipping a JS shim or a native
  helper — friction for the primary consumer to save microseconds.
- Why not chosen: WS over loopback is already sub-millisecond; the pipe wins
  nothing the overlay can use.

### gRPC (with grpc-web for the browser)

- Pros: Strong typing via protobuf; codegen for clients; streaming RPCs.
- Cons: grpc-web requires a proxy (Envoy or in-process); protobuf adds a
  build step to a vanilla-JS overlay; the schema is overkill for ~10 event
  types.
- Why not chosen: the contract is small enough that JSON + a hand-written
  schema doc is lower-friction than protobuf + codegen + a grpc-web proxy.

### Raw UDP rebroadcast

- Pros: Trivial in the sidecar; overlay just listens.
- Cons: Browsers can't open UDP sockets. Non-starter for the HTML overlay.
- Why not chosen: see above.

## References

- Issue #9 — `feat(sidecar): WebSocket API for overlay (telemetry stream + recommendations)`
- `docs/PLAN.md` — Phase 1 sidecar foundation
- `.github/agents/telemetry-expert.agent.md` — Forza Dash packet schema (informs telemetry payload)
- `.github/agents/race-engineer.agent.md` — recommendation format (informs `recommendation` payload)
- `docs/reference/api.md` — user-facing API reference (companion to this ADR)
- RFC 6455 — The WebSocket Protocol (close codes, subprotocols)
- `axum::extract::ws` — server implementation primitives
- `tokio::sync::broadcast` — fan-out channel with bounded capacity + lag semantics
