use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs,
    path::Path,
};

use rusqlite::Connection;

pub(crate) const LATEST_SCHEMA_VERSION: u32 = 2;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../../migrations/0002_incremental_scan_cache.sql"),
    },
];

#[derive(Debug)]
pub(crate) enum StorageError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    UnsupportedSchemaVersion { found: u32, latest: u32 },
    MigrationVersionMismatch { expected: u32, found: u32 },
}

impl Display for StorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "local storage I/O error: {error}"),
            Self::Sqlite(error) => write!(formatter, "local SQLite error: {error}"),
            Self::UnsupportedSchemaVersion { found, latest } => write!(
                formatter,
                "local database schema version {found} is newer than supported version {latest}"
            ),
            Self::MigrationVersionMismatch { expected, found } => write!(
                formatter,
                "migration expected schema version {expected}, but database reported {found}"
            ),
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::UnsupportedSchemaVersion { .. } | Self::MigrationVersionMismatch { .. } => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

pub(crate) fn initialize(path: &Path) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut connection = Connection::open(path)?;
    configure_connection(&connection)?;
    migrate(&mut connection)?;

    Ok(())
}

fn configure_connection(connection: &Connection) -> Result<(), StorageError> {
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA journal_mode = WAL;
        ",
    )?;

    Ok(())
}

fn schema_version(connection: &Connection) -> Result<u32, StorageError> {
    Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

fn migrate(connection: &mut Connection) -> Result<(), StorageError> {
    let current = schema_version(connection)?;
    if current > LATEST_SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchemaVersion {
            found: current,
            latest: LATEST_SCHEMA_VERSION,
        });
    }

    apply_migrations_through(connection, LATEST_SCHEMA_VERSION)
}

fn apply_migrations_through(
    connection: &mut Connection,
    target_version: u32,
) -> Result<(), StorageError> {
    let mut current = schema_version(connection)?;

    for migration in MIGRATIONS {
        if migration.version <= current || migration.version > target_version {
            continue;
        }

        let transaction = connection.transaction()?;
        transaction.execute_batch(migration.sql)?;

        let reported = schema_version(&transaction)?;
        if reported != migration.version {
            return Err(StorageError::MigrationVersionMismatch {
                expected: migration.version,
                found: reported,
            });
        }

        transaction.commit()?;
        current = migration.version;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn in_memory_database() -> Connection {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        configure_connection(&connection).expect("configure database");
        connection
    }

    fn table_names(connection: &Connection) -> BTreeSet<String> {
        let mut statement = connection
            .prepare(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .expect("prepare table-name query");

        statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query table names")
            .collect::<Result<BTreeSet<_>, _>>()
            .expect("collect table names")
    }

    fn column_names(connection: &Connection, table: &str) -> BTreeSet<String> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("prepare table-info query");

        statement
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query columns")
            .collect::<Result<BTreeSet<_>, _>>()
            .expect("collect columns")
    }

    #[test]
    fn fresh_database_reaches_latest_schema() {
        let mut connection = in_memory_database();

        migrate(&mut connection).expect("migrate fresh database");

        assert_eq!(
            schema_version(&connection).expect("schema version"),
            LATEST_SCHEMA_VERSION
        );

        let expected = BTreeSet::from([
            "conflict_observations".to_string(),
            "diagnostics".to_string(),
            "fingerprints".to_string(),
            "installations".to_string(),
            "local_artifacts".to_string(),
            "local_files".to_string(),
            "preferences".to_string(),
            "restore_points".to_string(),
            "scan_sessions".to_string(),
        ]);

        assert_eq!(table_names(&connection), expected);
    }

    #[test]
    fn version_one_upgrades_forward_without_losing_local_files() {
        let mut connection = in_memory_database();
        apply_migrations_through(&mut connection, 1).expect("apply v1");

        connection
            .execute(
                "INSERT INTO installations
                    (game_root, mods_root, platform, game_version, discovered_at, last_seen_at)
                 VALUES (?1, ?2, 'windows', ?3, ?4, ?4)",
                (
                    r"C:\Games\The Sims 4",
                    r"C:\Users\Player\Documents\Electronic Arts\The Sims 4\Mods",
                    "1.128.90",
                    "2026-09-25T12:00:00Z",
                ),
            )
            .expect("insert installation");

        connection
            .execute(
                "INSERT INTO scan_sessions
                    (installation_id, started_at, status, mode)
                 VALUES (1, ?1, 'completed', 'full')",
                ["2026-09-25T12:01:00Z"],
            )
            .expect("insert scan");

        connection
            .execute(
                "INSERT INTO local_files
                    (installation_id, relative_path, file_kind, first_seen_scan_id, last_seen_scan_id)
                 VALUES (1, ?1, 'package', 1, 1)",
                ["Gameplay/example.package"],
            )
            .expect("insert local file");

        migrate(&mut connection).expect("upgrade database");

        assert_eq!(
            schema_version(&connection).expect("schema version"),
            LATEST_SCHEMA_VERSION
        );

        let columns = column_names(&connection, "local_files");
        for required in ["size_bytes", "modified_ns", "quick_fingerprint", "hashed_at"] {
            assert!(columns.contains(required), "missing cache column {required}");
        }

        let relative_path: String = connection
            .query_row("SELECT relative_path FROM local_files WHERE id = 1", [], |row| row.get(0))
            .expect("local file survives migration");

        assert_eq!(relative_path, "Gameplay/example.package");
    }

    #[test]
    fn incremental_cache_key_is_indexed() {
        let mut connection = in_memory_database();
        migrate(&mut connection).expect("migrate database");

        let index_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_local_files_incremental_cache'",
                [],
                |row| row.get(0),
            )
            .expect("incremental cache index");

        assert!(index_sql.contains("installation_id"));
        assert!(index_sql.contains("relative_path"));
        assert!(index_sql.contains("size_bytes"));
        assert!(index_sql.contains("modified_ns"));
    }

    #[test]
    fn foreign_keys_are_enabled() {
        let connection = in_memory_database();

        let enabled: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign-key pragma");

        assert_eq!(enabled, 1);
    }

    #[test]
    fn future_database_version_is_rejected_instead_of_downgraded() {
        let mut connection = in_memory_database();
        connection
            .execute_batch("PRAGMA user_version = 99;")
            .expect("set future schema version");

        let error = migrate(&mut connection).expect_err("future schema must be rejected");

        assert!(matches!(
            error,
            StorageError::UnsupportedSchemaVersion {
                found: 99,
                latest: LATEST_SCHEMA_VERSION
            }
        ));
    }
}
