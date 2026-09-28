use rusqlite::{params, Connection};

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
