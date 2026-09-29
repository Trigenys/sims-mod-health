use rusqlite::{params, Connection, OptionalExtension};

use super::GameContentSnapshot;

pub(crate) trait GameContentRepository {
    fn persist_snapshot(&self, snapshot: &GameContentSnapshot) -> rusqlite::Result<()>;
}

pub(crate) struct SqliteGameContentRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGameContentRepository<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl GameContentRepository for SqliteGameContentRepository<'_> {
    fn persist_snapshot(&self, snapshot: &GameContentSnapshot) -> rusqlite::Result<()> {
        for installation in &snapshot.installations {
            let sentinel_json = serde_json::to_string(&installation.build.sentinels)
                .unwrap_or_else(|_| "[]".to_string());

            self.connection.execute(
                "INSERT INTO game_content_installations (
                    install_root, provider, provider_evidence, game_version,
                    version_evidence_kind, version_confidence, sentinel_json,
                    first_seen_at, last_seen_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                 ON CONFLICT(install_root) DO UPDATE SET
                    provider = excluded.provider,
                    provider_evidence = excluded.provider_evidence,
                    game_version = excluded.game_version,
                    version_evidence_kind = excluded.version_evidence_kind,
                    version_confidence = excluded.version_confidence,
                    sentinel_json = excluded.sentinel_json,
                    last_seen_at = excluded.last_seen_at",
                params![
                    installation.install_root.to_string_lossy(),
                    installation.provider.as_str(),
                    installation.provider_evidence.as_str(),
                    installation
                        .build
                        .version
                        .as_ref()
                        .map(|version| version.normalized.as_str()),
                    installation.build.evidence_kind.as_str(),
                    installation.build.confidence.as_str(),
                    sentinel_json,
                    installation.observed_at,
                ],
            )?;

            let installation_id: i64 = self.connection.query_row(
                "SELECT id FROM game_content_installations WHERE install_root = ?1",
                [installation.install_root.to_string_lossy().to_string()],
                |row| row.get(0),
            )?;

            self.connection.execute(
                "DELETE FROM installed_packs WHERE game_content_installation_id = ?1",
                [installation_id],
            )?;

            for pack in &installation.packs {
                self.connection.execute(
                    "INSERT INTO installed_packs (
                        game_content_installation_id, pack_code, pack_kind, local_state,
                        size_bytes, marker_count, observed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        installation_id,
                        pack.pack_code,
                        pack.pack_kind.as_str(),
                        pack.local_state.as_str(),
                        pack.size_bytes,
                        pack.marker_count,
                        pack.observed_at,
                    ],
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::content_inventory::{
        EvidenceConfidence, GameBuildEvidence, GameContentInstallation, GameProvider,
        InstalledPackObservation, PackKind, PackLocalState, ProviderEvidence, VersionEvidenceKind,
    };
    use rusqlite::Connection;
    use std::path::PathBuf;

    #[test]
    fn persists_normalized_installation_and_pack_observations() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(include_str!(
                "../../../migrations/0007_game_content_inventory.sql"
            ))
            .expect("inventory schema");

        let snapshot = GameContentSnapshot {
            installations: vec![GameContentInstallation {
                install_root: PathBuf::from(r"C:\Games\The Sims 4"),
                provider: GameProvider::EaApp,
                provider_evidence: ProviderEvidence::Registry,
                build: GameBuildEvidence {
                    version: None,
                    evidence_kind: VersionEvidenceKind::SentinelFingerprint,
                    confidence: EvidenceConfidence::Unknown,
                    sentinels: Vec::new(),
                    detail: "fixture".to_string(),
                },
                packs: vec![InstalledPackObservation {
                    pack_code: "EP01".to_string(),
                    pack_kind: PackKind::Expansion,
                    local_state: PackLocalState::Installed,
                    size_bytes: Some(42),
                    marker_count: 1,
                    observed_at: "test".to_string(),
                }],
                observed_at: "test".to_string(),
            }],
        };

        SqliteGameContentRepository::new(&connection)
            .persist_snapshot(&snapshot)
            .expect("persist");

        let provider: String = connection
            .query_row(
                "SELECT provider FROM game_content_installations",
                [],
                |row| row.get(0),
            )
            .expect("provider");
        let pack: String = connection
            .query_row("SELECT pack_code FROM installed_packs", [], |row| {
                row.get(0)
            })
            .expect("pack");

        assert_eq!(provider, "ea_app");
        assert_eq!(pack, "EP01");
    }
}


pub(crate) fn latest_game_content_version(
    connection: &Connection,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT game_version
             FROM game_content_installations
             WHERE game_version IS NOT NULL
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
}
