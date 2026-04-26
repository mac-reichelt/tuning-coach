use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The 9 valid category values for the `recommendations.category` column.
const VALID_CATEGORIES: &[&str] = &[
    "tires",
    "gearing",
    "alignment",
    "anti_roll",
    "springs",
    "damping",
    "aero",
    "brakes",
    "differential",
];

/// A row from the `recommendations` table.
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) struct RecommendationRow {
    pub id: i64,
    pub session_id: i64,
    pub lap_id: Option<i64>,
    pub created_at: String,
    pub category: String,
    pub parameter: Option<String>,
    pub confidence: String,
    pub delivered: bool,
    pub dismissed: bool,
    pub payload_json: Value,
    pub schema_version: i32,
}

/// Typed representation of a `car_setups` row returned by [`Storage::read_car_setup`].
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct CarSetup {
    pub setup: serde_json::Map<String, Value>,
    pub locked_params: Vec<String>,
    pub upgrades: serde_json::Map<String, Value>,
    pub source: String,
}

/// Private type alias for the raw column tuple used by recommendation row mappers.
/// Avoids repeating the long type in multiple function signatures.
type RecommendationRawRow = (
    i64,
    i64,
    Option<i64>,
    String,
    String,
    Option<String>,
    String,
    i64,
    i64,
    String,
    i32,
);

#[derive(Clone)]
pub(crate) struct Storage {
    pool: Pool<SqliteConnectionManager>,
}

impl Storage {
    pub(crate) fn open(data_dir: &Path) -> Result<Self, StorageError> {
        fs::create_dir_all(data_dir).map_err(|source| StorageError::CreateDataDir {
            path: data_dir.to_path_buf(),
            source,
        })?;
        let db_path = data_dir.join("tuning-coach.db");
        let manager = SqliteConnectionManager::file(&db_path).with_init(|conn| {
            conn.execute_batch(
                "PRAGMA busy_timeout=5000;
                 PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=NORMAL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA temp_store=MEMORY;",
            )
        });

        let pool = Pool::builder().max_size(1).build(manager)?;
        let mut conn = pool.get()?;
        run_migrations(&mut conn)?;

        Ok(Self { pool })
    }

