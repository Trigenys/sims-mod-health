use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::storage;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySnapshot {
    pub(crate) has_installation: bool,
    pub(crate) game_version: Option<String>,
    pub(crate) mods_root: Option<String>,
    pub(crate) indexed_count: u64,
    pub(crate) items: Vec<LocalLibraryItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalLibraryItem {
    pub(crate) id: i64,
    pub(crate) relative_path: String,
    pub(crate) file_kind: String,
    pub(crate) enabled: bool,
    pub(crate) parse_status: String,
    pub(crate) size_bytes: Option<i64>,
    pub(crate) embedded_version: Option<String>,
    pub(crate) creator_hint: Option<String>,
}

pub(crate) fn load(database_path: &Path) -> Result<LibrarySnapshot, String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;

    let installation = connection
        .query_row(
            "SELECT id, game_version, mods_root
             FROM installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;

    let Some((installation_id, game_version, mods_root)) = installation else {
        return Ok(LibrarySnapshot {
            has_installation: false,
            game_version: None,
            mods_root: None,
            indexed_count: 0,
            items: Vec::new(),
        });
    };

    let items = load_items(&connection, installation_id).map_err(|error| error.to_string())?;

    Ok(LibrarySnapshot {
        has_installation: true,
        game_version,
        mods_root: Some(mods_root),
        indexed_count: items.len() as u64,
        items,
    })
}

fn load_items(
    connection: &Connection,
    installation_id: i64,
) -> Result<Vec<LocalLibraryItem>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT
            lf.id,
            lf.relative_path,
            lf.file_kind,
            lf.enabled,
            lf.parse_status,
            lf.size_bytes,
            la.embedded_version,
            la.creator_hint
         FROM local_files lf
         LEFT JOIN local_artifacts la ON la.local_file_id = lf.id
         WHERE lf.installation_id = ?1
         ORDER BY lower(lf.relative_path), lf.id",
    )?;

    let rows = statement.query_map([installation_id], |row| {
        Ok(LocalLibraryItem {
            id: row.get(0)?,
            relative_path: row.get(1)?,
            file_kind: row.get(2)?,
            enabled: row.get::<_, i64>(3)? != 0,
            parse_status: row.get(4)?,
            size_bytes: row.get(5)?,
            embedded_version: row.get(6)?,
            creator_hint: row.get(7)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::TempDir;

    #[test]
    fn empty_database_never_invents_library_items() {
        let temp = TempDir::new().expect("temp dir");
        let database = temp.path().join("library.sqlite3");

        let snapshot = load(&database).expect("load empty library");

        assert!(!snapshot.has_installation);
        assert_eq!(snapshot.indexed_count, 0);
        assert!(snapshot.items.is_empty());
    }

    #[test]
    fn library_rows_come_from_scanned_local_files() {
        let temp = TempDir::new().expect("temp dir");
        let database = temp.path().join("library.sqlite3");
        let connection = storage::open(&database).expect("open database");

        connection
            .execute(
                "INSERT INTO installations
                    (game_root, mods_root, platform, game_version, discovered_at, last_seen_at)
                 VALUES (?1, ?2, 'windows', '1.128.90.1030', 'now', 'now')",
                params![r"C:\Users\Player\Documents\Electronic Arts\The Sims 4", r"C:\Users\Player\Documents\Electronic Arts\The Sims 4\Mods"],
            )
            .expect("insert installation");
        connection
            .execute(
                "INSERT INTO local_files
                    (installation_id, relative_path, file_kind, enabled, parse_status)
                 VALUES (1, 'Gameplay/ActuallyInstalled.package', 'package', 1, 'pending')",
                [],
            )
            .expect("insert local file");

        drop(connection);

        let snapshot = load(&database).expect("load library");

        assert!(snapshot.has_installation);
        assert_eq!(snapshot.indexed_count, 1);
        assert_eq!(
            snapshot.items[0].relative_path,
            "Gameplay/ActuallyInstalled.package"
        );
    }
}
