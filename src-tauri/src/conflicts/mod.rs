use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{Display, Formatter},
    path::{Component, Path},
};

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::{
    dbpf::{self, ResourceKey},
    fingerprint::{self, ExactDuplicateGroup},
};

const MAX_RESOURCE_OVERLAP_PAIRS: usize = 10_000;
const MAX_RESOURCE_KEY_SAMPLES: usize = 8;

#[derive(Debug, Clone)]
struct PackageFile {
    local_file_id: i64,
    relative_path: String,
}

#[derive(Debug)]
struct ResourceOverlapAccumulator {
    left: PackageFile,
    right: PackageFile,
    shared_resource_count: u64,
    sample_keys: Vec<ResourceKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceOverlapFinding {
    pub(crate) classification: String,
    pub(crate) left_file_id: i64,
    pub(crate) left_relative_path: String,
    pub(crate) right_file_id: i64,
    pub(crate) right_relative_path: String,
    pub(crate) shared_resource_count: u64,
    pub(crate) sample_resource_keys: Vec<ResourceKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageParseFailure {
    pub(crate) local_file_id: i64,
    pub(crate) relative_path: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalConflictAnalysis {
    pub(crate) exact_duplicates: Vec<ExactDuplicateGroup>,
    pub(crate) resource_overlaps: Vec<ResourceOverlapFinding>,
    pub(crate) parse_failures: Vec<PackageParseFailure>,
    pub(crate) overlap_pairs_truncated: bool,
}

#[derive(Debug)]
pub(crate) enum ConflictAnalysisError {
    Sqlite(rusqlite::Error),
}

impl Display for ConflictAnalysisError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "local conflict query failed: {error}"),
        }
    }
}

impl Error for ConflictAnalysisError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
        }
    }
}

impl From<rusqlite::Error> for ConflictAnalysisError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

pub(crate) fn analyze_installation(
    connection: &Connection,
    installation_id: i64,
    mods_root: &Path,
) -> Result<LocalConflictAnalysis, ConflictAnalysisError> {
    let exact_duplicates =
        fingerprint::exact_duplicate_groups(connection, installation_id)?;
    let package_files = load_package_files(connection, installation_id)?;

    let mut resource_owners: BTreeMap<ResourceKey, Vec<PackageFile>> = BTreeMap::new();
    let mut overlap_pairs: BTreeMap<(i64, i64), ResourceOverlapAccumulator> =
        BTreeMap::new();
    let mut parse_failures = Vec::new();
    let mut overlap_pairs_truncated = false;

    for package in package_files {
        if !safe_relative_path(&package.relative_path) {
            parse_failures.push(PackageParseFailure {
                local_file_id: package.local_file_id,
                relative_path: package.relative_path,
                error: "unsafe relative package path".to_string(),
            });
            continue;
        }

        let full_path = mods_root.join(&package.relative_path);
        let metadata = match dbpf::parse_path(&full_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                parse_failures.push(PackageParseFailure {
                    local_file_id: package.local_file_id,
                    relative_path: package.relative_path,
                    error: error.to_string(),
                });
                continue;
            }
        };

        let keys = metadata.resource_keys().collect::<BTreeSet<_>>();
        for key in keys {
            if let Some(previous_owners) = resource_owners.get(&key) {
                for previous in previous_owners {
                    let (left, right) = ordered_pair(previous, &package);
                    let pair_key = (left.local_file_id, right.local_file_id);

                    if !overlap_pairs.contains_key(&pair_key)
                        && overlap_pairs.len() >= MAX_RESOURCE_OVERLAP_PAIRS
                    {
                        overlap_pairs_truncated = true;
                        continue;
                    }

                    let accumulator =
                        overlap_pairs
                            .entry(pair_key)
                            .or_insert_with(|| ResourceOverlapAccumulator {
                                left: left.clone(),
                                right: right.clone(),
                                shared_resource_count: 0,
                                sample_keys: Vec::new(),
                            });

                    accumulator.shared_resource_count += 1;
                    if accumulator.sample_keys.len() < MAX_RESOURCE_KEY_SAMPLES {
                        accumulator.sample_keys.push(key);
                    }
                }
            }

            resource_owners.entry(key).or_default().push(package.clone());
        }
    }

    let mut resource_overlaps = overlap_pairs
        .into_values()
        .map(|item| ResourceOverlapFinding {
            classification: "potentialConflict".to_string(),
            left_file_id: item.left.local_file_id,
            left_relative_path: item.left.relative_path,
            right_file_id: item.right.local_file_id,
            right_relative_path: item.right.relative_path,
            shared_resource_count: item.shared_resource_count,
            sample_resource_keys: item.sample_keys,
        })
        .collect::<Vec<_>>();

    resource_overlaps.sort_by(|left, right| {
        (
            &left.left_relative_path,
            &left.right_relative_path,
            left.left_file_id,
            left.right_file_id,
        )
            .cmp(&(
                &right.left_relative_path,
                &right.right_relative_path,
                right.left_file_id,
                right.right_file_id,
            ))
    });

    parse_failures.sort_by(|left, right| {
        (&left.relative_path, left.local_file_id)
            .cmp(&(&right.relative_path, right.local_file_id))
    });

    Ok(LocalConflictAnalysis {
        exact_duplicates,
        resource_overlaps,
        parse_failures,
        overlap_pairs_truncated,
    })
}

