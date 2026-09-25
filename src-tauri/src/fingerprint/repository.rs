use std::path::Path;

use rusqlite::{params, Connection};

use super::{
    domain::{
        ArtifactKind, FingerprintError, FingerprintKind, FingerprintValue,
        SHA256_ALGORITHM_VERSION,
    },
    providers,
};

pub(crate) struct FingerprintRepository<'a> {
    connection: &'a Connection,
}

impl<'a> FingerprintRepository<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn invalidate_local_derived(
        &self,
        local_file_id: i64,
    ) -> Result<(), rusqlite::Error> {
        self.connection.execute(
            "DELETE FROM fingerprints
             WHERE local_file_id = ?1
               AND kind IN ('sha256', 'resource_signature', 'script_signature', 'quick')",
            [local_file_id],
        )?;
        Ok(())
    }

    pub(crate) fn store(
        &self,
        local_file_id: i64,
        fingerprint: &FingerprintValue,
    ) -> Result<(), rusqlite::Error> {
        self.connection.execute(
            "INSERT INTO fingerprints (
                local_file_id,
                kind,
                value,
                algorithm_version,
                computed_at
             )
             VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(local_file_id, kind) DO UPDATE SET
                value = excluded.value,
                algorithm_version = excluded.algorithm_version,
                computed_at = excluded.computed_at",
            params![
                local_file_id,
                fingerprint.kind.database_value(),
                fingerprint.value,
                fingerprint.algorithm_version
            ],
        )?;
        Ok(())
    }

    pub(crate) fn store_sha256(
        &self,
        local_file_id: i64,
        value: &str,
    ) -> Result<(), rusqlite::Error> {
        self.store(
            local_file_id,
            &FingerprintValue {
                kind: FingerprintKind::Sha256,
                value: value.to_string(),
                algorithm_version: SHA256_ALGORITHM_VERSION,
            },
        )
    }

    pub(crate) fn replace_structural(
        &self,
        local_file_id: i64,
        kind: ArtifactKind,
        path: &Path,
    ) -> Result<Option<FingerprintValue>, FingerprintError> {
        let fingerprint_kind = kind.structural_fingerprint_kind();

        self.connection.execute(
            "DELETE FROM fingerprints WHERE local_file_id = ?1 AND kind = ?2",
            params![local_file_id, fingerprint_kind.database_value()],
        )?;

        let fingerprint = providers::compute_structural_fingerprint(kind, path)?;
        if let Some(value) = &fingerprint {
            self.store(local_file_id, value)?;
        }

        Ok(fingerprint)
    }
}

pub(crate) fn delete_cached_fingerprints(
    connection: &Connection,
    local_file_id: i64,
) -> Result<(), rusqlite::Error> {
    FingerprintRepository::new(connection).invalidate_local_derived(local_file_id)
}

pub(crate) fn store_sha256(
    connection: &Connection,
    local_file_id: i64,
    value: &str,
) -> Result<(), rusqlite::Error> {
    FingerprintRepository::new(connection).store_sha256(local_file_id, value)
}

