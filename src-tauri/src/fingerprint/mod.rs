use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{dbpf, ts4script};

const HASH_BUFFER_SIZE: usize = 64 * 1024;

pub(crate) const SHA256_ALGORITHM_VERSION: &str = "sha256-v1";
pub(crate) const DBPF_RESOURCE_SIGNATURE_VERSION: &str = "dbpf-resource-keys-v1";
pub(crate) const TS4SCRIPT_SIGNATURE_VERSION: &str = "ts4script-python-identity-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FingerprintKind {
    Sha256,
    CurseForge,
    ResourceSignature,
    ScriptSignature,
    Quick,
}

impl FingerprintKind {
    pub(crate) fn database_value(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::CurseForge => "curseforge",
            Self::ResourceSignature => "resource_signature",
            Self::ScriptSignature => "script_signature",
            Self::Quick => "quick",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactKind {
    Package,
    Ts4Script,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FingerprintValue {
    pub(crate) kind: FingerprintKind,
    pub(crate) value: String,
    pub(crate) algorithm_version: &'static str,
}

pub(crate) trait FingerprintProvider {
    fn kind(&self) -> FingerprintKind;
    fn algorithm_version(&self) -> &'static str;
    fn compute(&self, path: &Path) -> Result<Option<String>, FingerprintError>;

    fn fingerprint(&self, path: &Path) -> Result<Option<FingerprintValue>, FingerprintError> {
        Ok(self.compute(path)?.map(|value| FingerprintValue {
            kind: self.kind(),
            value,
            algorithm_version: self.algorithm_version(),
        }))
    }
}

#[derive(Debug, Default)]
pub(crate) struct DbpfResourceSignatureProvider;

impl FingerprintProvider for DbpfResourceSignatureProvider {
    fn kind(&self) -> FingerprintKind {
        FingerprintKind::ResourceSignature
    }

    fn algorithm_version(&self) -> &'static str {
        DBPF_RESOURCE_SIGNATURE_VERSION
    }

    fn compute(&self, path: &Path) -> Result<Option<String>, FingerprintError> {
        let metadata = dbpf::parse_path(path)?;
        let mut keys = metadata.resource_keys().collect::<Vec<_>>();
        keys.sort_unstable();
        keys.dedup();

        let mut hasher = Sha256::new();
        hasher.update(b"sims-mod-health:dbpf-resource-signature:v1\0");
        hasher.update((keys.len() as u64).to_be_bytes());

        for key in keys {
            hasher.update(key.resource_type.to_be_bytes());
            hasher.update(key.group.to_be_bytes());
            hasher.update(key.instance.to_be_bytes());
        }

        Ok(Some(format!("{:x}", hasher.finalize())))
    }
}

#[derive(Debug, Default)]
pub(crate) struct Ts4ScriptIdentityProvider;

impl FingerprintProvider for Ts4ScriptIdentityProvider {
    fn kind(&self) -> FingerprintKind {
        FingerprintKind::ScriptSignature
    }

    fn algorithm_version(&self) -> &'static str {
        TS4SCRIPT_SIGNATURE_VERSION
    }

    fn compute(&self, path: &Path) -> Result<Option<String>, FingerprintError> {
        let metadata = ts4script::inspect_path(path)?;
        let modules = metadata.module_names.into_iter().collect::<BTreeSet<_>>();
        let packages = metadata.package_names.into_iter().collect::<BTreeSet<_>>();

        if modules.is_empty() && packages.is_empty() {
            return Ok(None);
        }

        let mut hasher = Sha256::new();
        hasher.update(b"sims-mod-health:ts4script-python-identity:v1\0");
        hash_string_set(&mut hasher, b"modules\0", &modules)?;
        hash_string_set(&mut hasher, b"packages\0", &packages)?;

        Ok(Some(format!("{:x}", hasher.finalize())))
    }
}

fn hash_string_set(
    hasher: &mut Sha256,
    domain: &[u8],
    values: &BTreeSet<String>,
) -> Result<(), FingerprintError> {
    hasher.update(domain);
    hasher.update((values.len() as u64).to_be_bytes());

    for value in values {
        let bytes = value.as_bytes();
        let length = u32::try_from(bytes.len())
            .map_err(|_| FingerprintError::ValueTooLarge("fingerprint identity component"))?;
        hasher.update(length.to_be_bytes());
        hasher.update(bytes);
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) enum FingerprintError {
    Io(io::Error),
    Sqlite(rusqlite::Error),
    Dbpf(dbpf::DbpfError),
    Ts4Script(ts4script::Ts4ScriptError),
    ValueTooLarge(&'static str),
}

impl Display for FingerprintError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "fingerprint I/O error: {error}"),
            Self::Sqlite(error) => write!(formatter, "fingerprint SQLite error: {error}"),
            Self::Dbpf(error) => write!(formatter, "DBPF fingerprint failed: {error}"),
            Self::Ts4Script(error) => write!(formatter, "TS4Script fingerprint failed: {error}"),
            Self::ValueTooLarge(context) => {
                write!(formatter, "{context} is too large to fingerprint safely")
            }
        }
    }
}

