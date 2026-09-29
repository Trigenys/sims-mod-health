use std::path::Path;

use rusqlite::OptionalExtension;
use serde::Serialize;

use crate::{
    overview,
    registry::{DiscoveryRecommendation, RegistryClient},
    storage,
};

const RECOMMENDATION_LIMIT: usize = 12;
const MIN_IDENTIFIED_MODS: usize = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoveryPrerequisites {
    pub(crate) game_detected: bool,
    pub(crate) patch_known: bool,
    pub(crate) packs_known: bool,
    pub(crate) installed_pack_count: Option<u64>,
    pub(crate) mods_scanned: bool,
    pub(crate) installed_mod_files: Option<u64>,
    pub(crate) identified_mods: Option<u64>,
    pub(crate) registry_available: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoverySnapshot {
    pub(crate) patch_version: Option<String>,
    pub(crate) state: String,
    pub(crate) detail: String,
    pub(crate) blocker: Option<String>,
    pub(crate) prerequisites: DiscoveryPrerequisites,
    pub(crate) recommendations: Vec<DiscoveryRecommendation>,
}

pub(crate) async fn load(database_path: &Path) -> DiscoverySnapshot {
    let (mut prerequisites, local_patch_version) = match local_prerequisites(database_path) {
        Ok(value) => value,
        Err(error) => {
            return DiscoverySnapshot {
                patch_version: None,
                state: "partial".to_string(),
                detail: error,
                blocker: Some("local_state".to_string()),
                prerequisites: empty_prerequisites(),
                recommendations: Vec::new(),
            };
        }
    };

    if !prerequisites.game_detected {
        return blocked(
            local_patch_version,
            prerequisites,
            "game",
            "The Sims 4 program installation has not been detected yet.",
        );
    }

    if !prerequisites.patch_known {
        return blocked(
            local_patch_version,
            prerequisites,
            "patch",
            "The game installation is known, but its version could not be read.",
        );
    }

    if !prerequisites.packs_known {
        return blocked(
            local_patch_version,
            prerequisites,
            "packs",
            "Installed packs have not been inventoried yet.",
        );
    }

    if !prerequisites.mods_scanned {
        return blocked(
            local_patch_version,
            prerequisites,
            "mods_scan",
            "No completed Mods scan is available yet.",
        );
    }

    let seed = match overview::discovery_seed(database_path).await {
        Ok(Some(seed)) => seed,
        Ok(None) => {
            return blocked(
                local_patch_version,
                prerequisites,
                "mods_scan",
                "Run the Mods scan again so Discover can use the current game version.",
            );
        }
        Err(error) => {
            prerequisites.registry_available = Some(false);
            return DiscoverySnapshot {
                patch_version: local_patch_version,
                state: "partial".to_string(),
                detail: error,
                blocker: Some("registry".to_string()),
                prerequisites,
                recommendations: Vec::new(),
            };
        }
    };

    prerequisites.identified_mods = Some(seed.installed_release_ids.len() as u64);
    if seed.registry_checked {
        prerequisites.registry_available = Some(true);
    }

    if seed.installed_release_ids.len() < MIN_IDENTIFIED_MODS {
        return blocked(
            Some(seed.patch_version),
            prerequisites,
            "identified_mods",
            "No installed mod has been identified well enough to seed recommendations yet.",
        );
    }

    let client = match RegistryClient::new(seed.registry_base_url) {
        Ok(client) => client,
        Err(error) => {
            prerequisites.registry_available = Some(false);
            return DiscoverySnapshot {
                patch_version: Some(seed.patch_version),
                state: "partial".to_string(),
                detail: error.to_string(),
                blocker: Some("registry".to_string()),
                prerequisites,
                recommendations: Vec::new(),
            };
        }
    };

    match client
        .recommend_discovery(
            &seed.patch_version,
            &seed.installed_release_ids,
            RECOMMENDATION_LIMIT,
        )
        .await
    {
        Ok(recommendations) => {
            prerequisites.registry_available = Some(true);
            DiscoverySnapshot {
                patch_version: Some(seed.patch_version),
                state: "ready".to_string(),
                detail: if recommendations.is_empty() {
                    "Your setup is ready, but no safe recommendation currently passes compatibility and known-conflict filters."
                        .to_string()
                } else {
                    "Recommendations are ready for the current game and installed mods."
                        .to_string()
                },
                blocker: None,
                prerequisites,
                recommendations,
            }
        }
        Err(error) => {
            prerequisites.registry_available = Some(false);
            DiscoverySnapshot {
                patch_version: Some(seed.patch_version),
                state: if error.is_transport() {
                    "offline".to_string()
                } else {
                    "partial".to_string()
                },
                detail: error.to_string(),
                blocker: Some("registry".to_string()),
                prerequisites,
                recommendations: Vec::new(),
            }
        }
    }
}

fn blocked(
    patch_version: Option<String>,
    prerequisites: DiscoveryPrerequisites,
    blocker: &str,
    detail: &str,
) -> DiscoverySnapshot {
    DiscoverySnapshot {
        patch_version,
        state: "blocked".to_string(),
        detail: detail.to_string(),
        blocker: Some(blocker.to_string()),
        prerequisites,
        recommendations: Vec::new(),
    }
}

fn empty_prerequisites() -> DiscoveryPrerequisites {
    DiscoveryPrerequisites {
        game_detected: false,
        patch_known: false,
        packs_known: false,
        installed_pack_count: None,
        mods_scanned: false,
        installed_mod_files: None,
        identified_mods: None,
        registry_available: None,
    }
}

fn local_prerequisites(
    database_path: &Path,
) -> Result<(DiscoveryPrerequisites, Option<String>), String> {
    let connection = storage::open(database_path).map_err(|error| error.to_string())?;

    let game = connection
        .query_row(
            "SELECT id, game_version
             FROM game_content_installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    let (game_detected, game_version, installed_pack_count) = match game {
        Some((installation_id, version)) => {
            let count = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM installed_packs
                     WHERE game_content_installation_id = ?1
                       AND local_state = 'installed'",
                    [installation_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| error.to_string())?
                .max(0) as u64;
            (true, version, Some(count))
        }
        None => (false, None, None),
    };

    let mod_installation_id = connection
        .query_row(
            "SELECT id
             FROM installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    let (mods_scanned, installed_mod_files) = match mod_installation_id {
        Some(installation_id) => {
            let completed = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1
                        FROM scan_sessions
                        WHERE installation_id = ?1
                          AND status = 'completed'
                    )",
                    [installation_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| error.to_string())?
                != 0;

            let count = connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM local_files
                     WHERE installation_id = ?1
                       AND enabled = 1
                       AND file_kind IN ('package', 'ts4script')",
                    [installation_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| error.to_string())?
                .max(0) as u64;

            (completed, Some(count))
        }
        None => (false, None),
    };

    Ok((
        DiscoveryPrerequisites {
            game_detected,
            patch_known: game_version
                .as_deref()
                .is_some_and(|version| !version.trim().is_empty()),
            packs_known: game_detected,
            installed_pack_count,
            mods_scanned,
            installed_mod_files,
            identified_mods: None,
            registry_available: None,
        },
        game_version,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn database() -> (TempDir, std::path::PathBuf) {
        let temp = TempDir::new().expect("discovery temp");
        let path = temp.path().join("discover.sqlite3");
        storage::initialize(&path).expect("initialize database");
        (temp, path)
    }

    #[test]
    fn zero_extra_packs_is_still_a_known_pack_inventory() {
        let (_temp, path) = database();
        let connection = storage::open(&path).expect("open database");
        connection
            .execute(
                "INSERT INTO game_content_installations (
                    install_root, provider, provider_evidence, game_version,
                    version_evidence_kind, version_confidence, sentinel_json,
                    first_seen_at, last_seen_at
                 ) VALUES (
                    'game', 'ea_app', 'registry', '1.128.90.1030',
                    'default_ini', 'definitive', '[]', 'now', 'now'
                 )",
                [],
            )
            .expect("insert game inventory");
        drop(connection);

        let (prerequisites, patch) = local_prerequisites(&path).expect("prerequisites");

        assert!(prerequisites.game_detected);
        assert!(prerequisites.patch_known);
        assert!(prerequisites.packs_known);
        assert_eq!(prerequisites.installed_pack_count, Some(0));
        assert_eq!(patch.as_deref(), Some("1.128.90.1030"));
    }

    #[test]
    fn completed_mod_scan_is_distinct_from_missing_scan() {
        let (_temp, path) = database();
        let connection = storage::open(&path).expect("open database");
        connection
            .execute(
                "INSERT INTO installations (
                    game_root, mods_root, platform, game_version,
                    discovered_at, last_seen_at
                 ) VALUES ('user', 'mods', 'windows', '1.128.90.1030', 'now', 'now')",
                [],
            )
            .expect("insert mod installation");
        connection
            .execute(
                "INSERT INTO scan_sessions (
                    installation_id, started_at, completed_at, status, mode,
                    files_seen, files_hashed, files_skipped, observation_count
                 ) VALUES (1, 'now', 'now', 'completed', 'full', 0, 0, 0, 0)",
                [],
            )
            .expect("insert scan");
        drop(connection);

        let (prerequisites, _) = local_prerequisites(&path).expect("prerequisites");

        assert!(prerequisites.mods_scanned);
        assert_eq!(prerequisites.installed_mod_files, Some(0));
    }
}