    pub(crate) fn start_session(
        &self,
        car_ordinal: Option<i32>,
        sidecar_version: &str,
    ) -> Result<i64, StorageError> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO sessions (started_at, car_ordinal, sidecar_version)
             VALUES (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?1, ?2)",
            params![car_ordinal, sidecar_version],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub(crate) fn end_session(&self, session_id: i64) -> Result<(), StorageError> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE sessions
             SET ended_at = COALESCE(ended_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub(crate) fn ensure_lap(
        &self,
        session_id: i64,
        lap_number: u16,
        started_t_ms: u32,
    ) -> Result<(), StorageError> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO laps(session_id, lap_number, started_t_ms)
                VALUES (?1, ?2, ?3)",
            params![session_id, i64::from(lap_number), i64::from(started_t_ms)],
        )?;
        Ok(())
    }

    pub(crate) fn active_session_id(&self) -> Result<Option<i64>, StorageError> {
        let conn = self.pool.get()?;
        conn.query_row(
            "SELECT id
               FROM sessions
              WHERE ended_at IS NULL
              ORDER BY id DESC
              LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(StorageError::Sqlite(other)),
        })
    }

    pub(crate) fn mark_lap_rewind(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<(), StorageError> {
        let _ = self.mark_lap_dirty(session_id, lap_number, "Rewind")?;
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE laps
                SET is_reset = 1
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
        )?;
        Ok(())
    }

    pub(crate) fn mark_lap_pit_stop(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<(), StorageError> {
        let _ = self.mark_lap_dirty(session_id, lap_number, "PitStop")?;
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE laps
                SET is_pit = 1
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
        )?;
        Ok(())
    }

    pub(crate) fn mark_lap_dirty(
        &self,
        session_id: i64,
        lap_number: u16,
        reason: &str,
    ) -> Result<i64, StorageError> {
        let conn = self.pool.get()?;
        let (lap_id, dirty_reason, dirty_reasons) = conn.query_row(
            "SELECT id, dirty_reason, dirty_reasons
               FROM laps
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )?;

        let dirty_reason = dirty_reason.unwrap_or_else(|| reason.to_string());
        let mut reasons = match dirty_reasons {
            Some(serialized) => parse_dirty_reasons_json(&serialized),
            None => Vec::new(),
        };
        if reasons.is_empty() {
            reasons.push(dirty_reason.clone());
        }
        if !reasons.iter().any(|existing| existing == reason) {
            reasons.push(reason.to_string());
        }
        let dirty_reasons_json = serde_json::to_string(&reasons)?;

        conn.execute(
            "UPDATE laps
                SET valid = 0,
                    dirty_reason = ?3,
                    dirty_reasons = ?4
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![
                session_id,
                i64::from(lap_number),
                dirty_reason,
                dirty_reasons_json
            ],
        )?;
        Ok(lap_id)
    }

    pub(crate) fn mark_lap_dirty_manual_override(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<i64, StorageError> {
        let conn = self.pool.get()?;
        let (lap_id, dirty_reasons) = conn.query_row(
            "SELECT id, dirty_reasons
               FROM laps
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )?;

        let mut reasons = match dirty_reasons {
            Some(serialized) => parse_dirty_reasons_json(&serialized),
            None => Vec::new(),
        };
        if !reasons.iter().any(|existing| existing == "ManualOverride") {
            reasons.push("ManualOverride".to_string());
        }

        conn.execute(
            "UPDATE laps
                SET valid = 0,
                    dirty_reason = 'ManualOverride',
                    dirty_reasons = ?3
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![
                session_id,
                i64::from(lap_number),
                serde_json::to_string(&reasons)?
            ],
        )?;
        Ok(lap_id)
    }

    pub(crate) fn mark_lap_clean(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<Option<String>, StorageError> {
        let conn = self.pool.get()?;
        let previous_reason = conn.query_row(
            "SELECT dirty_reason
               FROM laps
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
            |row| row.get::<_, Option<String>>(0),
        )?;

        conn.execute(
            "UPDATE laps
                SET valid = 1,
                    dirty_reason = NULL,
                    dirty_reasons = NULL
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
        )?;
        Ok(previous_reason)
    }

    /// Insert a new recommendation row.
    ///
    /// Returns the `rowid` of the inserted row on success.
    ///
    /// # Errors
    /// Returns [`StorageError::Schema`] if `category` is not one of the 9 valid values.
    #[allow(dead_code)]
    pub(crate) fn insert_recommendation(
        &self,
        session_id: i64,
        lap_id: Option<i64>,
        category: &str,
        parameter: Option<&str>,
        confidence: &str,
        payload_json: &Value,
    ) -> Result<i64, StorageError> {
        if !VALID_CATEGORIES.contains(&category) {
            return Err(StorageError::Schema(format!(
                "invalid category {:?}, must be one of: {}",
                category,
                VALID_CATEGORIES.join(", ")
            )));
        }
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO recommendations
                (session_id, lap_id, created_at, category, parameter, confidence, payload_json)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?3, ?4, ?5, ?6)",
            params![
                session_id,
                lap_id,
                category,
                parameter,
                confidence,
                payload_json.to_string(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Return all recommendations for a session ordered by `created_at ASC`.
    #[allow(dead_code)]
    pub(crate) fn list_recommendations_for_session(
        &self,
        session_id: i64,
    ) -> Result<Vec<RecommendationRow>, StorageError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, lap_id, created_at, category, parameter,
                    confidence, delivered, dismissed, payload_json, schema_version
               FROM recommendations
              WHERE session_id = ?1
              ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![session_id], map_recommendation_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(build_recommendation_row(row?)?);
        }
        Ok(result)
    }

    /// Return all recommendations associated with a specific lap ordered by `created_at ASC`.
    #[allow(dead_code)]
    pub(crate) fn list_recommendations_for_lap(
        &self,
        lap_id: i64,
    ) -> Result<Vec<RecommendationRow>, StorageError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, lap_id, created_at, category, parameter,
                    confidence, delivered, dismissed, payload_json, schema_version
               FROM recommendations
              WHERE lap_id = ?1
              ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![lap_id], map_recommendation_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(build_recommendation_row(row?)?);
        }
        Ok(result)
    }

    /// Look up the current setup for a car by ordinal.
    ///
    /// Returns `Ok(None)` when no row exists for `car_ordinal` (not an error).
    #[allow(dead_code)]
    pub(crate) fn read_car_setup(
        &self,
        car_ordinal: i32,
    ) -> Result<Option<CarSetup>, StorageError> {
        let conn = self.pool.get()?;
        let result = conn.query_row(
            "SELECT setup_json, locked_params_json, upgrades_json, source
               FROM car_setups
              WHERE car_ordinal = ?1",
            params![car_ordinal],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        );
        match result {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Sqlite(e)),
            Ok((setup_json, locked_params_json, upgrades_json, source)) => {
                let setup = serde_json::from_str::<serde_json::Map<String, Value>>(&setup_json)?;
                let locked_params = serde_json::from_str::<Vec<String>>(&locked_params_json)?;
                let upgrades =
                    serde_json::from_str::<serde_json::Map<String, Value>>(&upgrades_json)?;
                Ok(Some(CarSetup {
                    setup,
                    locked_params,
                    upgrades,
                    source,
                }))
            }
        }
    }

    pub(crate) fn insert_hotkey_event(
        &self,
        session_id: i64,
        t_ms: Option<u32>,
        action: &str,
        payload_json: &Value,
    ) -> Result<String, StorageError> {
        let conn = self.pool.get()?;
        conn.query_row(
            "INSERT INTO hotkey_events (session_id, received_at, t_ms, action, payload_json)
             VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?2, ?3, ?4)
             RETURNING received_at",
            params![
                session_id,
                t_ms.map(i64::from),
                action,
                payload_json.to_string()
            ],
            |row| row.get(0),
        )
        .map_err(StorageError::from)
    }

    #[cfg(test)]
    pub(crate) fn read_lap_validity(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<(bool, Option<String>), StorageError> {
        let conn = self.pool.get()?;
        let tuple = conn.query_row(
            "SELECT valid, dirty_reason
               FROM laps
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )?;
        Ok((tuple.0 != 0, tuple.1))
    }

    #[cfg(test)]
    pub(crate) fn read_lap_dirty_reasons(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<Vec<String>, StorageError> {
        let conn = self.pool.get()?;
        let dirty_reasons = conn.query_row(
            "SELECT dirty_reasons
               FROM laps
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
            |row| row.get::<_, Option<String>>(0),
        )?;
        Ok(dirty_reasons
            .as_deref()
            .map(parse_dirty_reasons_json)
            .unwrap_or_default())
    }

    #[cfg(test)]
    pub(crate) fn count_sessions(&self) -> Result<i64, StorageError> {
        let conn = self.pool.get()?;
        conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .map_err(StorageError::from)
    }

    #[cfg(test)]
    pub(crate) fn first_session_id(&self) -> Result<i64, StorageError> {
        let conn = self.pool.get()?;
        conn.query_row(
            "SELECT id FROM sessions ORDER BY id ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(StorageError::from)
    }

    #[cfg(test)]
    pub(crate) fn session_has_ended_at(&self, session_id: i64) -> Result<bool, StorageError> {
        let conn = self.pool.get()?;
        let ended_at = conn.query_row(
            "SELECT ended_at FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, Option<String>>(0),
        )?;
        Ok(ended_at.is_some())
    }
}

#[derive(Debug, Error)]
pub(crate) enum StorageError {
    #[error("failed to create data directory {path:?}")]
    CreateDataDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("sqlite pool error")]
    Pool(#[from] r2d2::Error),
    #[error("sqlite error")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json serialization error")]
    Json(#[from] serde_json::Error),
    #[error("failed to read migrations directory {path:?}")]
    ReadMigrationsDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read migration file {path:?}")]
    ReadMigrationFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid migration filename {filename:?}, expected NNNN_name.sql")]
    InvalidMigrationFilename { filename: String },
    #[error("duplicate migration version {version}")]
    DuplicateMigrationVersion { version: i64 },
    #[error(
        "applied migration version {version} has mismatched name (db={db_name}, file={file_name})"
    )]
    AppliedMigrationMismatch {
        version: i64,
        db_name: String,
        file_name: String,
    },
    #[error("applied migration version {version} has mismatched sha256 (db={db_sha256}, file={file_sha256})")]
    AppliedMigrationHashMismatch {
        version: i64,
        db_sha256: String,
        file_sha256: String,
    },
    #[error("failed to apply migration {path:?}")]
    ApplyMigration {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[allow(dead_code)]
    #[error("schema constraint violated: {0}")]
    Schema(String),
}

#[derive(Debug)]
struct Migration {
    version: i64,
    name: String,
    sha256: String,
    path: PathBuf,
    sql: String,
}

fn run_migrations(conn: &mut Connection) -> Result<(), StorageError> {
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    run_migrations_from_dir(conn, &migrations_dir)
}

fn run_migrations_from_dir(
    conn: &mut Connection,
    migrations_dir: &Path,
) -> Result<(), StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let migrations = collect_migrations(migrations_dir)?;
    let applied = load_applied_migrations(conn)?;

    for migration in migrations {
        if let Some((db_name, db_sha256)) = applied.get(&migration.version) {
            if db_name != &migration.name {
                return Err(StorageError::AppliedMigrationMismatch {
                    version: migration.version,
                    db_name: db_name.clone(),
                    file_name: migration.name,
                });
            }
            if db_sha256 != &migration.sha256 {
                return Err(StorageError::AppliedMigrationHashMismatch {
                    version: migration.version,
                    db_sha256: db_sha256.clone(),
                    file_sha256: migration.sha256,
                });
            }
            continue;
        }

        let tx = conn.transaction()?;
        tx.execute_batch(&migration.sql)
            .map_err(|source| StorageError::ApplyMigration {
                path: migration.path.clone(),
                source,
            })?;
        tx.execute(
            "INSERT INTO _migrations(version, name, sha256) VALUES (?1, ?2, ?3)",
            params![migration.version, migration.name, migration.sha256],
        )?;
        tx.commit()?;
    }

    Ok(())
}

fn collect_migrations(migrations_dir: &Path) -> Result<Vec<Migration>, StorageError> {
    let entries =
        fs::read_dir(migrations_dir).map_err(|source| StorageError::ReadMigrationsDir {
            path: migrations_dir.to_path_buf(),
            source,
        })?;
    let mut migrations = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|source| StorageError::ReadMigrationsDir {
            path: migrations_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StorageError::InvalidMigrationFilename {
                filename: path.display().to_string(),
            })?;

        let (version, name) = parse_migration_filename(filename)?;
        let sql = fs::read_to_string(&path).map_err(|source| StorageError::ReadMigrationFile {
            path: path.clone(),
            source,
        })?;

        migrations.push(Migration {
            version,
            name,
            sha256: sha256_hex(sql.as_bytes()),
            path,
            sql,
        });
    }

    migrations.sort_by_key(|migration| migration.version);

    for window in migrations.windows(2) {
        if window[0].version == window[1].version {
            return Err(StorageError::DuplicateMigrationVersion {
                version: window[0].version,
            });
        }
    }

    Ok(migrations)
}