pub(crate) fn refresh_structural_fingerprint(
    connection: &Connection,
    local_file_id: i64,
    kind: ArtifactKind,
    path: &Path,
) -> Result<Option<FingerprintValue>, FingerprintError> {
    FingerprintRepository::new(connection).replace_structural(local_file_id, kind, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dbpf::ResourceKey, storage};
    use tempfile::TempDir;

    fn database_fixture() -> (TempDir, Connection, i64) {
        let temp = TempDir::new().expect("create database temp directory");
        let database_path = temp.path().join("fingerprints.sqlite3");
        storage::initialize(&database_path).expect("initialize fingerprint database");
        let connection = storage::open(&database_path).expect("open fingerprint database");

        connection
            .execute(
                "INSERT INTO installations (
                    game_root, mods_root, platform, discovered_at, last_seen_at
                 )
                 VALUES ('game', 'mods', 'windows', 'now', 'now')",
                [],
            )
            .expect("insert installation");

        connection
            .execute(
                "INSERT INTO local_files (
                    installation_id, relative_path, file_kind
                 )
                 VALUES (1, 'mod.package', 'package')",
                [],
            )
            .expect("insert local file");

        (temp, connection, 1)
    }

    fn minimal_dbpf(keys: &[ResourceKey]) -> Vec<u8> {
        const HEADER_SIZE: usize = 96;
        let mut index = Vec::new();
        index.extend_from_slice(&0_u32.to_le_bytes());

        for key in keys {
            index.extend_from_slice(&key.resource_type.to_le_bytes());
            index.extend_from_slice(&key.group.to_le_bytes());
            index.extend_from_slice(&((key.instance >> 32) as u32).to_le_bytes());
            index.extend_from_slice(&(key.instance as u32).to_le_bytes());
            index.extend_from_slice(&0_u32.to_le_bytes());
            index.extend_from_slice(&0_u32.to_le_bytes());
            index.extend_from_slice(&0_u32.to_le_bytes());
        }

        let mut bytes = vec![0_u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"DBPF");
        bytes[4..8].copy_from_slice(&2_u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&1_u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&(keys.len() as u32).to_le_bytes());
        bytes[40..44].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        bytes[44..48].copy_from_slice(&(index.len() as u32).to_le_bytes());
        bytes.extend(index);
        bytes
    }

    #[test]
    fn storing_same_kind_replaces_previous_value() {
        let (_temp, connection, local_file_id) = database_fixture();
        let repository = FingerprintRepository::new(&connection);

        repository.store_sha256(local_file_id, "old").expect("store old SHA");
        repository.store_sha256(local_file_id, "new").expect("replace SHA");

        let values = connection
            .prepare(
                "SELECT value FROM fingerprints
                 WHERE local_file_id = ?1 AND kind = 'sha256'",
            )
            .expect("prepare current SHA query")
            .query_map([local_file_id], |row| row.get::<_, String>(0))
            .expect("query current SHA")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect current SHA");

        assert_eq!(values, vec!["new"]);
    }

    #[test]
    fn invalidation_removes_local_derived_fingerprints() {
        let (_temp, connection, local_file_id) = database_fixture();
        let repository = FingerprintRepository::new(&connection);

        repository.store_sha256(local_file_id, "old-sha").expect("store SHA");
        repository
            .store(
                local_file_id,
                &FingerprintValue {
                    kind: FingerprintKind::ResourceSignature,
                    value: "old-resource".to_string(),
                    algorithm_version: super::super::domain::DBPF_RESOURCE_SIGNATURE_VERSION,
                },
            )
            .expect("store resource signature");

        repository
            .invalidate_local_derived(local_file_id)
            .expect("invalidate local fingerprint cache");

        let remaining: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM fingerprints WHERE local_file_id = ?1",
                [local_file_id],
                |row| row.get(0),
            )
            .expect("count remaining fingerprints");

        assert_eq!(remaining, 0);
    }

    #[test]
    fn invalidation_preserves_source_owned_fingerprints() {
        let (_temp, connection, local_file_id) = database_fixture();
        let repository = FingerprintRepository::new(&connection);

        repository.store_sha256(local_file_id, "local-sha").expect("store SHA");
        repository
            .store(
                local_file_id,
                &FingerprintValue {
                    kind: FingerprintKind::CurseForge,
                    value: "source-owned".to_string(),
                    algorithm_version: "curseforge-v1",
                },
            )
            .expect("store source fingerprint");

        repository
            .invalidate_local_derived(local_file_id)
            .expect("invalidate local derived fingerprints");

        let remaining: Vec<(String, String)> = connection
            .prepare(
                "SELECT kind, value
                 FROM fingerprints
                 WHERE local_file_id = ?1
                 ORDER BY kind",
            )
            .expect("prepare remaining fingerprint query")
            .query_map([local_file_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query remaining fingerprints")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect remaining fingerprints");

        assert_eq!(
            remaining,
            vec![("curseforge".to_string(), "source-owned".to_string())]
        );
    }

    #[test]
    fn structural_fingerprint_is_persisted_with_algorithm_version() {
        let temp = TempDir::new().expect("create DBPF temp directory");
        let path = temp.path().join("mod.package");
        std::fs::write(
            &path,
            minimal_dbpf(&[ResourceKey {
                resource_type: 1,
                group: 2,
                instance: 3,
            }]),
        )
        .expect("write DBPF fixture");

        let (_db_temp, connection, local_file_id) = database_fixture();
        let repository = FingerprintRepository::new(&connection);

        let fingerprint = repository
            .replace_structural(local_file_id, ArtifactKind::Package, &path)
            .expect("refresh structural signature")
            .expect("resource signature exists");

        let persisted: (String, String) = connection
            .query_row(
                "SELECT value, algorithm_version
                 FROM fingerprints
                 WHERE local_file_id = ?1 AND kind = 'resource_signature'",
                [local_file_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load resource signature");

        assert_eq!(persisted.0, fingerprint.value);
        assert_eq!(
            persisted.1,
            super::super::domain::DBPF_RESOURCE_SIGNATURE_VERSION
        );
    }
}
