# Lap validity heuristics

This page is the source-of-truth reference for Phase 2 lap-validity detection.
Defaults were verified against [`LapValidityConfig::default()`](../../sidecar/src/lap_validity.rs)
and [`AppConfig::default()`](../../sidecar/src/main.rs).

## Thresholds quick reference (verified against source)

All keys are loaded from `TUNING_COACH_*` environment variables in
[`AppConfig::load`](../../sidecar/src/main.rs).

| Config key | Env var | Default | Used by |
|---|---|---:|---|
| `off_track_window_ms` | `TUNING_COACH_OFF_TRACK_WINDOW_MS` | `500` | `OffTrack` |
| `off_track_min_wheels` | `TUNING_COACH_OFF_TRACK_MIN_WHEELS` | `2` | `OffTrack` |
| `surface_rumble_threshold` | `TUNING_COACH_SURFACE_RUMBLE_THRESHOLD` | `0.35` | `OffTrack` |
| `surface_rumble_window_packets` | `TUNING_COACH_SURFACE_RUMBLE_WINDOW_PACKETS` | `5` | `OffTrack` |
| `wall_contact_g_threshold` | `TUNING_COACH_WALL_CONTACT_G_THRESHOLD` | `10.0` | `WallContact` |
| `corner_cut_speed_kph_min` | `TUNING_COACH_CORNER_CUT_SPEED_KPH_MIN` | `30.0` | `CornerCut` |
| `corner_cut_combined_slip_threshold` | `TUNING_COACH_CORNER_CUT_COMBINED_SLIP_THRESHOLD` | `1.0` | `CornerCut` |
| `corner_cut_max_abs_steer_norm` | `TUNING_COACH_CORNER_CUT_MAX_ABS_STEER_NORM` | `0.07874016` (`10.0/127.0`) | `CornerCut` |
| `pit_entry_speed_threshold_kph` | `TUNING_COACH_PIT_ENTRY_SPEED_THRESHOLD_KPH` | `20.0` | `PitStop` |
| `pit_entry_dwell_s` | `TUNING_COACH_PIT_ENTRY_DWELL_S` | `3.0` | `PitStop` |
| `pit_exit_speed_threshold_kph` | `TUNING_COACH_PIT_EXIT_SPEED_THRESHOLD_KPH` | `40.0` | `PitStop` |
| `pit_exit_dwell_s` | `TUNING_COACH_PIT_EXIT_DWELL_S` | `1.0` | `PitStop` |
| `rewind_backward_jump_m` | `TUNING_COACH_REWIND_BACKWARD_JUMP_M` | `50.0` | `Rewind` |
| `session_reset_race_time_window_s` | `TUNING_COACH_SESSION_RESET_RACE_TIME_WINDOW_S` | `2.0` | `SessionReset` |

## Detector reference

Telemetry offsets below are from the FM 2023 Dash packet layout used by the
sidecar parser (`RawDashPacket`/`RawSledPacket`) and fixture docs:
[`telemetry.rs`](../../sidecar/src/telemetry.rs),
[`fixtures README`](../../sidecar/tests/fixtures/lap_validity/README.md).

### `OffTrack`

- **Code path**: [`off_track_triggered`](../../sidecar/src/lap_validity.rs)
- **Telemetry fields**:
  - `WheelOnRumbleStripFrontLeft/Right/RearLeft/RearRight` @ offsets `116/120/124/128` (`i32`)
  - `SurfaceRumbleFrontLeft/Right/RearLeft/RearRight` @ offsets `148/152/156/160` (`f32`)
  - `TimestampMS` @ offset `4` (`u32`)
- **Defaults / config overrides**:
  - `off_track_window_ms = 500`
  - `off_track_min_wheels = 2`
  - `surface_rumble_threshold = 0.35`
  - `surface_rumble_window_packets = 5`
- **Known false positive**: aggressive kerb riding can keep `WheelOnRumbleStrip`
  high and spike rumble.
- **Mitigation**: raise `off_track_min_wheels` and/or
  `surface_rumble_threshold`, or increase `off_track_window_ms`.

### `WallContact`

- **Code path**: [`wall_contact_triggered`](../../sidecar/src/lap_validity.rs)
- **Telemetry fields**:
  - `AccelerationX/Y/Z` @ offsets `20/24/28` (`f32`, m/s²)
- **Defaults / config overrides**:
  - `wall_contact_g_threshold = 10.0` (converted to m/s² with `9.81`)
- **Known false positive**: suspension/compression spikes on severe kerbs can
  exceed the threshold.
- **Mitigation**: increase `wall_contact_g_threshold`.

### `CornerCut` (`best_effort`)

- **Code path**: [`corner_cut_triggered`](../../sidecar/src/lap_validity.rs)
- **Telemetry fields**:
  - `Speed` @ offset `244` (`f32`, m/s; converted to kph)
  - `TireCombinedSlipFrontLeft/Right/RearLeft/RearRight` @ offsets `180/184/188/192` (`f32`)
  - `Steer` @ offset `308` (`i8`, normalized by `/127.0`)
- **Defaults / config overrides**:
  - `corner_cut_speed_kph_min = 30.0`
  - `corner_cut_combined_slip_threshold = 1.0`
  - `corner_cut_max_abs_steer_norm = 0.07874016`
- **Known false positive**: wheelspin or traction-loss events while steering is
  near center can look like a cut.
