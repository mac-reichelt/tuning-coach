# ADR 0001: Sidecar Storage Schema (SQLite)

- Status: proposed
- Date: 2025-11-15
- Deciders: @mac-reichelt
- Issue: [#8](https://github.com/mac-reichelt/tuning-coach/issues/8)

## Context

The `tuning-coach` Rust sidecar ingests Forza Motorsport telemetry over UDP at
~60 Hz, runs heuristics over rolling windows, emits tuning recommendations to
the SimHub overlay over WebSocket, accepts hotkey events from SimHub, and
persists per-car setup state. Phase 1 of [`docs/PLAN.md`](../PLAN.md) requires a
durable local store for:

- **Sessions** — one bounded period of driving on a given car/track.
- **Laps** — segmented from telemetry; tagged valid/dirty/pit/reset.
- **Telemetry snapshots** — high-rate parsed packets, ~60 Hz.
- **Recommendations** — structured engineer output (symptom, mechanism,
  adjustment, confidence) emitted live and aggregated post-session.
- **Car setups** — per-car known/locked/current parameter values.
- **User preferences** — overlay + coach config (style override, snooze, etc.).
- **Hotkey events** — discrete inputs from SimHub Global Hotkeys
  (mark dirty / pit / snooze / request feedback / reset).

### Constraints

- **Single-user desktop app.** No multi-tenant, no auth, no network DB.
- **Pre-1.0.** Schema can change, but every change must ship a migration.
- **`rusqlite` with `bundled` feature** — pin the SQLite version to what we
  test against; avoid platform SQLite drift on Windows.
- **WAL mode.** Concurrent reader (overlay query thread) + single writer
  (telemetry ingest) without contention.
- **File path:** `<data_dir>/tuning-coach.db` (Windows
  `%LOCALAPPDATA%\tuning-coach\tuning-coach.db`).
- **Backup-friendly.** Copying the `.db` file (and `.db-wal`, `.db-shm`) while
  the app is closed must produce a usable backup.

### Read patterns

| Reader | Query shape | Latency budget |
|---|---|---|
| Live overlay HUD | "give me the most recent N recommendations for current session" | < 50 ms |
| Heuristics engine | "give me last K seconds of telemetry for current lap" | < 10 ms (in-memory ring is primary; DB is fallback for replay) |
| Post-session report | "all laps + aggregated recommendations for session X" | < 1 s |
| Recorded session replay (Phase 9) | "stream all snapshots for session X in t_ms order" | sequential scan, throughput-bound |
| Setup form | "current setup + locked params for car ordinal C" | < 50 ms |

### Write patterns

| Writer | Rate | Burst |
|---|---|---|
| Telemetry ingest | ~60 rows/s during sessions | bounded by packet rate |
| Recommendations | < 1 row/s typical | bursts at lap boundaries |
| Hotkey events | event-driven; tens per session | n/a |
| Car setups / preferences | manual; rare | n/a |

The dominant cost is telemetry. Everything else is rounding error.

## Decision

We will use **SQLite** (via `rusqlite` with the `bundled` feature) as the
canonical persistence layer, opened in **WAL mode**, with the schema below.

For `telemetry_snapshots` we will use a **hybrid layout**: store the raw Forza
packet as a `BLOB`, alongside a small set of indexed/denormalized columns
needed for the most common live and analysis queries (`lap_id`, `t_ms`,
`speed_mps`, `rpm`, `throttle`, `brake`, `gear`, `is_race_on`). All other
fields are recovered by re-parsing the blob on demand. This pins our storage
cost close to the raw-blob lower bound while preserving fast lap/time-range
scans and basic filtering without a parse step.

A `_migrations` table tracks applied schema versions. Migration files live in
`sidecar/migrations/NNNN_<name>.sql` and are applied in numeric order at
startup, with each migration executed in its own transaction; each migration
is recorded with its SHA-256 to detect tampering.

### Schema

All timestamps are **UTC ISO-8601 strings** (`TEXT`) for human readability
unless they're sub-second deltas, in which case they are `INTEGER` milliseconds.
Wall-clock fields are display-only; elapsed-time fields use monotonic deltas
captured by the ingest loop.

#### `_migrations`

Tracks which migration files have been applied.

| Column | Type | Notes |
|---|---|---|
| `version` | `INTEGER PRIMARY KEY` | matches `NNNN` prefix of migration file |
| `name` | `TEXT NOT NULL` | filename minus prefix and extension |
| `sha256` | `TEXT NOT NULL` | hex digest of the migration file at apply time |
| `applied_at` | `TEXT NOT NULL` | UTC ISO-8601 |

Behavior:
- On startup, migrations are sorted by `version`, applied in a single
  transaction each, recorded on success.
- If a previously applied migration's on-disk SHA differs, the runner aborts
  with a clear error (do not silently re-run).
- The runner is idempotent: re-running with no new files is a no-op.

#### `sessions`

| Column | Type | Notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | |
| `started_at` | `TEXT NOT NULL` | UTC ISO-8601 |
| `ended_at` | `TEXT` | NULL while in progress |
| `car_ordinal` | `INTEGER` | from packet; NULL if unknown at start |
| `car_class` | `INTEGER` | 0=D … 7=X |
| `car_pi` | `INTEGER` | performance index |
| `drivetrain` | `INTEGER` | 0=FWD, 1=RWD, 2=AWD |
| `track_id` | `TEXT` | resolved name from SimHub list (Phase 11); NULL until then |
| `game` | `TEXT NOT NULL DEFAULT 'forza_motorsport_2023'` | discriminator for future games |
| `sidecar_version` | `TEXT NOT NULL` | semver of producing build, for replay forward-compat |
| `notes` | `TEXT` | user-editable, optional |

Indexes:
- `idx_sessions_started_at (started_at DESC)` — list view sort
- `idx_sessions_car_track (car_ordinal, track_id, started_at DESC)` — trend queries

#### `laps`

| Column | Type | Notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | |
| `session_id` | `INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE` | |
| `lap_number` | `INTEGER NOT NULL` | from packet (`LapNumber`); 0 = out lap |
| `started_t_ms` | `INTEGER NOT NULL` | session-relative monotonic ms |
| `ended_t_ms` | `INTEGER` | NULL until lap closes |
| `time_s` | `REAL` | final lap time in seconds; NULL until close |
| `valid` | `INTEGER NOT NULL DEFAULT 1` | boolean (0/1) |
| `dirty_reason` | `TEXT` | enum string: `off_track` \| `contact` \| `grip_anomaly` \| `manual` \| `pit` \| `reset` \| NULL |
| `is_pit` | `INTEGER NOT NULL DEFAULT 0` | boolean |
| `is_reset` | `INTEGER NOT NULL DEFAULT 0` | boolean (rewind detected) |
| `best_split_s` | `REAL` | optional aggregate, populated post-lap |

Indexes:
- `idx_laps_session_lap (session_id, lap_number)` UNIQUE — guard double-insert
- `idx_laps_session_started (session_id, started_t_ms)` — time-range joins

#### `telemetry_snapshots`

The hot table. Hybrid layout: blob + a thin index of common-query columns.

| Column | Type | Notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | |
| `session_id` | `INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE` | denormalized for session-scoped scans without join |
| `lap_id` | `INTEGER REFERENCES laps(id) ON DELETE SET NULL` | NULL for pre-lap-close packets, backfilled on lap close |
| `t_ms` | `INTEGER NOT NULL` | session-relative monotonic ms; primary time axis |
| `is_race_on` | `INTEGER NOT NULL` | boolean from packet |
| `speed_mps` | `REAL NOT NULL` | from packet |
| `rpm` | `REAL NOT NULL` | `CurrentEngineRpm` |
| `throttle` | `INTEGER NOT NULL` | 0..255 raw |
| `brake` | `INTEGER NOT NULL` | 0..255 raw |
| `gear` | `INTEGER NOT NULL` | 0=R, 1..N |
| `packet` | `BLOB NOT NULL` | raw 311-byte Forza Dash packet, little-endian |
| `packet_format` | `INTEGER NOT NULL DEFAULT 1` | discriminator: 1=`forza_dash_v2_311b` |

Indexes:
- `idx_telemetry_session_t (session_id, t_ms)` — replay & windowed scans
- `idx_telemetry_lap_t (lap_id, t_ms)` WHERE `lap_id IS NOT NULL` (partial index) — per-lap analysis

Notes:
- The blob is the **source of truth**. The denormalized columns are a cache;
  any disagreement is a parser bug. Tests will assert round-trip equality on a
  sampling basis.
- We do **not** index `t_ms` alone — all time queries are scoped by session or
  lap.
- Future packet formats (Sled-only, other games) are stored as new
  `packet_format` values with their own raw bytes.

#### `recommendations`

| Column | Type | Notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | |
| `session_id` | `INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE` | |
| `lap_id` | `INTEGER REFERENCES laps(id) ON DELETE SET NULL` | NULL = session-level (e.g., post-session) |
| `created_at` | `TEXT NOT NULL` | UTC ISO-8601 |
| `category` | `TEXT NOT NULL` | enum: `tires` \| `gearing` \| `alignment` \| `anti_roll` \| `springs` \| `damping` \| `aero` \| `brakes` \| `differential` |
| `parameter` | `TEXT` | specific param key, e.g. `spring_rate_front`; NULL for multi-param advice |
| `confidence` | `TEXT NOT NULL` | enum: `high` \| `medium` \| `low` |
| `delivered` | `INTEGER NOT NULL DEFAULT 0` | 1 once shown in HUD |
| `dismissed` | `INTEGER NOT NULL DEFAULT 0` | user dismissed in HUD |
| `payload_json` | `TEXT NOT NULL` | full structured rec (see [race-engineer.agent.md](../../.github/agents/race-engineer.agent.md) format): symptom, mechanism, adjustment, expected outcome, caveats, style/locked-param notes |
| `schema_version` | `INTEGER NOT NULL DEFAULT 1` | of the JSON payload, independent of DB schema |

Indexes:
- `idx_recs_session_created (session_id, created_at DESC)` — list view
- `idx_recs_lap (lap_id)` — per-lap rollup
- `idx_recs_category (session_id, category)` — "show me all springs advice"

#### `car_setups`

One row per car ordinal — current setup state, with snapshot history kept in
`payload_json` (small enough we don't need a separate history table yet).

| Column | Type | Notes |
|---|---|---|
| `car_ordinal` | `INTEGER PRIMARY KEY` | from packet |
| `car_label` | `TEXT` | display name; resolved later from SimHub list |
| `setup_json` | `TEXT NOT NULL` | full tune object, keyed by parameter |
| `locked_params_json` | `TEXT NOT NULL` | array of param keys the car can't adjust |
| `upgrades_json` | `TEXT NOT NULL DEFAULT '{}'` | installed upgrades affecting locked set |
| `source` | `TEXT NOT NULL` | enum: `manual` \| `ocr` \| `imported` |
| `updated_at` | `TEXT NOT NULL` | UTC ISO-8601 |
| `schema_version` | `INTEGER NOT NULL DEFAULT 1` | of the JSON shape |

No additional indexes — single-row lookup by PK.

#### `user_preferences`

Free-form key-value store for overlay + coach config. Typed keys are validated
in code, not in the schema, so adding a preference doesn't require a migration.

| Column | Type | Notes |
|---|---|---|
| `key` | `TEXT PRIMARY KEY` | dot.separated.key |
| `value_json` | `TEXT NOT NULL` | JSON-encoded scalar/object |
| `updated_at` | `TEXT NOT NULL` | UTC ISO-8601 |

#### `hotkey_events`

| Column | Type | Notes |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | |
| `session_id` | `INTEGER REFERENCES sessions(id) ON DELETE CASCADE` | NULL if no active session |
| `received_at` | `TEXT NOT NULL` | UTC ISO-8601 (display) |
| `t_ms` | `INTEGER` | session-relative monotonic ms (NULL if no session) |
| `action` | `TEXT NOT NULL` | enum: `mark_dirty` \| `pit_in` \| `pit_out` \| `snooze` \| `request_feedback` \| `reset_session` |
| `payload_json` | `TEXT` | optional extra context |

Indexes:
- `idx_hotkey_session_t (session_id, t_ms)` — correlate with telemetry

### Connection management

- Single connection used for writes, opened with `journal_mode=WAL`,
  `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`.
- Read-only connections via a small pool (`r2d2_sqlite`, size 4) for the WS
  query path. Reads see WAL snapshots so they don't block the writer.
- All schema state is created/upgraded by the migration runner before any
  reader/writer is exposed.

### Pragmas applied at startup

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
```

## Consequences

### Positive

- **One file = one backup.** Users can copy `tuning-coach.db` (with `-wal` /
  `-shm` siblings if open) and have everything.
- **Hybrid telemetry layout** keeps storage close to raw-bytes minimum while
  the few hot columns make the live overlay and lap analysis fast without a
  parse step.
- **Blob is source of truth** — adding a new derived field never requires a
  destructive migration; we just re-parse historical packets.
- **Migration runner with SHA pinning** catches the classic "someone edited an
  applied migration" footgun loudly instead of silently corrupting state.
- **Foreign keys + cascades** make session deletion (a user-facing feature
  later) a single statement.
- **JSON columns** for recommendations / setups / preferences absorb iteration
  during the heuristics phase without DB churn; we promote fields out of JSON
  if they become query targets.

### Negative

- **Storage cost.** A 1-hour active session is ~6.6 MB of raw packets
  (~311 B × 60 Hz × 3600 s) plus ~10–15 MB for the row overhead and
  denormalized columns and indexes — call it **~20 MB/hour active**. A heavy
  weekend (~10 hours) is ~200 MB. Acceptable for a desktop app; we'll add a
  retention setting before 1.0 (open question, deferred).
- **Querying non-indexed packet fields requires reparsing.** Acceptable: those
  queries are post-session/replay paths, not hot.
- **JSON-in-SQL** loses static schema guarantees. Mitigated with strict
  serde-typed structs in Rust + `schema_version` columns so we can migrate
  payloads in code.
- **Two sources of truth** for the indexed columns vs the blob. Mitigated by a
  single write path (parser → row builder → insert) and a sampled round-trip
  test in CI.

### Neutral

- All timestamps are TEXT ISO-8601 except monotonic deltas, which are INTEGER
  ms. Mixing types is intentional — display vs math.
- We use `INTEGER` (0/1) for booleans because that's what SQLite gives us.
- Car/track resolution to human names is a Phase 11 concern; the schema
  already has the columns, they just sit NULL until then.

## Alternatives Considered

### A. Telemetry as raw blob only (no denormalized columns)

- **Pros:** smallest on-disk footprint; one source of truth; no risk of column
  drift; simplest insert path.
- **Cons:** every "show me speed over last 30 s" query has to read + parse N
  blobs. Live overlay would either need a parallel in-memory mirror (which we
  have anyway, but it has finite history) or pay the parse cost on every
  request.
- **Why not chosen:** the parse cost isn't catastrophic, but it forces every
  consumer to depend on the parser crate. Hybrid layout costs us ~30% extra
  storage in exchange for SQL-native queries on the fields that 90% of
  callers want.

### B. Telemetry fully denormalized (one column per packet field, no blob)

- **Pros:** fully queryable; no parser dependency on the read path; trivial
  joins.
- **Cons:** ~80–100 columns per row; ~3–4× storage of the blob layout; every
  new packet field is a destructive migration; replaying historical sessions
  through an updated parser is impossible because we threw the bytes away.
- **Why not chosen:** loses our forward-compat escape hatch. Heuristics will
  iterate fast; we will *want* to re-derive new fields from old sessions.

### C. DuckDB instead of SQLite

- **Pros:** columnar, vectorized, fast aggregates over telemetry; great for
  the post-session report and replay use cases.
- **Cons:** less mature on Windows desktop; weaker concurrent
  reader-while-writing story; embedding story for `rusqlite`-style bundled
  use is less battle-tested; bigger binary; team familiarity with SQLite is
  higher.
- **Why not chosen:** boring tech wins. If aggregate query perf becomes a
  bottleneck, we can ETL session blobs into Parquet for offline analytics
  without changing the canonical store.

### D. Flat Parquet files per session

- **Pros:** ideal for replay scans and external tooling (DuckDB, pandas);
  excellent compression on telemetry.
- **Cons:** mutable state (recommendations, setups, preferences) doesn't fit
  cleanly; concurrent live append + read requires extra machinery; lose ACID;
  backup story is per-file not per-DB.
- **Why not chosen:** wrong shape for the *transactional* parts of the app.
  Parquet is on the table as an **export** format (the existing "on-demand
  JSON export" decision in PLAN.md can grow a Parquet sibling) but not as the
  primary store.

### E. SQLite with one row per *field* (EAV)

- **Pros:** infinitely flexible; no migrations.
- **Cons:** unusable. ~80× row count, no type safety, no useful indexes.
- **Why not chosen:** classic anti-pattern.

## Migration & evolution plan

- Migrations are files: `sidecar/migrations/0001_initial.sql`,
  `0002_<change>.sql`, etc. Numeric prefix is monotonic and gap-free.
- Every change to the schema (including adding/removing indexes) ships a new
  migration file; we never edit a previously-applied migration.
- Payload-only changes (`recommendations.payload_json`, `car_setups.setup_json`)
  bump the row's `schema_version` and are migrated lazily in code on read.
- Pre-1.0: if a migration would be too painful to write, we may ship a
  one-shot "drop & recreate the dev DB" path *behind a feature flag*. After
  1.0, every change is a real migration.
- Rolling back is not supported; the user keeps the previous binary if they
  want the previous schema. (Single-user desktop app — acceptable.)

## Backup story

- App-shutdown backup: copy `tuning-coach.db`. WAL has been checkpointed at
  graceful shutdown; no sidecar files needed.
- Live backup: copy `tuning-coach.db`, `tuning-coach.db-wal`,
  `tuning-coach.db-shm` together, or use `VACUUM INTO 'backup.db'` from a
  read connection. Both produce a consistent snapshot.
- `docs/getting-started.md` will document the file path and the
  shutdown-then-copy flow as the recommended approach.

## Storage estimates

Assumptions: ~311 B per Forza Dash packet, ~60 Hz, ~150 B of denormalized
columns + row overhead, ~50% index overhead on the indexed columns.

| Workload | Raw bytes | With indexes | Per session (~30 min) | Per heavy week (~10 h) |
|---|---|---|---|---|
| Telemetry only | ~22 MB/h | ~30 MB/h | ~15 MB | ~300 MB |
| + recommendations + laps + hotkeys | +<1 MB/h | +<1 MB/h | ~16 MB | ~310 MB |

Order of magnitude: a serious user generates **GB-scale data per year**. We
will introduce a configurable retention policy (open question in PLAN.md)
before 1.0, e.g. "keep last N sessions in full; older sessions keep
recommendations + laps but drop telemetry."

## Observability

- Migration runner emits structured INFO logs per applied version
  (`migration.applied version=NNNN sha=…`).
- Telemetry insert path exposes a counter (`telemetry_rows_written_total`) and
  a gauge for write-batch latency (`telemetry_write_batch_ms`). DB file size
  is a periodic gauge.
- Errors are typed (`StorageError::{Migrate, Open, Write, Read, Schema}`),
  mapped to user-facing strings only at the WS / CLI boundary.

## References

- Issue [#8 — feat(sidecar): SQLite schema](https://github.com/mac-reichelt/tuning-coach/issues/8)
- [docs/PLAN.md](../PLAN.md) — Phase 1 sidecar foundation
- [.github/agents/telemetry-expert.agent.md](../../.github/agents/telemetry-expert.agent.md) — Forza Dash packet schema (311 B)
- [.github/agents/race-engineer.agent.md](../../.github/agents/race-engineer.agent.md) — recommendation payload shape
- SQLite WAL: <https://www.sqlite.org/wal.html>
- `rusqlite` bundled feature: <https://docs.rs/rusqlite/latest/rusqlite/#bundled>