fn parse_migration_filename(filename: &str) -> Result<(i64, String), StorageError> {
    let base =
        filename
            .strip_suffix(".sql")
            .ok_or_else(|| StorageError::InvalidMigrationFilename {
                filename: filename.to_string(),
            })?;
    let (version_str, name) =
        base.split_once('_')
            .ok_or_else(|| StorageError::InvalidMigrationFilename {
                filename: filename.to_string(),
            })?;
    if version_str.len() != 4
        || !version_str.chars().all(|ch| ch.is_ascii_digit())
        || name.is_empty()
    {
        return Err(StorageError::InvalidMigrationFilename {
            filename: filename.to_string(),
        });
    }
    let version =
        version_str
            .parse::<i64>()
            .map_err(|_| StorageError::InvalidMigrationFilename {
                filename: filename.to_string(),
            })?;
    Ok((version, name.to_string()))
}

fn load_applied_migrations(
    conn: &Connection,
) -> Result<HashMap<i64, (String, String)>, StorageError> {
    let mut statement = conn.prepare("SELECT version, name, sha256 FROM _migrations")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut applied = HashMap::new();
    for row in rows {
        let (version, name, sha256) = row?;
        applied.insert(version, (name, sha256));
    }
    Ok(applied)
}

fn sha256_hex(contents: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents);
    format!("{:x}", hasher.finalize())
}

