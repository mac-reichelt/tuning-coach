# Sidecar WebSocket API

> **Status:** v1 contract proposed in [ADR-0002](../adr/0002-ws-api-contract.md).
> This page is a user-facing summary of the same contract; the ADR is the
> source of truth.

The sidecar exposes a single WebSocket endpoint that any client can use to
receive live telemetry and tuning recommendations. The primary consumer is
the SimHub HTML overlay; Stream Deck plugins, custom dashboards, and future
native viewers can attach to the same endpoint.

## Quick start

```js
const ws = new WebSocket("ws://127.0.0.1:38920/ws", "tuning-coach.v1");

ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  switch (msg.type) {
    case "hello":          console.log("connected", msg.data); break;
    case "telemetry":      renderHud(msg.data); break;
    case "recommendation": showRecommendation(msg.data); break;
    case "lap_completed":  updateLapBoard(msg.data); break;
    case "error":          console.warn("ws error", msg.data); break;
  }
};

// Optional: drop the rate to 5 Hz for a lower-overhead HUD
ws.onopen = () => ws.send(JSON.stringify({
  type: "set_rate", schema_version: 1, t_ms: Date.now(), data: { hz: 5 }
}));
```

## Endpoint

| Property      | Default                   | Override                            |
|---------------|---------------------------|-------------------------------------|
| URL           | `ws://127.0.0.1:38920/ws` | config `ws.bind`, env `TUNING_COACH_WS_BIND` |
| Bind address  | `127.0.0.1` (loopback)    | set `ws.bind` to `0.0.0.0` (logs a warning) |
| Subprotocol   | `tuning-coach.v1`         | optional in current implementation |
| Encoding      | UTF-8 JSON, text frames   | (CBOR deferred to a future ADR)     |
| Auth          | none                      | local-only by design                |

## Envelope

Every frame — both directions — is a JSON object with this shape:

```json
{
  "type":           "<event-type>",
  "schema_version": 1,
  "t_ms":           1738012345678,
  "data":           { /* type-specific */ }
}
```

- `type` — one of the event types listed below
- `schema_version` — currently `1`; mismatched clients are closed with code `4001`
- `t_ms` — Unix epoch milliseconds (server wall clock for server frames; client clock for client frames)
- `data` — type-specific payload; never null, may be `{}`

**Forward compatibility:** clients must ignore unknown fields inside `data`.
Adding fields is non-breaking. Removing or retyping fields is breaking and
bumps `schema_version`.

## Server → client events

| `type`             | When                            | Drop policy                           |
|--------------------|----------------------------------|---------------------------------------|
| `hello`            | First frame after upgrade        | Never dropped                         |
| `session_started`  | `IsRaceOn` 0 → 1                 | Never dropped                         |
| `session_ended`    | `IsRaceOn` 1 → 0                 | Never dropped                         |
| `telemetry`        | Streaming (default 10 Hz)        | **Oldest dropped** if client is slow  |
| `lap_completed`    | At each lap boundary             | Never dropped                         |
| `recommendation`   | When the heuristics engine emits | Never dropped                         |
| `pong`             | Reply to a client `ping`         | Never dropped                         |
| `error`            | On client protocol violations    | Never dropped                         |

### `hello`

```json
{
  "type": "hello",
  "schema_version": 1,
  "sidecar_version": "0.1.0"
}
```

It is always sent as the first frame right after the WebSocket upgrade.

### `telemetry`

Curated subset of the Forza Dash packet. **Tire temps are converted from °F
to °C at this boundary.** Steering is normalized to `[-1, 1]`; pedals are
normalized to `[0, 1]`; speed is in km/h.

```json
{
  "type": "telemetry", "schema_version": 1, "t_ms": 1738012345678,
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
      "number": 3, "current_s": 42.318, "last_s": 89.114,
      "best_s": 88.902, "position": 4, "distance_m": 4821.6
    },
    "tire_temp_c":            { "fl": 88.3, "fr": 92.1, "rl": 79.5, "rr": 81.0 },
    "tire_slip_ratio":        { "fl": 0.02, "fr": 0.03, "rl": 0.05, "rr": 0.06 },
    "tire_slip_angle_rad":    { "fl": 0.04, "fr": 0.05, "rl": 0.03, "rr": 0.04 },
    "suspension_travel_norm": { "fl": 0.41, "fr": 0.43, "rl": 0.55, "rr": 0.57 },
    "fuel_frac": 0.62,
    "boost_bar": 0.8,
    "accel_g": { "x": 0.12, "y": -0.04, "z": 1.21 },
    "lap_status": "valid"
  }
}
```

`lap_status ∈ {"valid","dirty","pit","reset","out_lap"}`.

### `lap_completed`

```json
{
  "type": "lap_completed", "schema_version": 1, "t_ms": 1738012345678,
  "data": {
    "lap_number": 3,
    "lap_time_s": 88.902,
    "is_personal_best": true,
    "validity": "valid",
    "invalid_reasons": []
  }
}
```

### `session_started` / `session_ended`

```json
{
  "type": "session_started", "schema_version": 1, "t_ms": 1738012345678,
  "data": { "session_id": "01HQ...", "car_ordinal": 3456,
            "car_class": "S", "car_pi": 750, "drivetrain": "AWD" }
}
```

```json
{
  "type": "session_ended", "schema_version": 1, "t_ms": 1738012345678,
  "data": { "session_id": "01HQ...", "duration_s": 1842.3, "lap_count": 12 }
}
```

### `recommendation`

Mirrors the race-engineer recommendation template
(`Detected / Cause / Adjustment / Expected outcome / Confidence / Caveats`).

