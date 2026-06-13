# ADR 0005: Recommendation urgency + live heuristics emission timing

- Status: accepted
- Date: 2026-06-13
- Deciders: @mac-reichelt
- Supersedes: —
- Extends: ADR-0003 (additive only — no `schema_version` bump)

## Context

Phase 7 ships the first real heuristics engine
(`sidecar/src/heuristics/`). It reads the live FM 2023 UDP telemetry stream and
produces tuning recommendations driven by
`docs/research/fm2023-tunable-values-and-telemetry-optimization.md`.

The product requirement is a two-tier delivery model:

- **Immediately-apparent, safety-relevant issues** (suspension bottoming,
  wheelspin, brake lockup) must surface **live during the lap** so the driver
  can react.
- **Everything else** (cornering balance, gearing, chassis utilisation) is
  lower priority, needs a whole lap of data to judge, and must **not** interrupt
  a flying lap — it waits until the lap completes or the session pauses/finishes.

The overlay needs to distinguish the two so it can present a live, urgent issue
differently from calmer lap-review feedback. The existing
`RecommendationPayload` (ADR-0003) has no field carrying this distinction.

Separately, the heuristics engine does not yet read the car's live in-game setup
values (that arrives with the Phase 4 setup form). It therefore cannot always
fill `adjustment.from` with the current value, which makes `adjustment.to`
(documented in ADR-0003 as "recommended target value") ambiguous for purely
directional advice such as "soften the front ARB ~2 clicks".

## Decision

### Additive `urgency` field

Add one additive field to the `recommendation` `data` payload:

| Field     | JSON type   | Notes                              |
|-----------|-------------|------------------------------------|
| `urgency` | string enum | `"critical"` \| `"deferred"`.      |

- `critical` — emitted live during the lap, debounced so it fires once per
  episode and at most once per category per cooldown window.
- `deferred` — held until the lap completes or the session pauses/finishes.

This is strictly additive over ADR-0003, so `schema_version` stays `1` per
ADR-0002's forward-compatibility rule (clients ignore unknown fields).

The Rust representation lives in `sidecar/src/recommendation.rs` as
`RecommendationUrgency` (`#[serde(rename_all = "snake_case")]`).

### `adjustment.to` delta semantics when `from` is null

Clarify (not change) the ADR-0003 adjustment contract:

- When `from` is a number, `to` is an **absolute target** value (unchanged from
  ADR-0003).
- When `from` is `null` (current setup value unknown — the common case until the
  Phase 4 setup form lands), `to` is a **signed delta** expressed in `unit`
  increments, and the human-readable `summary` is the authoritative instruction
  the overlay renders (e.g. `summary: "Soften front ARB ~2 clicks"`,
  `to: -2.0`, `unit: "clicks (Δ)"`).

`unit` strings for directional adjustments carry a `(Δ)` suffix to make the
delta semantics explicit to any consumer.

### Emission timing (informative)

The engine maps the two tiers onto the existing channels:

| Tier       | Detector site            | Emitted when                              |
|------------|--------------------------|-------------------------------------------|
| `critical` | per-packet (`on_packet`) | live, on sustained symptom + cooldown     |
| `deferred` | per-lap (`analyze_lap`)  | lap boundary, or session pause/finish     |

Dirty laps (off-track / rewind / reset) suppress **both** tiers for the affected
lap, reusing the existing `suppress_heuristics_tx` gate plus an internal
per-lap dirty flag.

## Consequences

### Positive

- The overlay can style a live critical issue distinctly from lap-review
  feedback (badge + accent) without any schema-version bump.
- Directional recommendations are unambiguous before the setup form exists.
- The two-tier model is encoded in the payload, so future consumers
  (post-session report, Stream Deck) get the distinction for free.

### Negative / risks

- `to`-as-delta is a soft contract that depends on `from === null`; once the
  setup form lands and `from` is populated, recommendations should switch to
  absolute `to` values. The delta convention is intended as a bridge, not a
  permanent design.

## Alternatives considered

**Encode urgency inside `confidence` or `caveats`.** Rejected — overloads
fields with orthogonal meaning and is brittle for consumers to parse.

**Bump `schema_version` to 2.** Rejected — the addition is strictly additive;
existing clients that ignore unknown fields keep working.

## References

- ADR-0002 — WS API contract (envelope, forward-compatibility rule)
- ADR-0003 — Recommendation payload extensions (core/additive fields)
- `docs/research/fm2023-tunable-values-and-telemetry-optimization.md`
- `.github/agents/race-engineer.agent.md` — confidence rules, adjustment caps
- `.github/agents/telemetry-expert.agent.md` — UDP field semantics
- `sidecar/src/heuristics/` — the engine that emits this contract
