use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{game::content_inventory::SentinelFingerprint, registry::RegistryGameContentManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalPackState {
    pub(crate) code: String,
    pub(crate) local_state: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalGameContentState {
    pub(crate) game_version: Option<String>,
    pub(crate) sentinels: Vec<SentinelFingerprint>,
    pub(crate) packs: Vec<LocalPackState>,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedManifest {
    pub(crate) manifest: RegistryGameContentManifest,
    pub(crate) cached_at: String,
}

pub(crate) fn load_latest_local_state(
    connection: &Connection,
) -> Result<Option<LocalGameContentState>, String> {
    let installation = connection
        .query_row(
            "SELECT id, game_version, sentinel_json
             FROM game_content_installations
             ORDER BY id DESC
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

    let Some((installation_id, game_version, sentinel_json)) = installation else {
        return Ok(None);
    };

    let sentinels =
        serde_json::from_str::<Vec<SentinelFingerprint>>(&sentinel_json).unwrap_or_default();

    let mut statement = connection
        .prepare(
            "SELECT pack_code, local_state
             FROM installed_packs
             WHERE game_content_installation_id = ?1
             ORDER BY pack_code",
        )
        .map_err(|error| error.to_string())?;

    let packs = statement
        .query_map([installation_id], |row| {
            Ok(LocalPackState {
                code: row.get(0)?,
                local_state: row.get(1)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    Ok(Some(LocalGameContentState {
        game_version,
        sentinels,
        packs,
    }))
}

pub(crate) fn save_cached_manifest(
    connection: &Connection,
    manifest: &RegistryGameContentManifest,
) -> Result<(), String> {
    let manifest_json = serde_json::to_string(manifest).map_err(|error| error.to_string())?;

    connection
        .execute(
            "INSERT INTO game_content_manifest_cache (
                singleton_id, manifest_json, source_identity, source_url,
                retrieved_at, expires_at, checksum_sha256, cached_at
             )
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(singleton_id) DO UPDATE SET
                manifest_json = excluded.manifest_json,
                source_identity = excluded.source_identity,
                source_url = excluded.source_url,
                retrieved_at = excluded.retrieved_at,
                expires_at = excluded.expires_at,
                checksum_sha256 = excluded.checksum_sha256,
                cached_at = excluded.cached_at",
            params![
                manifest_json,
                manifest.source_identity,
                manifest.source_url,
                manifest.retrieved_at,
                manifest.expires_at,
                manifest.checksum_sha256,
                observation_timestamp(),
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

pub(crate) fn load_cached_manifest(
    connection: &Connection,
) -> Result<Option<CachedManifest>, String> {
    connection
        .query_row(
            "SELECT manifest_json, cached_at
             FROM game_content_manifest_cache
             WHERE singleton_id = 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(|(manifest_json, cached_at)| {
            let manifest = serde_json::from_str::<RegistryGameContentManifest>(&manifest_json)
                .map_err(|error| error.to_string())?;
            Ok(CachedManifest {
                manifest,
                cached_at,
            })
        })
        .transpose()
}

fn observation_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{RegistryGameBuildManifestEntry, RegistryManifestFingerprint};

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().expect("db");
        connection
            .execute_batch(include_str!(
                "../../../migrations/0007_game_content_inventory.sql"
            ))
            .expect("inventory schema");
        connection
            .execute_batch(include_str!(
                "../../../migrations/0008_game_content_manifest_cache.sql"
            ))
            .expect("cache schema");
        connection
    }

    fn manifest() -> RegistryGameContentManifest {
        RegistryGameContentManifest {
            schema_version: 1,
            manifest_version: "m1".to_string(),
            source_identity: "registry".to_string(),
            source_url: None,
            retrieved_at: "2026-09-28T08:00:00Z".to_string(),
            expires_at: None,
            checksum_sha256: None,
            signature: None,
            latest_game_build: "1.128.90.1030".to_string(),
            game_builds: vec![RegistryGameBuildManifestEntry {
                version: "1.128.90.1030".to_string(),
                released_at: None,
                fingerprints: vec![RegistryManifestFingerprint {
                    relative_path: "Game/Bin/Default.ini".to_string(),
                    sha256: "a".repeat(64),
                }],
            }],
            packs: Vec::new(),
        }
    }

    #[test]
    fn cached_manifest_round_trips_for_offline_use() {
        let connection = connection();
        save_cached_manifest(&connection, &manifest()).expect("save");

        let cached = load_cached_manifest(&connection)
            .expect("load")
            .expect("cached");

        assert_eq!(cached.manifest.manifest_version, "m1");
        assert!(cached.cached_at.starts_with("unix:"));
    }
}
