use rusqlite::{params, Connection};
use serde::Serialize;

use super::domain::SHA256_ALGORITHM_VERSION;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuplicateFile {
    pub(crate) local_file_id: i64,
    pub(crate) relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExactDuplicateGroup {
    pub(crate) sha256: String,
    pub(crate) files: Vec<DuplicateFile>,
}

pub(crate) struct ExactDuplicateQuery<'a> {
    connection: &'a Connection,
}

impl<'a> ExactDuplicateQuery<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn for_installation(
        &self,
        installation_id: i64,
    ) -> Result<Vec<ExactDuplicateGroup>, rusqlite::Error> {
        let mut statement = self.connection.prepare(
            "SELECT f.value, lf.id, lf.relative_path
             FROM fingerprints f
             JOIN local_files lf ON lf.id = f.local_file_id
             WHERE lf.installation_id = ?1
               AND f.kind = 'sha256'
               AND f.algorithm_version = ?2
               AND f.value IN (
                    SELECT duplicate.value
                    FROM fingerprints duplicate
                    JOIN local_files duplicate_file
                      ON duplicate_file.id = duplicate.local_file_id
                    WHERE duplicate_file.installation_id = ?1
                      AND duplicate.kind = 'sha256'
                      AND duplicate.algorithm_version = ?2
                    GROUP BY duplicate.value
                    HAVING COUNT(*) > 1
               )
             ORDER BY f.value, lf.relative_path, lf.id",
        )?;

        let rows = statement.query_map(
            params![installation_id, SHA256_ALGORITHM_VERSION],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    DuplicateFile {
                        local_file_id: row.get(1)?,
                        relative_path: row.get(2)?,
                    },
                ))
            },
        )?;

        let mut groups: Vec<ExactDuplicateGroup> = Vec::new();

        for row in rows {
            let (sha256, file) = row?;

            if groups
                .last()
                .is_none_or(|group| group.sha256 != sha256)
            {
                groups.push(ExactDuplicateGroup {
                    sha256: sha256.clone(),
                    files: Vec::new(),
                });
            }

            groups
                .last_mut()
                .expect("duplicate group exists after insertion")
                .files
                .push(file);
        }

        Ok(groups)
    }}

pub(crate) fn exact_duplicate_groups(
    connection: &Connection,
    installation_id: i64,
) -> Result<Vec<ExactDuplicateGroup>, rusqlite::Error> {
    ExactDuplicateQuery::new(connection).for_installation(installation_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fingerprint::repository::FingerprintRepository, storage};
    use std::time::Instant;
    use tempfile::TempDir;

    fn database_fixture() -> (TempDir, Connection, i64) {
        let temp = TempDir::new().expect("create duplicate database directory");
        let database_path = temp.path().join("duplicates.sqlite3");
        storage::initialize(&database_path).expect("initialize duplicate database");
        let connection = storage::open(&database_path).expect("open duplicate database");

        connection
            .execute(
                "INSERT INTO installations (
                    game_root, mods_root, platform, discovered_at, last_seen_at
                 )
                 VALUES ('game', 'mods', 'windows', 'now', 'now')",
                [],
            )
            .expect("insert installation");

        (temp, connection, 1)
    }

    fn insert_local_file(
        connection: &Connection,
        installation_id: i64,
        relative_path: &str,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO local_files (
                    installation_id, relative_path, file_kind
                 )
                 VALUES (?1, ?2, 'package')",
                params![installation_id, relative_path],
            )
            .expect("insert local file");
        connection.last_insert_rowid()
    }

    #[test]
    fn exact_duplicates_are_grouped_by_hash_not_path() {
        let (_temp, connection, installation_id) = database_fixture();
        let repository = FingerprintRepository::new(&connection);
        let first = insert_local_file(&connection, installation_id, "A/alpha.package");
        let second = insert_local_file(&connection, installation_id, "B/beta.package");
        let third = insert_local_file(&connection, installation_id, "C/alpha.package");

        repository.store_sha256(first, "same").expect("store first hash");
        repository.store_sha256(second, "same").expect("store second hash");
        repository
            .store_sha256(third, "different")
            .expect("store third hash");

        let groups = ExactDuplicateQuery::new(&connection)
            .for_installation(installation_id)
            .expect("query duplicate groups");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].sha256, "same");
        assert_eq!(
            groups[0]
                .files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["A/alpha.package", "B/beta.package"]
        );
    }

    #[test]
    fn same_filename_with_different_hashes_is_not_duplicate() {
        let (_temp, connection, installation_id) = database_fixture();
        let repository = FingerprintRepository::new(&connection);
        let left = insert_local_file(&connection, installation_id, "CreatorA/shared.package");
        let right = insert_local_file(&connection, installation_id, "CreatorB/shared.package");

        repository.store_sha256(left, "first").expect("store left hash");
        repository.store_sha256(right, "second").expect("store right hash");

        assert!(ExactDuplicateQuery::new(&connection)
            .for_installation(installation_id)
            .expect("query duplicate groups")
            .is_empty());
    }

    #[test]
    #[ignore = "large-library benchmark; run in dedicated scanner benchmark workflow"]
    fn benchmark_5000_file_duplicate_grouping() {
        let (_db_temp, mut connection, installation_id) = database_fixture();
        let transaction = connection
            .transaction()
            .expect("start benchmark transaction");
        let repository = FingerprintRepository::new(&transaction);

        for index in 0..5_000_i64 {
            transaction
                .execute(
                    "INSERT INTO local_files (
                        installation_id, relative_path, file_kind
                     )
                     VALUES (?1, ?2, 'package')",
                    params![
                        installation_id,
                        format!("Creator{:02}/item-{index:05}.package", index % 25)
                    ],
                )
                .expect("insert benchmark local file");
            let local_file_id = transaction.last_insert_rowid();
            let hash = if index < 200 {
                format!("duplicate-{:03}", index / 2)
            } else {
                format!("unique-{index:05}")
            };
            repository
                .store_sha256(local_file_id, &hash)
                .expect("store benchmark fingerprint");
        }

        transaction.commit().expect("commit benchmark fixture");

        let started = Instant::now();
        let groups = ExactDuplicateQuery::new(&connection)
            .for_installation(installation_id)
            .expect("group benchmark duplicates");
        let elapsed = started.elapsed();

        eprintln!(
            "FINGERPRINT_BENCHMARK files=5000 duplicate_groups={} grouping_ms={}",
            groups.len(),
            elapsed.as_millis()
        );

        assert_eq!(groups.len(), 100);
        assert!(groups.iter().all(|group| group.files.len() == 2));
    }
}