fn load_package_files(
    connection: &Connection,
    installation_id: i64,
) -> Result<Vec<PackageFile>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT id, relative_path
         FROM local_files
         WHERE installation_id = ?1
           AND file_kind = 'package'
           AND enabled = 1
         ORDER BY relative_path, id",
    )?;

    statement
        .query_map(params![installation_id], |row| {
            Ok(PackageFile {
                local_file_id: row.get(0)?,
                relative_path: row.get(1)?,
            })
        })?
        .collect()
}

fn safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn ordered_pair<'a>(
    first: &'a PackageFile,
    second: &'a PackageFile,
) -> (&'a PackageFile, &'a PackageFile) {
    if (first.relative_path.as_str(), first.local_file_id)
        <= (second.relative_path.as_str(), second.local_file_id)
    {
        (first, second)
    } else {
        (second, first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dbpf::ResourceKey, fingerprint::FingerprintRepository, storage};
    use tempfile::TempDir;

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

    fn insert_file(
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
            .expect("insert local package");
        connection.last_insert_rowid()
    }

    #[test]
    fn exact_duplicates_and_resource_overlaps_are_separate_findings() {
        let temp = TempDir::new().expect("create conflict temp directory");
        let mods_root = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_root).expect("create Mods directory");
        let database_path = temp.path().join("conflicts.sqlite3");
        storage::initialize(&database_path).expect("initialize local database");
        let connection = storage::open(&database_path).expect("open local database");

        connection
            .execute(
                "INSERT INTO installations (
                    game_root, mods_root, platform, discovered_at, last_seen_at
                 )
                 VALUES ('game', ?1, 'windows', 'now', 'now')",
                [mods_root.to_string_lossy().to_string()],
            )
            .expect("insert installation");
        let installation_id = connection.last_insert_rowid();

        let shared = ResourceKey {
            resource_type: 1,
            group: 2,
            instance: 3,
        };
        let second = ResourceKey {
            resource_type: 4,
            group: 5,
            instance: 6,
        };
        let third = ResourceKey {
            resource_type: 7,
            group: 8,
            instance: 9,
        };

        let first_bytes = minimal_dbpf(&[shared, second]);
        let third_bytes = minimal_dbpf(&[shared, third]);
        std::fs::write(mods_root.join("a.package"), &first_bytes)
            .expect("write first package");
        std::fs::write(mods_root.join("b.package"), &first_bytes)
            .expect("write exact duplicate package");
        std::fs::write(mods_root.join("c.package"), &third_bytes)
            .expect("write overlapping package");

        let first_id = insert_file(&connection, installation_id, "a.package");
        let second_id = insert_file(&connection, installation_id, "b.package");
        let third_id = insert_file(&connection, installation_id, "c.package");

        let fingerprints = FingerprintRepository::new(&connection);
        fingerprints
            .store_sha256(first_id, "same")
            .expect("store first SHA");
        fingerprints
            .store_sha256(second_id, "same")
            .expect("store second SHA");
        fingerprints
            .store_sha256(third_id, "different")
            .expect("store third SHA");

        let analysis = analyze_installation(&connection, installation_id, &mods_root)
            .expect("analyze local conflicts");

        assert_eq!(analysis.exact_duplicates.len(), 1);
        assert_eq!(analysis.exact_duplicates[0].files.len(), 2);
        assert_eq!(analysis.resource_overlaps.len(), 3);
        assert!(analysis
            .resource_overlaps
            .iter()
            .all(|finding| finding.classification == "potentialConflict"));

        let exact_pair_overlap = analysis
            .resource_overlaps
            .iter()
            .find(|finding| {
                BTreeSet::from([finding.left_file_id, finding.right_file_id])
                    == BTreeSet::from([first_id, second_id])
            })
            .expect("exact duplicate pair also has resource overlap");
        assert_eq!(exact_pair_overlap.shared_resource_count, 2);

        let third_overlaps = analysis
            .resource_overlaps
            .iter()
            .filter(|finding| {
                finding.left_file_id == third_id || finding.right_file_id == third_id
            })
            .collect::<Vec<_>>();
        assert_eq!(third_overlaps.len(), 2);
        assert!(third_overlaps
            .iter()
            .all(|finding| finding.shared_resource_count == 1));
        assert!(!analysis.overlap_pairs_truncated);
        assert!(analysis.parse_failures.is_empty());
    }

    #[test]
    fn malformed_package_is_reported_without_aborting_other_results() {
        let temp = TempDir::new().expect("create malformed conflict temp directory");
        let mods_root = temp.path().join("Mods");
        std::fs::create_dir_all(&mods_root).expect("create Mods directory");
        std::fs::write(mods_root.join("bad.package"), b"not-dbpf")
            .expect("write malformed package");

        let database_path = temp.path().join("malformed.sqlite3");
        storage::initialize(&database_path).expect("initialize local database");
        let connection = storage::open(&database_path).expect("open local database");
        connection
            .execute(
                "INSERT INTO installations (
                    game_root, mods_root, platform, discovered_at, last_seen_at
                 )
                 VALUES ('game', ?1, 'windows', 'now', 'now')",
                [mods_root.to_string_lossy().to_string()],
            )
            .expect("insert installation");
        let installation_id = connection.last_insert_rowid();
        insert_file(&connection, installation_id, "bad.package");

        let analysis = analyze_installation(&connection, installation_id, &mods_root)
            .expect("analysis survives malformed package");

        assert_eq!(analysis.parse_failures.len(), 1);
        assert_eq!(analysis.parse_failures[0].relative_path, "bad.package");
        assert!(analysis.resource_overlaps.is_empty());
    }
}
