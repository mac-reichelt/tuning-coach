use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use thiserror::Error;

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
                "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=NORMAL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA busy_timeout=5000;
                 PRAGMA temp_store=MEMORY;",
            )
        });

        let pool = Pool::builder().max_size(4).build(manager)?;
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

    pub(crate) fn mark_lap_rewind(
        &self,
        session_id: i64,
        lap_number: u16,
    ) -> Result<(), StorageError> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE laps
                SET valid = 0,
                    dirty_reason = 'Rewind',
                    is_reset = 1
              WHERE session_id = ?1
                AND lap_number = ?2",
            params![session_id, i64::from(lap_number)],
        )?;
        Ok(())
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
    pub(crate) fn count_sessions(&self) -> Result<i64, StorageError> {
        let conn = self.pool.get()?;
        conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
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
}
