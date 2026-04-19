# Forza Motorsport telemetry fixtures

Real-world Forza FM 2023 Dash UDP telemetry captured for testing the tuning-coach
sidecar's lap-validity, contact, rewind, pit, and session-reset detectors.

## Files

| File                | Description                                                  |
| ------------------- | ------------------------------------------------------------ |
| `fm-training.pcapng` | 11.9 MB sliced packet capture (30,048 Forza Dash packets, ~360s) |
| `scenarios.json`     | Curated annotation: driver narrative, gap classifications, expected detector outputs |

The fixture was sliced from a larger 814 MB capture; only UDP packets to
`127.0.0.1:5300` containing 331-byte Dash payloads are kept, trimmed to the range
between the first and last `IsRaceOn=1` packet. The fixture's SHA256 is recorded
in `scenarios.json` (`fixture_sha256`).

## Capture conditions

- **Game**: Forza Motorsport (FM 2023), Dash telemetry format
- **Car**: 2006 Audi RS4 (`CarOrdinal = 419`)
- **Track**: Brands Hatch Indy Circuit (`TrackOrdinal = 861`)
- **UDP**: forwarded by SimHub to `127.0.0.1:5300`
- **Rate**: ~60 Hz
- **OS**: Windows 11 capture via Wireshark on `\Device\NPF_Loopback`

## Driver's notes

5 laps were driven from a stationary start at the start/finish line:

| Lap | What happened                          |
| --- | -------------------------------------- |
| 1   | rewind, off-track                      |
| 2   | clean                                  |
| 3   | contact, pit                           |
| 4   | off-track, rewind across start line    |
| 5   | session reset, then clean lap          |

Plus pre-lap activity: an exploratory crawl + pause/unpause before Lap 1.

## Telemetry timeline structure

The capture contains alternating **segments** (player in control, `IsRaceOn=1`)
and **gaps** (player not in control, `IsRaceOn=0`). During gaps Forza zeros the
position/speed fields in outgoing packets but **continues to update
`DistanceTraveled` internally**, which lets us classify what happened in the gap
by comparing the last in-control packet to the first packet of the next segment.

### Gap classification rules

| Classification    | Rule                                                                                |
| ----------------- | ----------------------------------------------------------------------------------- |
| `session_reset`   | First packet after the gap has `DistanceTraveled = -1944.0` (game's fresh-session sentinel) |
| `rewind`          | `DistanceTraveled` decreased by >10 m across the gap                                |
| `cinematic`       | Position teleported >50 m **and** `DistanceTraveled` increased >25 m (game drove the car) |
| `pause`           | Position teleport <5 m **and** `\|DistanceTraveled` delta`\| <5 m`                  |
| `unknown`         | Did not match any rule (investigate)                                                |

### In-segment event rules

| Event     | Rule                                                                                |
| --------- | ----------------------------------------------------------------------------------- |
| `contact` | `sqrt(AccelX² + AccelY² + AccelZ²) / 9.81 ≥ 5 G`, debounced by 0.5 s, peak retained |

> **Note on off-track / dirty laps**: surface rumble fires on intentional curb
> use and is too noisy for a reliable detector. Consider lap-time invalidation
> from the game itself or a future tire-slip + position-against-track-edges
> detector. Off-track is intentionally not a detector category in this fixture.

## Driver lap → telemetry mapping

| Driver lap | Telemetry segment / gap                                                |
| ---------- | ---------------------------------------------------------------------- |
| Lap 1      | `seg02 LapNum=0` (preceded by `gap01 rewind`)                          |
| Lap 2      | `seg02 LapNum=1`                                                       |
| Lap 3      | `seg02 LapNum=2` (contact at pkt 13464; pit cinematic in `gap02`)      |
| Lap 4      | `seg03 LapNum=3` (rewind across start in `gap03`)                      |
| Lap 5      | `seg06 LapNum=0` (preceded by `gap05 session_reset`)                   |

See `scenarios.json` for the full per-segment / per-gap breakdown including
packet ranges, durations, and expected detector outputs.

## Expected detector outputs

| Detector       | Expected hits                                                          |
| -------------- | ---------------------------------------------------------------------- |
| `rewind`       | `gap01`, `gap03`                                                       |
| `cinematic`    | `gap02`                                                                |
| `pause`        | `gap00`, `gap04`                                                       |
| `session_reset`| `gap05`                                                                |
| `contact`      | 1 hit, ~13.4 G in `seg02 LapNum=2` (Lap 3)                             |

## Regenerating the fixture

The slicer and analyzer are committed alongside this fixture under
`sidecar/scripts/` (Python 3.10+, stdlib only):

```bash
# slice a master capture down to the relevant range
python3 sidecar/scripts/fm-slice.py \
    /path/to/master.pcapng \
    sidecar/tests/fixtures/fm-training.pcapng \
    <first_forza_idx> <last_forza_idx>

# regenerate scenarios.json
python3 sidecar/scripts/fm-analyze.py sidecar/tests/fixtures/fm-training.pcapng
# then curate via the script in scripts/build-scenarios.py
```

`<first_forza_idx>` and `<last_forza_idx>` are 0-based positions within the
filtered Forza-stream (port 5300 only), as printed by `fm-analyze.py`.

## Forza Dash packet field offsets (FM 2023)

Verified against this capture; in little-endian byte order:

| Offset | Type | Field                  |
| -----: | ---- | ---------------------- |
|      0 | i32  | `IsRaceOn` (0 / 1)     |
|      4 | u32  | `TimestampMS`          |
|     20 | f32  | `AccelerationX`        |
|     24 | f32  | `AccelerationY`        |
|     28 | f32  | `AccelerationZ`        |
|    212 | i32  | `CarOrdinal`           |
|    232 | f32  | `PositionX`            |
|    236 | f32  | `PositionY`            |
|    240 | f32  | `PositionZ`            |
|    244 | f32  | `Speed` (m/s)          |
|    280 | f32  | `DistanceTraveled` (m, sentinel `-1944.0` on fresh session) |
|    284 | f32  | `BestLap`              |
|    288 | f32  | `LastLap`              |
|    292 | f32  | `CurrentLap`           |
|    296 | f32  | `RaceTime`             |
|    300 | u16  | `LapNumber`            |
|    327 | i32  | `TrackOrdinal` (FM 2023 addition) |