- **Mitigation**: increase `corner_cut_combined_slip_threshold` and/or
  `corner_cut_speed_kph_min`, or lower `corner_cut_max_abs_steer_norm`.

### `PitStop`

- **Code path**: [`detect_pit_stop`](../../sidecar/src/lap_validity.rs)
- **Telemetry fields**:
  - `Speed` @ offset `244` (`f32`, m/s; converted to kph)
  - `TimestampMS` @ offset `4` (`u32`)
  - `LapNumber` @ offset `300` (`u16`)
- **Defaults / config overrides**:
  - entry: `pit_entry_speed_threshold_kph = 20.0` for `pit_entry_dwell_s = 3.0`
  - exit: `pit_exit_speed_threshold_kph = 40.0` for `pit_exit_dwell_s = 1.0`
  - validation rule: entry threshold must be lower than exit threshold
- **Known false positive**: very slow sectors/hairpins can mimic pit-entry speed.
- **Mitigation**: reduce `pit_entry_speed_threshold_kph` and/or increase
  `pit_entry_dwell_s`.

### `Rewind`

- **Code path**: [`detect_rewind`](../../sidecar/src/lap_validity.rs)
- **Telemetry fields**:
  - `DistanceTraveled` @ offset `280` (`f32`)
  - `TimestampMS` @ offset `4` (`u32`)
  - `LapNumber` @ offset `300` (`u16`)
- **Defaults / config overrides**:
  - `rewind_backward_jump_m = 50.0`
  - hard guardrail (non-config): ignore samples if packet gap > `10_000 ms`
- **Known false positive**: large backward teleports unrelated to explicit rewind
  (for example mode-specific repositioning) can still look like rewind.
- **Mitigation**: increase `rewind_backward_jump_m`.

### `SessionReset`

- **Code path**: [`detect_session_reset`](../../sidecar/src/lap_validity.rs)
- **Telemetry fields**:
  - `LapNumber` @ offset `300` (`u16`)
  - `CurrentRaceTime` @ offset `296` (`f32`)
  - `TimestampMS` @ offset `4` (`u32`)
  - `CarOrdinal` @ offset `212` (`i32`) to start the replacement session
- **Defaults / config overrides**:
  - `session_reset_race_time_window_s = 2.0`
  - trigger requires both:
    - lap number drops to zero from a previous non-zero lap
    - race time moves backward and is below the configured window
- **Known false positive**: game modes with atypical lap/time resets near start
  can satisfy both conditions.
- **Mitigation**: lower `session_reset_race_time_window_s` to tighten detection.

### `ManualOverride`

- **Code path**:
  - [`POST /hotkeys/mark-lap-dirty`](../../sidecar/src/hotkeys.rs)
  - [`POST /hotkeys/mark-lap-clean`](../../sidecar/src/hotkeys.rs)
- **Telemetry/session fields used indirectly**:
  - active session id from storage
  - current lap context (`LapNumber`, `TimestampMS`, `CarOrdinal`) from latest
    Dash packet
- **Defaults / config overrides**: none (operator-triggered only).
- **Known false positive**: accidental hotkey press marks a lap dirty/clean.
- **Mitigation**: bind deliberate hotkeys and use `mark-lap-clean` to revert an
  accidental dirty override.

## Dirty reason persistence behavior

- `OffTrack`, `WallContact`, `CornerCut`, `Rewind`, and `ManualOverride` are
  stored as dirty reasons.
- First reason is persisted to `laps.dirty_reason`; additional reasons append to
  `laps.dirty_reasons` JSON list in storage.
- `PitStop` and `SessionReset` emit lap-validity events but do not create a
  dirty reason code.

## Session state machine reference

State machine implementation: [`session_state.rs`](../../sidecar/src/session_state.rs).

### States

- `Loading`
- `InRace`
- `Paused`
- `Finished`

### Legal transitions and triggers

| From | To | Trigger condition |
|---|---|---|
| `Loading` | `InRace` | `is_race_on == true` and `current_race_time_s > 0.0` |
| `Finished` | `InRace` | Same race-start condition as above |
| `InRace` | `Paused` | `is_race_on == false` continuously for at least `pause_debounce_ms` |
| `Paused` | `InRace` | `is_race_on == true` |
| `InRace` | `Loading` | Session boundary: `lap_number == 0` and `current_race_time_s < 2.0` |
| `InRace` | `Finished` | No packet for at least `packet_timeout_ms` |
| `Paused` | `Finished` | No packet for at least `packet_timeout_ms` |

## Resolved open questions from Phase 2 stories

- **Wall-contact G threshold finalized** at `10.0 G`
  (`TUNING_COACH_WALL_CONTACT_G_THRESHOLD`).
- **Off-track wheel count finalized** at `2+ wheels` plus sustained-window logic
  (`500 ms`) and smoothed rumble fallback (`0.35` over 5 packets).
- **Pit-stop thresholds finalized** with hysteresis and dwell
  (`20/40 kph`, `3.0/1.0 s`) to reduce chatter.
- **Rewind/session-reset split finalized**:
  - Rewind = backward jump in `DistanceTraveled` over threshold.
  - Session reset = lap drop to zero + race-time reset window.
- **Manual override behavior finalized** as explicit hotkey/API actions that
  emit `ManualOverride` dirty events or `LapCleanMarked`.
