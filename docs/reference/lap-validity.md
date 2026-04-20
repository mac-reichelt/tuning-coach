# Lap validity heuristics

The sidecar marks a lap dirty while the lap is in progress and keeps the dirty
state sticky for the rest of the lap.

## Dirty reasons

- `OffTrack` — sustained off-track signal (wheel rumble and/or smoothed surface rumble)
- `WallContact` — acceleration spike above configured G threshold on any axis
- `CornerCut` — excessive combined tire slip at speed while normalized steer is near zero (`best_effort`)
- `Rewind` — rewind detection from distance-traveled regression

The first detected reason is stored in `laps.dirty_reason`. Additional reasons
for the same lap are appended to `laps.dirty_reasons` as a JSON list.

## Defaults and config

All values can be overridden with `TUNING_COACH_*` environment variables.

| Config key | Env var | Default |
|---|---|---|
| `rewind_backward_jump_m` | `TUNING_COACH_REWIND_BACKWARD_JUMP_M` | `50.0` |
| `session_reset_race_time_window_s` | `TUNING_COACH_SESSION_RESET_RACE_TIME_WINDOW_S` | `2.0` |
| `off_track_window_ms` | `TUNING_COACH_OFF_TRACK_WINDOW_MS` | `500` |
| `off_track_min_wheels` | `TUNING_COACH_OFF_TRACK_MIN_WHEELS` | `2` |
| `surface_rumble_threshold` | `TUNING_COACH_SURFACE_RUMBLE_THRESHOLD` | `0.35` |
| `surface_rumble_window_packets` | `TUNING_COACH_SURFACE_RUMBLE_WINDOW_PACKETS` | `5` |
| `wall_contact_g_threshold` | `TUNING_COACH_WALL_CONTACT_G_THRESHOLD` | `10.0` |
| `corner_cut_speed_kph_min` | `TUNING_COACH_CORNER_CUT_SPEED_KPH_MIN` | `30.0` |
| `corner_cut_combined_slip_threshold` | `TUNING_COACH_CORNER_CUT_COMBINED_SLIP_THRESHOLD` | `1.0` |
| `corner_cut_max_abs_steer_norm` | `TUNING_COACH_CORNER_CUT_MAX_ABS_STEER_NORM` | `0.07874016` |

## Signal notes

- `WheelOnRumbleStrip` is evaluated per wheel and defaults to requiring at least
  two wheels.
- `SurfaceRumble` is noisy, so the detector uses a rolling mean over a packet
  window before thresholding.
- `Steer` arrives as `i8` (`-127..127`) and is normalized by dividing by `127.0`.
