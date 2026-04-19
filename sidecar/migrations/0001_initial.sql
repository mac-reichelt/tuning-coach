CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    car_ordinal INTEGER NOT NULL,
    track_id TEXT NOT NULL,
    session_type TEXT
);

CREATE TABLE laps (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    lap_number INTEGER NOT NULL,
    time_s REAL,
    valid INTEGER NOT NULL DEFAULT 1,
    dirty_reason TEXT,
    started_at TEXT,
    ended_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    UNIQUE (session_id, lap_number)
);

CREATE TABLE telemetry_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lap_id TEXT NOT NULL,
    t_ms INTEGER NOT NULL,
    packet_blob BLOB NOT NULL,
    speed_mps REAL,
    rpm REAL,
    throttle REAL,
    brake REAL,
    gear INTEGER,
    is_race_on INTEGER,
    FOREIGN KEY (lap_id) REFERENCES laps(id) ON DELETE CASCADE
);

CREATE TABLE recommendations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    lap_id TEXT,
    category TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    confidence REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (lap_id) REFERENCES laps(id) ON DELETE SET NULL
);

CREATE TABLE car_setups (
    car_ordinal INTEGER PRIMARY KEY,
    setup_json TEXT NOT NULL,
    locked_params_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL
);

CREATE TABLE user_preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE hotkey_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    ts TEXT NOT NULL,
    action TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_laps_session_lap_number
    ON laps(session_id, lap_number);

CREATE INDEX idx_telemetry_snapshots_lap_t_ms
    ON telemetry_snapshots(lap_id, t_ms);

CREATE INDEX idx_recommendations_session
    ON recommendations(session_id);

CREATE INDEX idx_recommendations_session_lap
    ON recommendations(session_id, lap_id);

CREATE INDEX idx_hotkey_events_session_ts
    ON hotkey_events(session_id, ts);