impl std::error::Error for FingerprintError {}

impl From<io::Error> for FingerprintError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for FingerprintError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<dbpf::DbpfError> for FingerprintError {
    fn from(value: dbpf::DbpfError) -> Self {
        Self::Dbpf(value)
    }
}

impl From<ts4script::Ts4ScriptError> for FingerprintError {
    fn from(value: ts4script::Ts4ScriptError) -> Self {
        Self::Ts4Script(value)
    }
}

pub(crate) fn sha256_file<F>(path: &Path, mut is_cancelled: F) -> Result<Option<String>, io::Error>
where
    F: FnMut() -> bool,
{
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_SIZE];

    loop {
        if is_cancelled() {
            return Ok(None);
        }

        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(Some(format!("{:x}", hasher.finalize())))
}

pub(crate) fn delete_cached_fingerprints(
    connection: &Connection,
    local_file_id: i64,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "DELETE FROM fingerprints
         WHERE local_file_id = ?1
           AND kind IN ('sha256', 'resource_signature', 'script_signature', 'quick')",
        [local_file_id],
    )?;
    Ok(())
}

pub(crate) fn store_fingerprint(
    connection: &Connection,
    local_file_id: i64,
    fingerprint: &FingerprintValue,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "INSERT INTO fingerprints (
            local_file_id,
            kind,
            value,
            algorithm_version,
            computed_at
         )
         VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT(local_file_id, kind, value) DO UPDATE SET
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
    connection: &Connection,
    local_file_id: i64,
    value: &str,
) -> Result<(), rusqlite::Error> {
    store_fingerprint(
        connection,
        local_file_id,
        &FingerprintValue {
            kind: FingerprintKind::Sha256,
            value: value.to_string(),
            algorithm_version: SHA256_ALGORITHM_VERSION,
        },
    )
}

pub(crate) fn compute_structural_fingerprint(
    kind: ArtifactKind,
    path: &Path,
) -> Result<Option<FingerprintValue>, FingerprintError> {
    match kind {
        ArtifactKind::Package => DbpfResourceSignatureProvider.fingerprint(path),
        ArtifactKind::Ts4Script => Ts4ScriptIdentityProvider.fingerprint(path),
    }
}

pub(crate) fn refresh_structural_fingerprint(
    connection: &Connection,
    local_file_id: i64,
    kind: ArtifactKind,
    path: &Path,
) -> Result<Option<FingerprintValue>, FingerprintError> {
    let fingerprint_kind = match kind {
        ArtifactKind::Package => FingerprintKind::ResourceSignature,
        ArtifactKind::Ts4Script => FingerprintKind::ScriptSignature,
    };

    connection.execute(
        "DELETE FROM fingerprints WHERE local_file_id = ?1 AND kind = ?2",
        params![local_file_id, fingerprint_kind.database_value()],
    )?;

    let fingerprint = compute_structural_fingerprint(kind, path)?;
    if let Some(value) = &fingerprint {
        store_fingerprint(connection, local_file_id, value)?;
    }

    Ok(fingerprint)
}

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

