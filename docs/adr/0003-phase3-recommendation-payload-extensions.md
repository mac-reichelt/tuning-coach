# ADR 0003: Phase 3 recommendation payload extensions

- Status: accepted
- Date: 2026-04-26
- Deciders: @mac-reichelt
- Supersedes: —
- Extends: ADR-0002 (additive only — no `schema_version` bump)

## Context

ADR-0002 §`recommendation` defined the top-level WS envelope and the core
fields of the `recommendation` payload. Phase 7 heuristics now need to ship
typed recommendations, and Phase 3 overlay (#52) needs to render them. Before
either can land we must:

1. **Lock the Rust type** that the heuristics engine writes and the WS layer
   broadcasts, so that both sides compile against the same schema.
2. **Document all additive fields** the analyzers require but ADR-0002 left
   underspecified — `corners[]`, `needs_setup_form`, `locked_fallback_used`,
   and `tire_wear_max_at_emit`.
3. **Provide a stub trigger** (`POST /admin/test/recommendation`) so the
   overlay team can wire up their renderer before any live heuristic lands.

No existing field is removed or retyped, so `schema_version` stays `1` per
ADR-0002's forward-compatibility rule.

## Decision

### Additive-only principle

Any change to the `recommendation` `data` payload that adds a new field is
non-breaking. Clients written before this ADR must already ignore unknown
fields (ADR-0002 §Envelope forward-compatibility rule). If any field must be
removed or its type changed, `schema_version` bumps and a new ADR is required.

### Full `recommendation` envelope

```json
{
  "type": "recommendation",
  "schema_version": 1,
  "t_ms": 1738012345678,
  "data": {
    "id": "01HQ7K8YV3EXAMPLE0000001",
    "session_id": "01HQ7K8YV3EXAMPLE0000000",
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
      "step": 1.0,
      "unit": "N/mm"
    },
    "expected_outcome": "Eliminates bottoming on T1/T3; slight loss of mechanical grip mid-corner.",
    "confidence": "high",
    "caveats": [
      "Assumes smooth driving style",
      "Re-check after 3 clean laps",
      "If Race Springs not installed, raise ride height +2mm instead"
    ],
    "alternatives": [
      {
        "summary": "Ride height F +2mm",
        "parameter": "ride_height_front",
        "from": 110.0,
        "to": 112.0,
        "step": 1.0,
        "unit": "mm"
      }
    ],
    "driving_style_assumed": "smooth",
    "locked_fallback_used": false,
    "corners": ["T1", "T3", "T7"],
    "needs_setup_form": false,
    "tire_wear_max_at_emit": 0.15
  }
}
```

### Field reference

#### Envelope fields

| Field            | Type    | Notes                                             |
|------------------|---------|---------------------------------------------------|
| `type`           | string  | Always `"recommendation"`.                        |
| `schema_version` | integer | Always `1` until a breaking change bumps it.      |
| `t_ms`           | integer | Unix epoch ms (server wall clock at emit time).   |
| `data`           | object  | See below.                                        |

#### `data` fields — core (from ADR-0002)

| Field                  | JSON type       | Notes                                                   |
|------------------------|-----------------|---------------------------------------------------------|
| `id`                   | string          | ULID; unique per recommendation event.                  |
| `session_id`           | string          | ULID; matches the active session.                       |
| `lap_number`           | integer ≥ 0     | Lap on which the heuristic fired.                       |
| `category`             | string enum     | See §Category enum below.                               |
| `title`                | string          | One-line human summary.                                 |
| `detected`             | string          | Engineer-format "Detected" line.                        |
| `cause`                | string          | Engineer-format "Likely cause" line.                    |
| `adjustment`           | object          | Primary adjustment; see §Adjustment object.             |
| `expected_outcome`     | string          | Engineer-format "Expected outcome" line.                |
| `confidence`           | string enum     | `"high"` \| `"medium"` \| `"low"`.                     |
| `caveats`              | string[]        | Zero or more engineer-format caveat bullets.            |
| `alternatives`         | Adjustment[]    | Zero or more alternative adjustments.                   |
| `driving_style_assumed`| string          | Driving style context used by the heuristic.            |
| `locked_fallback_used` | boolean         | `true` when the heuristic fell back to a locked preset. |

#### `data` fields — additive (ADR-0003)

| Field                  | JSON type | Notes                                                         |
|------------------------|-----------|---------------------------------------------------------------|
| `corners`              | string[]  | Corner labels (e.g. `"T1"`) where the symptom was observed.   |
| `needs_setup_form`     | boolean   | `true` when the overlay should prompt for a fresh setup form. |
| `tire_wear_max_at_emit`| number    | Highest per-tyre wear fraction `[0.0, 1.0]` at emit time.    |

#### Adjustment object

| Field       | JSON type      | Notes                                                         |
|-------------|----------------|---------------------------------------------------------------|
| `summary`   | string         | Human-readable one-liner (e.g. `"Front spring rate 85 → 92 N/mm"`). |
| `parameter` | string         | Tuning parameter key (e.g. `"spring_rate_front"`).            |
| `from`      | number \| null | Current value; `null` when unknown.                           |
| `to`        | number         | Recommended target value.                                     |
| `step`      | number         | Smallest meaningful increment for this parameter.             |
| `unit`      | string         | Unit label (e.g. `"N/mm"`, `"mm"`, `"°"`, `"%"`).            |

#### Category enum

`category` must be one of:

| Value          | Tuning surface area |
|----------------|---------------------|
| `springs`      | Spring rates (front/rear) |
| `damping`      | Rebound / bump damping |
| `anti_roll`    | Anti-roll bar stiffness |
| `ride_height`  | Ride height (front/rear) |
| `brakes`       | Brake balance / pressure |
| `tires`        | Tyre pressures |
| `gearing`      | Final drive / individual ratios |
| `alignment`    | Camber / toe / caster |
| `aero`         | Front / rear downforce |
| `differential` | Accel / decel / centre diff |
| `engine`       | Engine tuning (where applicable) |

#### Confidence enum

`confidence` must be one of `"high"`, `"medium"`, or `"low"`, following the
race-engineer agent's confidence rules (`.github/agents/race-engineer.agent.md`).

### Stub trigger

```
POST /admin/test/recommendation
```

No request body required. The endpoint emits a canonical stub
`RecommendationPayload` (populated from `recommendation::stub_recommendation()`)
directly into the WS fan-out channel. Any connected overlay client receives
the stub within 200 ms. The endpoint is intended for integration testing and
overlay renderer development only — it must not be called from production
heuristic code.

HTTP response:

```json
{ "emitted": "recommendation" }
```

### Rust types

The canonical Rust representation lives in `sidecar/src/recommendation.rs`:

```
RecommendationCategory  — #[serde(rename_all = "snake_case")] enum
RecommendationConfidence — #[serde(rename_all = "snake_case")] enum
AdjustmentPayload       — serde Serialize + Deserialize struct
RecommendationPayload   — serde Serialize + Deserialize struct
stub_recommendation()   — returns a fully populated example
```

All types derive `Serialize` + `Deserialize` with `serde_json`; the JSON keys
match the field reference tables above.

## Consequences

### Positive

- Overlay (#52) can render recommendations without waiting for live heuristics.
- Heuristics engine has a typed target to emit against; compile-time guarantees
  prevent shape drift.
- The stub trigger is the same fan-out path as live production events, so
  integration tests exercise the real code path.

### Negative / risks

- `needs_setup_form` semantics are intentionally underspecified here; a
  follow-up ADR or issue should define exactly when the overlay must show the
  form and what it contains.
- `corners[]` are free-form strings today; a future ADR may enumerate track
  corner codes to enable i18n and analytics.

## Alternatives considered

**Bump schema_version to 2.** Rejected — all additions are strictly additive;
existing clients that ignore unknown fields continue to work unmodified.

**Inline the stub data in the existing `POST /test/recommendation` endpoint.**
Rejected — that endpoint accepts arbitrary JSON so callers control the shape.
A dedicated `/admin/test/recommendation` route provides a stable, documented
trigger that always emits a spec-conformant payload.

## References

- ADR-0002 — WS API contract (envelope, forward-compatibility rule)
- Issue #53 — this ADR's parent issue
- Issue #52 — overlay renderer (consumer of this contract)
- `.github/agents/race-engineer.agent.md` — confidence rules, tuning surface
- `sidecar/src/recommendation.rs` — authoritative Rust type definitions