```json
{
  "type": "recommendation", "schema_version": 1, "t_ms": 1738012345678,
  "data": {
    "id": "01HQ...",
    "session_id": "01HQ...",
    "lap_number": 3,
    "category": "springs",
    "title": "Front bottoming out",
    "detected": "Front suspension >95% travel on 3 of 4 corners (T1, T3, T7).",
    "cause": "Insufficient front spring rate / ride height for downforce + load.",
    "adjustment": {
      "summary": "Front spring rate 85 → 92 N/mm",
      "parameter": "spring_rate_front",
      "from": 85.0, "to": 92.0, "unit": "N/mm", "step": 1.0
    },
    "expected_outcome": "Eliminates bottoming on T1/T3; slight loss of mechanical grip mid-corner.",
    "confidence": "high",
    "caveats": [
      "Assumes smooth driving style",
      "Re-check after 3 clean laps",
      "If Race Springs not installed, raise ride height +2mm instead"
    ],
    "alternatives": [
      { "summary": "Ride height F +2mm", "parameter": "ride_height_front",
        "from": 110, "to": 112, "unit": "mm" }
    ],
    "driving_style_assumed": "smooth",
    "locked_fallback_used": false
  }
}
```

`category` matches the tuning surface in [`docs/PLAN.md`](../PLAN.md):
`tires | gearing | alignment | anti_roll | springs | damping | aero | brakes | differential`.

`confidence ∈ {"high","medium","low"}`. The live HUD typically shows
`high` and selected `medium`; `low` is post-session report material.

### `error`

```json
{
  "type": "error", "schema_version": 1, "t_ms": 1738012345678,
  "data": {
    "code": "bad_request",
    "message": "set_rate: hz must be in [1, 60]",
    "ref": null
  }
}
```

`code ∈ {"bad_request","unknown_type","schema_mismatch","rate_limited","internal"}`.

## Client → server messages

| `type`             | Effect                                                      |
|--------------------|-------------------------------------------------------------|
| `ping`             | Server replies with `pong` (resets idle timer)              |
| `subscribe`        | Replaces the client's event-type filter                     |
| `set_rate`         | Sets *this client's* telemetry downsample rate (1–60 Hz)    |
| `request_snapshot` | Server emits one `telemetry` frame within 100 ms            |

### `ping` / `pong`

```json
{ "type": "ping", "schema_version": 1, "t_ms": 1738012345678, "data": {} }
```

```json
{ "type": "pong", "schema_version": 1, "t_ms": 1738012345701,
  "data": { "echo_t_ms": 1738012345678 } }
```

### `subscribe`

`events` is a complete list (not a delta). Empty list = silence everything
except `error`/`pong`/`hello`. Default on connect = all event types.

```json
{
  "type": "subscribe", "schema_version": 1, "t_ms": 1738012345678,
  "data": { "events": ["telemetry", "recommendation", "lap_completed",
                       "session_started", "session_ended"] }
}
```

### `set_rate`

```json
{ "type": "set_rate", "schema_version": 1, "t_ms": 1738012345678,
  "data": { "hz": 10 } }
```

Out-of-range values produce an `error` envelope; the previous rate is kept.

### `request_snapshot`

```json
{ "type": "request_snapshot", "schema_version": 1, "t_ms": 1738012345678, "data": {} }
```

## Heartbeat

- Server sends a WebSocket PING control frame every **30 s**.
- Server closes idle clients after **60 s** with no client activity, using
  close code `1011` and reason `"idle timeout"`.
- Application-level `ping`/`pong` envelopes are accepted as an alternative
  for client libraries that hide control frames.

## Slow-consumer handling

- Per-client receive buffer holds **256 frames** (~25 s at 10 Hz).
- When the buffer fills, **oldest telemetry frames are dropped first**.
- `recommendation`, `lap_completed`, `session_*` are **never dropped**.
- A client whose event queue cannot drain within **5 s** is closed with
  code `1008` ("slow consumer").
- The producer (UDP ingest) is never blocked by a slow consumer.

## Versioning

- `schema_version` is an integer that increases with every breaking change.
- The subprotocol carries the major version: `tuning-coach.v1` today.
- A client offering a different `schema_version` than the server is closed
  with code `4001` and reason `"schema_version mismatch: server=N client=M"`.
- A client offering a non-existent subprotocol: the server completes the
  HTTP 101 upgrade and immediately closes the connection with code **`4002`**
  and reason `"unsupported subprotocol"`. The client must reconnect with
  `tuning-coach.v1`. (A post-upgrade close is used so that all client
  WebSocket implementations receive a clean close frame regardless of how
  they handle a refused upgrade.)
- Adding fields to a `data` payload or new event `type`s is non-breaking;
  clients must ignore unknown fields and unknown types.

## Close codes

| Code   | Meaning                              |
|--------|--------------------------------------|
| `1000` | Normal closure                       |
| `1008` | Policy violation (slow consumer)     |
| `1009` | Frame too large (>64 KiB)            |
| `1011` | Server error / idle timeout          |
| `4001` | `schema_version` mismatch            |
| `4002` | Unsupported subprotocol              |

## Multiple clients

Any number of clients may connect simultaneously. Each gets its own filter,
its own downsample rate, and its own slow-consumer accounting. A debug
dashboard at 60 Hz and a HUD at 10 Hz can coexist on the same sidecar.

## Security

The server binds to `127.0.0.1` by default and has no authentication.
Binding to `0.0.0.0` is opt-in via config and emits a warning log on every
accepted upgrade. Do not expose the sidecar to a non-trusted network until a
future ADR introduces auth.

## Test hooks (development/integration tests)

For integration testing, the sidecar exposes:

- `POST /test/telemetry` with body `{ "data": { ... } }`
- `POST /test/recommendation` with body `{ "data": { ... } }`

These routes inject events directly into the same WS fan-out path used by
live producers.