pub(crate) fn exact_duplicate_groups(
    connection: &Connection,
    installation_id: i64,
) -> Result<Vec<ExactDuplicateGroup>, rusqlite::Error> {
    let mut hashes = connection.prepare(
        "SELECT f.value
         FROM fingerprints f
         JOIN local_files lf ON lf.id = f.local_file_id
         WHERE lf.installation_id = ?1
           AND f.kind = 'sha256'
           AND f.algorithm_version = ?2
         GROUP BY f.value
         HAVING COUNT(*) > 1
         ORDER BY f.value",
    )?;

    let duplicate_hashes = hashes
        .query_map(params![installation_id, SHA256_ALGORITHM_VERSION], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut groups = Vec::with_capacity(duplicate_hashes.len());

    for sha256 in duplicate_hashes {
        let mut files = connection.prepare(
            "SELECT lf.id, lf.relative_path
             FROM local_files lf
             JOIN fingerprints f ON f.local_file_id = lf.id
             WHERE lf.installation_id = ?1
               AND f.kind = 'sha256'
               AND f.algorithm_version = ?2
               AND f.value = ?3
             ORDER BY lf.relative_path, lf.id",
        )?;

        let members = files
            .query_map(
                params![installation_id, SHA256_ALGORITHM_VERSION, sha256],
                |row| {
                    Ok(DuplicateFile {
                        local_file_id: row.get(0)?,
                        relative_path: row.get(1)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        groups.push(ExactDuplicateGroup {
            sha256,
            files: members,
        });
    }

    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;
    use std::{
        io::{Cursor, Write},
        time::Instant,
    };
    use tempfile::TempDir;
    use zip::{
        write::{SimpleFileOptions, ZipWriter},
        CompressionMethod,
    };

    fn write_bytes(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent");
        }
        std::fs::write(path, bytes).expect("write fingerprint fixture");
    }

    fn sha(path: &Path) -> String {
        sha256_file(path, || false)
            .expect("hash fixture")
            .expect("hash not cancelled")
    }

    fn minimal_dbpf(keys: &[dbpf::ResourceKey]) -> Vec<u8> {
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

    fn ts4script(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::DEFLATE);

        for (name, content) in entries {
            writer.start_file(*name, options).expect("start ZIP entry");
            writer.write_all(content).expect("write ZIP entry");
        }

        writer.finish().expect("finish TS4Script").into_inner()
    }

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
    fn identical_bytes_hash_identically_across_paths() {
        let temp = TempDir::new().expect("create hash temp directory");
        let left = temp.path().join("A/one.package");
        let right = temp.path().join("B/two.package");
        write_bytes(&left, b"same artifact bytes");
        write_bytes(&right, b"same artifact bytes");

        assert_eq!(sha(&left), sha(&right));
    }

    #[test]
    fn same_filename_with_different_content_is_not_an_exact_duplicate() {
        let temp = TempDir::new().expect("create hash temp directory");
        let left = temp.path().join("CreatorA/shared.package");
        let right = temp.path().join("CreatorB/shared.package");
        write_bytes(&left, b"first");
        write_bytes(&right, b"second");

        let (_db_temp, connection, installation_id) = database_fixture();
        let left_id = insert_local_file(&connection, installation_id, "CreatorA/shared.package");
        let right_id = insert_local_file(&connection, installation_id, "CreatorB/shared.package");
        store_sha256(&connection, left_id, &sha(&left)).expect("store left hash");
        store_sha256(&connection, right_id, &sha(&right)).expect("store right hash");

        assert!(exact_duplicate_groups(&connection, installation_id)
            .expect("query duplicate groups")
            .is_empty());
    }

    #[test]
    fn exact_duplicates_are_grouped_by_hash_not_path() {
        let (_db_temp, connection, installation_id) = database_fixture();
        let first = insert_local_file(&connection, installation_id, "A/alpha.package");
        let second = insert_local_file(&connection, installation_id, "B/beta.package");
        let third = insert_local_file(&connection, installation_id, "C/alpha.package");

        store_sha256(&connection, first, "same").expect("store first hash");
        store_sha256(&connection, second, "same").expect("store second hash");
        store_sha256(&connection, third, "different").expect("store third hash");

        let groups =
            exact_duplicate_groups(&connection, installation_id).expect("query duplicate groups");

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
    fn dbpf_resource_signature_is_independent_of_index_order() {
        let temp = TempDir::new().expect("create DBPF signature temp directory");
        let a = dbpf::ResourceKey {
            resource_type: 0x1000,
            group: 0x2000,
            instance: 0x3000,
        };
        let b = dbpf::ResourceKey {
            resource_type: 0x1001,
            group: 0x2001,
            instance: 0x3001,
        };
        let left = temp.path().join("left.package");
        let right = temp.path().join("right.package");
        write_bytes(&left, &minimal_dbpf(&[a, b]));
        write_bytes(&right, &minimal_dbpf(&[b, a]));

        let provider = DbpfResourceSignatureProvider;
        assert_eq!(
            provider.compute(&left).expect("left resource signature"),
            provider.compute(&right).expect("right resource signature")
        );
    }

    #[test]
    fn ts4script_identity_signature_ignores_archive_entry_order_and_version_hint() {
        let temp = TempDir::new().expect("create TS4Script signature temp directory");
        let left = temp.path().join("left.ts4script");
        let right = temp.path().join("right.ts4script");
        write_bytes(
            &left,
            &ts4script(&[
                ("creator/mod/core.pyc", b"compiled-a"),
                ("creator/mod/__init__.pyc", b"init"),
                ("manifest.txt", b"version = 1.0.0"),
            ]),
        );
        write_bytes(
            &right,
            &ts4script(&[
                ("manifest.txt", b"version = 2.0.0"),
                ("creator/mod/__init__.pyc", b"different-init"),
                ("creator/mod/core.pyc", b"different-bytecode"),
            ]),
        );

        let provider = Ts4ScriptIdentityProvider;
        assert_eq!(
            provider.compute(&left).expect("left script signature"),
            provider.compute(&right).expect("right script signature")
        );
    }

    #[test]
    fn cached_structural_fingerprint_is_invalidated_with_file_identity() {
        let (_db_temp, connection, installation_id) = database_fixture();
        let local_file_id = insert_local_file(&connection, installation_id, "A/mod.package");

        store_sha256(&connection, local_file_id, "old-sha").expect("store SHA");
        store_fingerprint(
            &connection,
            local_file_id,
            &FingerprintValue {
                kind: FingerprintKind::ResourceSignature,
                value: "old-resource".to_string(),
                algorithm_version: DBPF_RESOURCE_SIGNATURE_VERSION,
            },
        )
        .expect("store resource signature");

        delete_cached_fingerprints(&connection, local_file_id)
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
    fn structural_fingerprint_persists_with_algorithm_version() {
        let temp = TempDir::new().expect("create structural temp directory");
        let path = temp.path().join("mod.package");
        write_bytes(
            &path,
            &minimal_dbpf(&[dbpf::ResourceKey {
                resource_type: 1,
                group: 2,
                instance: 3,
            }]),
        );

        let (_db_temp, connection, installation_id) = database_fixture();
        let local_file_id = insert_local_file(&connection, installation_id, "mod.package");

        let fingerprint = refresh_structural_fingerprint(
            &connection,
            local_file_id,
            ArtifactKind::Package,
            &path,
        )
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
        assert_eq!(persisted.1, DBPF_RESOURCE_SIGNATURE_VERSION);
    }

    #[test]
    #[ignore = "large-library benchmark; run in dedicated scanner benchmark workflow"]
    fn benchmark_5000_file_duplicate_grouping() {
        let (_db_temp, mut connection, installation_id) = database_fixture();
        let transaction = connection
            .transaction()
            .expect("start benchmark transaction");

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
            store_sha256(&transaction, local_file_id, &hash).expect("store benchmark fingerprint");
        }

        transaction.commit().expect("commit benchmark fixture");

        let started = Instant::now();
        let groups = exact_duplicate_groups(&connection, installation_id)
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
