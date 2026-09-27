use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::Path,
};

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::storage::{self, StorageError};

const DIAGNOSTIC_TELEMETRY_KEY: &str = "privacy.diagnostic_telemetry_enabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrivacyPreferences {
    pub(crate) diagnostic_telemetry_enabled: bool,
}

impl Default for PrivacyPreferences {
    fn default() -> Self {
        Self {
            diagnostic_telemetry_enabled: false,
        }
    }
}

#[derive(Debug)]
pub(crate) enum PrivacyError {
    Storage(StorageError),
    Sqlite(rusqlite::Error),
    Serialization(serde_json::Error),
}

impl Display for PrivacyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::Sqlite(error) => write!(formatter, "privacy SQLite error: {error}"),
            Self::Serialization(error) => write!(formatter, "privacy serialization error: {error}"),
        }
    }
}

impl Error for PrivacyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Serialization(error) => Some(error),
        }
    }
}

impl From<StorageError> for PrivacyError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<rusqlite::Error> for PrivacyError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<serde_json::Error> for PrivacyError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

pub(crate) fn load_preferences(database_path: &Path) -> Result<PrivacyPreferences, PrivacyError> {
    let connection = storage::open(database_path)?;
    let stored = connection
        .query_row(
            "SELECT value_json FROM preferences WHERE key = ?1",
            [DIAGNOSTIC_TELEMETRY_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    let diagnostic_telemetry_enabled = match stored {
        Some(value) => serde_json::from_str::<bool>(&value).unwrap_or(false),
        None => false,
    };

    Ok(PrivacyPreferences {
        diagnostic_telemetry_enabled,
    })
}

pub(crate) fn set_diagnostic_telemetry(
    database_path: &Path,
    enabled: bool,
) -> Result<PrivacyPreferences, PrivacyError> {
    let connection = storage::open(database_path)?;
    let value_json = serde_json::to_string(&enabled)?;

    connection.execute(
        "INSERT INTO preferences (key, value_json, updated_at)
         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT(key) DO UPDATE SET
             value_json = excluded.value_json,
             updated_at = excluded.updated_at",
        params![DIAGNOSTIC_TELEMETRY_KEY, value_json],
    )?;

    Ok(PrivacyPreferences {
        diagnostic_telemetry_enabled: enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn database() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("privacy temp directory");
        let database_path = temp.path().join("privacy.sqlite3");
        storage::initialize(&database_path).expect("initialize privacy database");
        (temp, database_path)
    }

    #[test]
    fn diagnostic_telemetry_is_opt_in_by_default() {
        let (_temp, database_path) = database();

        let preferences = load_preferences(&database_path).expect("load privacy preferences");

        assert!(!preferences.diagnostic_telemetry_enabled);
    }

    #[test]
    fn explicit_consent_is_persisted_and_can_be_revoked() {
        let (_temp, database_path) = database();

        let enabled =
            set_diagnostic_telemetry(&database_path, true).expect("enable diagnostic telemetry");
        assert!(enabled.diagnostic_telemetry_enabled);
        assert!(load_preferences(&database_path)
            .expect("reload enabled privacy preferences")
            .diagnostic_telemetry_enabled);

        let disabled =
            set_diagnostic_telemetry(&database_path, false).expect("disable diagnostic telemetry");
        assert!(!disabled.diagnostic_telemetry_enabled);
        assert!(!load_preferences(&database_path)
            .expect("reload disabled privacy preferences")
            .diagnostic_telemetry_enabled);
    }

    #[test]
    fn malformed_stored_consent_fails_closed() {
        let (_temp, database_path) = database();
        let connection = storage::open(&database_path).expect("open privacy database");
        connection
            .execute(
                "INSERT INTO preferences (key, value_json, updated_at)
                 VALUES (?1, ?2, 'now')",
                params![DIAGNOSTIC_TELEMETRY_KEY, "\"unexpected\""],
            )
            .expect("insert malformed preference");

        let preferences = load_preferences(&database_path).expect("load malformed privacy setting");

        assert!(!preferences.diagnostic_telemetry_enabled);
    }
}
