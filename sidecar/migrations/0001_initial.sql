CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    car_ordinal INTEGER,
    car_class INTEGER,
    car_pi INTEGER,
    drivetrain INTEGER,
    track_id TEXT,
    game TEXT NOT NULL DEFAULT 'forza_motorsport_2023',
    sidecar_version TEXT NOT NULL,
    notes TEXT
);

CREATE TABLE laps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    lap_number INTEGER NOT NULL,
    started_t_ms INTEGER NOT NULL,
    ended_t_ms INTEGER,
    time_s REAL,
    valid INTEGER NOT NULL DEFAULT 1,
    dirty_reason TEXT,
    is_pit INTEGER NOT NULL DEFAULT 0,
    is_reset INTEGER NOT NULL DEFAULT 0,
    best_split_s REAL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE (session_id, lap_number)
);

CREATE TABLE telemetry_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    lap_id INTEGER,
    t_ms INTEGER NOT NULL,
    is_race_on INTEGER NOT NULL,
    speed_mps REAL NOT NULL,
    rpm REAL NOT NULL,
    throttle INTEGER NOT NULL,
    brake INTEGER NOT NULL,
    gear INTEGER NOT NULL,
    packet BLOB NOT NULL,
    packet_format INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (lap_id) REFERENCES laps(id) ON DELETE SET NULL
);

CREATE TABLE recommendations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    lap_id INTEGER,
    created_at TEXT NOT NULL,
    category TEXT NOT NULL,
    parameter TEXT,
    confidence TEXT NOT NULL,
    delivered INTEGER NOT NULL DEFAULT 0,
    dismissed INTEGER NOT NULL DEFAULT 0,
    payload_json TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (lap_id) REFERENCES laps(id) ON DELETE SET NULL
);

CREATE TABLE car_setups (
    car_ordinal INTEGER PRIMARY KEY,
    car_label TEXT,
    setup_json TEXT NOT NULL,
    locked_params_json TEXT NOT NULL,
    upgrades_json TEXT NOT NULL DEFAULT '{}',
    source TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE user_preferences (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE hotkey_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER,
    received_at TEXT NOT NULL,
    t_ms INTEGER,
    action TEXT NOT NULL,
    payload_json TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_started_at
    ON sessions(started_at DESC);

CREATE INDEX idx_sessions_car_track
    ON sessions(car_ordinal, track_id, started_at DESC);

CREATE UNIQUE INDEX idx_laps_session_lap
    ON laps(session_id, lap_number);

CREATE INDEX idx_laps_session_started
    ON laps(session_id, started_t_ms);

CREATE INDEX idx_telemetry_session_t
    ON telemetry_snapshots(session_id, t_ms);

CREATE INDEX idx_telemetry_lap_t
    ON telemetry_snapshots(lap_id, t_ms)
    WHERE lap_id IS NOT NULL;

CREATE INDEX idx_recs_session_created
    ON recommendations(session_id, created_at DESC);

CREATE INDEX idx_recs_lap
    ON recommendations(lap_id);

CREATE INDEX idx_recs_category
    ON recommendations(session_id, category);

CREATE INDEX idx_hotkey_session_t
    ON hotkey_events(session_id, t_ms);