fn parse_dirty_reasons_json(serialized: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(serialized) {
        Ok(Value::Array(values)) => values
            .into_iter()
            .filter_map(|value| value.as_str().map(ToOwned::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// Extract raw column values from a `recommendations` row.
///
/// Column positions match the SELECT order used in
/// `list_recommendations_for_session` and `list_recommendations_for_lap`:
/// id(0), session_id(1), lap_id(2), created_at(3), category(4), parameter(5),
/// confidence(6), delivered(7), dismissed(8), payload_json(9), schema_version(10).
#[allow(dead_code)]
fn map_recommendation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecommendationRawRow> {
    Ok((
        row.get::<_, i64>(0)?,
        row.get::<_, i64>(1)?,
        row.get::<_, Option<i64>>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, Option<String>>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, i64>(7)?,
        row.get::<_, i64>(8)?,
        row.get::<_, String>(9)?,
        row.get::<_, i32>(10)?,
    ))
}

/// Convert the raw tuple from [`map_recommendation_row`] into a [`RecommendationRow`].
///
/// Deserializes `payload_json` from the stored TEXT. Returns
/// [`StorageError::Json`] if the stored value is not valid JSON.
#[allow(dead_code)]
fn build_recommendation_row(
    (
        id,
        session_id,
        lap_id,
        created_at,
        category,
        parameter,
        confidence,
        delivered,
        dismissed,
        payload_json_str,
        schema_version,
    ): RecommendationRawRow,
) -> Result<RecommendationRow, StorageError> {
    let payload_json = serde_json::from_str(&payload_json_str)?;
    Ok(RecommendationRow {
        id,
        session_id,
        lap_id,
        created_at,
        category,
        parameter,
        confidence,
        delivered: delivered != 0,
        dismissed: dismissed != 0,
        payload_json,
        schema_version,
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::{run_migrations_from_dir, Storage};

    #[test]
    fn migration_runner_applies_and_is_idempotent() {
        let temp = TempDir::new().expect("temp dir");
        let migrations_dir = temp.path().join("migrations");
        fs::create_dir_all(&migrations_dir).expect("create migrations dir");
        fs::write(
            migrations_dir.join("0001_initial.sql"),
            "CREATE TABLE sessions (id TEXT PRIMARY KEY);",
        )
        .expect("write migration");

        let mut conn = Connection::open(temp.path().join("test.db")).expect("open sqlite");
        run_migrations_from_dir(&mut conn, &migrations_dir).expect("first migration run");
        run_migrations_from_dir(&mut conn, &migrations_dir).expect("second migration run");

        let migration_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .expect("count migrations");
        assert_eq!(migration_count, 1);
        let sha_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations WHERE LENGTH(sha256) = 64",
                [],
                |row| row.get(0),
            )
            .expect("sha256 stored");
        assert_eq!(sha_count, 1);
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'sessions'",
                [],
                |row| row.get(0),
            )
            .expect("sessions table should exist");
        assert_eq!(table_count, 1);
    }

    #[test]
    fn migration_runner_bad_sql_fails_with_context() {
        let temp = TempDir::new().expect("temp dir");
        let migrations_dir = temp.path().join("migrations");
        fs::create_dir_all(&migrations_dir).expect("create migrations dir");
        let bad_migration = migrations_dir.join("0001_bad.sql");
        fs::write(&bad_migration, "CREAT TABLE broken (id INTEGER);").expect("write migration");

        let mut conn = Connection::open(temp.path().join("test.db")).expect("open sqlite");
        let err = run_migrations_from_dir(&mut conn, &migrations_dir)
            .expect_err("bad migration should fail");
        assert!(
            err.to_string().contains("0001_bad.sql"),
            "error should include migration filename: {err}"
        );
    }

    #[test]
    fn integration_open_run_migrations_insert_and_query_session() {
        let temp = TempDir::new().expect("temp dir");
        let mut conn = Connection::open(temp.path().join("integration.db")).expect("open sqlite");
        let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        run_migrations_from_dir(&mut conn, &migrations_dir).expect("run migrations");

        conn.execute(
            "INSERT INTO sessions(started_at, car_ordinal, track_id, sidecar_version)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["2026-01-01T00:00:00Z", 123_i64, "silverstone", "0.1.0"],
        )
        .expect("insert session");

        let track_id: String = conn
            .query_row(
                "SELECT track_id FROM sessions WHERE car_ordinal = ?1",
                rusqlite::params![123_i64],
                |row| row.get(0),
            )
            .expect("query session");
        assert_eq!(track_id, "silverstone");
    }

    #[test]
    fn storage_start_and_end_session_persists_lifecycle() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage should open");
        let session_id = storage
            .start_session(Some(77), "0.1.0")
            .expect("session should insert");

        storage
            .end_session(session_id)
            .expect("session should close");

        let conn = Connection::open(temp.path().join("tuning-coach.db")).expect("open sqlite");
        let (car_ordinal, ended_at): (Option<i64>, Option<String>) = conn
            .query_row(
                "SELECT car_ordinal, ended_at FROM sessions WHERE id = ?1",
                rusqlite::params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("query session");
        assert_eq!(car_ordinal, Some(77));
        assert!(ended_at.is_some());
    }

    #[test]
    fn recommendations_round_trip_insert_and_list_for_session() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage opens");

        let session_id = storage
            .start_session(Some(42), "0.1.0")
            .expect("session starts");

        let payload = serde_json::json!({
            "symptom": "understeer",
            "adjustment": "+0.5 front spring rate",
        });

        let row_id = storage
            .insert_recommendation(
                session_id,
                None,
                "springs",
                Some("spring_rate_front"),
                "high",
                &payload,
            )
            .expect("insert succeeds");
        assert!(row_id > 0, "rowid should be positive");

        let recs = storage
            .list_recommendations_for_session(session_id)
            .expect("list succeeds");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].id, row_id);
        assert_eq!(recs[0].session_id, session_id);
        assert_eq!(recs[0].lap_id, None);
        assert_eq!(recs[0].category, "springs");
        assert_eq!(recs[0].parameter, Some("spring_rate_front".to_string()));
        assert_eq!(recs[0].confidence, "high");
        assert_eq!(recs[0].payload_json, payload);
        assert!(!recs[0].delivered);
        assert!(!recs[0].dismissed);
        assert_eq!(recs[0].schema_version, 1);
    }

    #[test]
    fn recommendations_invalid_category_returns_schema_error() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage opens");

        let session_id = storage
            .start_session(Some(1), "0.1.0")
            .expect("session starts");

        let payload = serde_json::json!({});
        let err = storage
            .insert_recommendation(session_id, None, "invalid_category", None, "high", &payload)
            .expect_err("invalid category should fail");

        assert!(
            matches!(err, super::StorageError::Schema(_)),
            "expected StorageError::Schema, got: {err}"
        );
    }

    #[test]
    fn recommendations_list_for_session_is_empty_when_no_rows() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage opens");

        let session_id = storage
            .start_session(None, "0.1.0")
            .expect("session starts");

        let recs = storage
            .list_recommendations_for_session(session_id)
            .expect("list succeeds");
        assert!(recs.is_empty());
    }

    #[test]
    fn recommendations_list_for_lap_returns_only_that_lap() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage opens");

        let session_id = storage
            .start_session(Some(10), "0.1.0")
            .expect("session starts");
        storage.ensure_lap(session_id, 1, 0).expect("lap created");

        let conn = Connection::open(temp.path().join("tuning-coach.db")).expect("open sqlite");
        let lap_id: i64 = conn
            .query_row(
                "SELECT id FROM laps WHERE session_id = ?1 AND lap_number = 1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .expect("lap exists");

        let payload = serde_json::json!({"detail": "lap-specific"});
        storage
            .insert_recommendation(session_id, Some(lap_id), "tires", None, "low", &payload)
            .expect("insert with lap_id succeeds");

        // Session-level rec (no lap_id) — should NOT appear in list_for_lap
        storage
            .insert_recommendation(
                session_id,
                None,
                "aero",
                None,
                "medium",
                &serde_json::json!({}),
            )
            .expect("session-level insert");

        let recs = storage
            .list_recommendations_for_lap(lap_id)
            .expect("list for lap succeeds");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].lap_id, Some(lap_id));
        assert_eq!(recs[0].category, "tires");
        assert_eq!(recs[0].payload_json, payload);
    }

    #[test]
    fn car_setup_returns_none_for_unknown_ordinal() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage opens");

        let result = storage
            .read_car_setup(99_999)
            .expect("unknown ordinal must not error");
        assert!(result.is_none(), "expected None for unknown ordinal");
    }

    #[test]
    fn car_setup_round_trip() {
        let temp = TempDir::new().expect("temp dir");
        let storage = Storage::open(temp.path()).expect("storage opens");

        let conn = Connection::open(temp.path().join("tuning-coach.db")).expect("open sqlite");
        conn.execute(
            "INSERT INTO car_setups
                (car_ordinal, setup_json, locked_params_json, upgrades_json, source, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                555,
                r#"{"spring_rate_front": 800.0}"#,
                r#"["spring_rate_rear"]"#,
                r#"{"turbo": "stage2"}"#,
                "manual",
                "2026-01-01T00:00:00Z",
            ],
        )
        .expect("insert car_setup");

        let setup = storage
            .read_car_setup(555)
            .expect("no error")
            .expect("setup row found");

        assert_eq!(setup.source, "manual");
        assert_eq!(setup.locked_params, vec!["spring_rate_rear".to_string()]);
        assert_eq!(
            setup.setup.get("spring_rate_front"),
            Some(&serde_json::json!(800.0))
        );
        assert_eq!(
            setup.upgrades.get("turbo"),
            Some(&serde_json::json!("stage2"))
        );
    }
}
