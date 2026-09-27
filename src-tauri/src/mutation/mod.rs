use std::{
    collections::{BTreeSet, HashMap, HashSet},
    error::Error,
    fmt::{Display, Formatter},
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::{redirect::Policy, Client, Url};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    fingerprint,
    registry::{
        ArtifactResolution, RegistryArtifactProbe, RegistryClient, RegistryFingerprintProbe,
        RelationshipResponse,
    },
    storage::{self, StorageError},
};

const MAX_DOWNLOAD_BYTES: u64 = 256 * 1024 * 1024;
const DEFAULT_REGISTRY_URL: &str = "http://127.0.0.1:8000";

pub(crate) const BULK_UPDATE_ENABLED: bool = false;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyUpdateRequest {
    pub(crate) installation_id: i64,
    pub(crate) target_relative_path: String,
    pub(crate) source_kind: String,
    pub(crate) source_url: String,
    pub(crate) current_release_id: String,
    pub(crate) replacement_release_id: String,
    pub(crate) expected_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateEventView {
    pub(crate) created_at: String,
    pub(crate) phase: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateTransactionView {
    pub(crate) transaction_id: i64,
    pub(crate) restore_point_id: i64,
    pub(crate) status: String,
    pub(crate) target_relative_path: String,
    pub(crate) source_kind: String,
    pub(crate) source_url: String,
    pub(crate) current_release_id: String,
    pub(crate) replacement_release_id: String,
    pub(crate) expected_sha256: Option<String>,
    pub(crate) observed_sha256: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) error: Option<String>,
    pub(crate) events: Vec<UpdateEventView>,
}

#[derive(Debug, Clone)]
struct PreparedTransaction {
    id: i64,
    restore_point_id: i64,
    installation_id: i64,
    mods_root: PathBuf,
    backup_root: PathBuf,
    staging_root: PathBuf,
    target_relative_path: PathBuf,
    original_sha256: String,
    expected_sha256: Option<String>,
}

#[derive(Debug, Clone)]
struct StoredTransaction {
    id: i64,
    restore_point_id: i64,
    installation_id: i64,
    mods_root: PathBuf,
    backup_root: PathBuf,
    target_relative_path: PathBuf,
    original_sha256: String,
    observed_sha256: Option<String>,
    status: String,
}

#[derive(Debug)]
pub(crate) enum MutationError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Storage(StorageError),
    Registry(crate::registry::RegistryError),
    Http(reqwest::Error),
    InvalidTarget(String),
    InvalidSource(String),
    InvalidHash(String),
    MissingInstallation(i64),
    TargetNotIndexed(String),
    TargetChanged,
    RestorePointRequired,
    MissingTransaction(i64),
    InvalidTransactionState(String),
    DownloadTooLarge,
    IntegrityMismatch { expected: String, observed: String },
    DependencyGraphIncomplete { unresolved_files: usize },
    CurrentReleaseNotInstalled(String),
    DependencyRegression(String),
}

impl Display for MutationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "mutation I/O error: {error}"),
            Self::Sqlite(error) => write!(formatter, "mutation SQLite error: {error}"),
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::Registry(error) => write!(formatter, "{error}"),
            Self::Http(error) => write!(formatter, "update download failed: {error}"),
            Self::InvalidTarget(reason) => write!(formatter, "unsafe update target: {reason}"),
            Self::InvalidSource(reason) => write!(formatter, "update source rejected: {reason}"),
            Self::InvalidHash(reason) => write!(formatter, "invalid expected SHA-256: {reason}"),
            Self::MissingInstallation(id) => write!(formatter, "installation {id} does not exist"),
            Self::TargetNotIndexed(path) => {
                write!(formatter, "target is not an enabled indexed mod file: {path}")
            }
            Self::TargetChanged => write!(
                formatter,
                "target changed after the restore point was created; update was stopped"
            ),
            Self::RestorePointRequired => write!(
                formatter,
                "mutation refused because the restore point is not ready"
            ),
            Self::MissingTransaction(id) => write!(formatter, "update transaction {id} not found"),
            Self::InvalidTransactionState(status) => {
                write!(formatter, "update transaction is not recoverable from state {status}")
            }
            Self::DownloadTooLarge => write!(
                formatter,
                "download exceeds the 256 MiB staged-update limit"
            ),
            Self::IntegrityMismatch { expected, observed } => write!(
                formatter,
                "download SHA-256 mismatch: expected {expected}, observed {observed}"
            ),
            Self::DependencyGraphIncomplete { unresolved_files } => write!(
                formatter,
                "dependency safety check is incomplete because {unresolved_files} enabled files are not deterministically resolved"
            ),
            Self::CurrentReleaseNotInstalled(release_id) => write!(
                formatter,
                "current release {release_id} is not deterministically resolved in the active installation"
            ),
            Self::DependencyRegression(detail) => write!(
                formatter,
                "replacement would introduce a dependency or incompatibility regression: {detail}"
            ),
        }
    }
}

impl Error for MutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Registry(error) => Some(error),
            Self::Http(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MutationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for MutationError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<StorageError> for MutationError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<crate::registry::RegistryError> for MutationError {
    fn from(value: crate::registry::RegistryError) -> Self {
        Self::Registry(value)
    }
}

impl From<reqwest::Error> for MutationError {
    fn from(value: reqwest::Error) -> Self {
        Self::Http(value)
    }
}

pub(crate) async fn apply_update(
    database_path: &Path,
    app_data_dir: &Path,
    request: ApplyUpdateRequest,
) -> Result<UpdateTransactionView, MutationError> {
    let prepared = prepare_transaction(database_path, app_data_dir, &request)?;

    if let Err(error) = download_and_stage(database_path, &prepared, &request).await {
        mark_failed(
            database_path,
            prepared.id,
            &error_detail_for_journal(&error),
        )?;
        return Err(error);
    }

    if let Err(error) = verify_dependency_safety(database_path, &prepared, &request).await {
        mark_failed(
            database_path,
            prepared.id,
            &error_detail_for_journal(&error),
        )?;
        return Err(error);
    }

    if let Err(error) = install_staged(database_path, &prepared) {
        mark_interrupted(database_path, prepared.id, &error.to_string())?;
        return Err(error);
    }

    load_transaction(database_path, prepared.id)
}

pub(crate) fn rollback_update(
    database_path: &Path,
    app_data_dir: &Path,
    transaction_id: i64,
) -> Result<UpdateTransactionView, MutationError> {
    let stored = load_stored_transaction(database_path, transaction_id)?;
    let restore_root = app_data_dir.join("restore-points");
    ensure_existing_directory_within(&restore_root, &stored.backup_root)?;

    if !matches!(
        stored.status.as_str(),
        "completed" | "interrupted" | "failed" | "installed" | "validated" | "archived"
    ) {
        return Err(MutationError::InvalidTransactionState(stored.status));
    }

    let target = checked_target_path(&stored.mods_root, &stored.target_relative_path)?;
    let backup = checked_backup_path(&stored.backup_root, &stored.target_relative_path)?;

    let backup_sha = hash_file(&backup)?;
    if backup_sha != stored.original_sha256 {
        return Err(MutationError::IntegrityMismatch {
            expected: stored.original_sha256,
            observed: backup_sha,
        });
    }

    if target.exists() {
        let current_sha = hash_file(&target)?;
        let allowed_current = stored
            .observed_sha256
            .as_ref()
            .map(|observed| observed == &current_sha)
            .unwrap_or(false)
            || current_sha == stored.original_sha256;

        if !allowed_current {
            return Err(MutationError::TargetChanged);
        }
    }

    transition(
        database_path,
        stored.id,
        "rolling_back",
        "rollback_started",
        "Rollback started from the ready restore point.",
        None,
    )?;

    let result = (|| -> Result<(), MutationError> {
        if target.exists() {
            fs::remove_file(&target)?;
        }

        let parent = target
            .parent()
            .ok_or_else(|| MutationError::InvalidTarget("target has no parent".to_string()))?;
        fs::create_dir_all(parent)?;
        ensure_no_symlink_ancestors(&stored.mods_root, &stored.target_relative_path)?;

        let temporary = parent.join(format!(".smh-rollback-{}.tmp", stored.id));
        if temporary.exists() {
            fs::remove_file(&temporary)?;
        }
        fs::copy(&backup, &temporary)?;
        fs::rename(&temporary, &target)?;

        let restored_sha = hash_file(&target)?;
        if restored_sha != stored.original_sha256 {
            return Err(MutationError::IntegrityMismatch {
                expected: stored.original_sha256.clone(),
                observed: restored_sha,
            });
        }

        Ok(())
    })();

    if let Err(error) = result {
        mark_interrupted(database_path, stored.id, &error.to_string())?;
        return Err(error);
    }

    let mut connection = storage::open(database_path)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE update_transactions
         SET status = 'rolled_back',
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             error = NULL
         WHERE id = ?1",
        [stored.id],
    )?;
    transaction.execute(
        "UPDATE restore_points
         SET status = 'restored'
         WHERE id = ?1",
        [stored.restore_point_id],
    )?;
    transaction.execute(
        "INSERT INTO update_events (
            update_transaction_id, created_at, phase, detail
         ) VALUES (
            ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 'rollback_completed',
            'Previous artifact and target layout restored from the verified restore point.'
         )",
        [stored.id],
    )?;
    transaction.commit()?;

    load_transaction(database_path, stored.id)
}

pub(crate) fn recover_interrupted_transactions(
    database_path: &Path,
) -> Result<usize, MutationError> {
    let connection = storage::open(database_path)?;
    let count = connection.execute(
        "UPDATE update_transactions
         SET status = 'interrupted',
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             error = COALESCE(error, 'Application stopped before the update transaction completed.')
         WHERE status IN (
            'prepared',
            'downloading',
            'staged',
            'dependency_checked',
            'archiving',
            'archived',
            'installed',
            'validated',
            'rolling_back'
         )",
        [],
    )?;

    if count > 0 {
        connection.execute(
            "INSERT INTO update_events (
                update_transaction_id, created_at, phase, detail
             )
             SELECT
                id,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                'startup_recovery',
                'Incomplete update detected at startup. Restore point remains available for rollback.'
             FROM update_transactions
             WHERE status = 'interrupted'
               AND NOT EXISTS (
                   SELECT 1
                   FROM update_events e
                   WHERE e.update_transaction_id = update_transactions.id
                     AND e.phase = 'startup_recovery'
               )",
            [],
        )?;
    }

    Ok(count)
}

pub(crate) fn load_transaction(
    database_path: &Path,
    transaction_id: i64,
) -> Result<UpdateTransactionView, MutationError> {
    let connection = storage::open(database_path)?;
    let mut view = connection
        .query_row(
            "SELECT
                id,
                restore_point_id,
                status,
                target_relative_path,
                source_kind,
                source_url,
                current_release_id,
                replacement_release_id,
                expected_sha256,
                observed_sha256,
                created_at,
                updated_at,
                error
             FROM update_transactions
             WHERE id = ?1",
            [transaction_id],
            |row| {
                Ok(UpdateTransactionView {
                    transaction_id: row.get(0)?,
                    restore_point_id: row.get(1)?,
                    status: row.get(2)?,
                    target_relative_path: row.get(3)?,
                    source_kind: row.get(4)?,
                    source_url: row.get(5)?,
                    current_release_id: row.get(6)?,
                    replacement_release_id: row.get(7)?,
                    expected_sha256: row.get(8)?,
                    observed_sha256: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    error: row.get(12)?,
                    events: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or(MutationError::MissingTransaction(transaction_id))?;

    let mut statement = connection.prepare(
        "SELECT created_at, phase, detail
         FROM update_events
         WHERE update_transaction_id = ?1
         ORDER BY created_at, id",
    )?;
    view.events = statement
        .query_map([transaction_id], |row| {
            Ok(UpdateEventView {
                created_at: row.get(0)?,
                phase: row.get(1)?,
                detail: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(view)
}

fn prepare_transaction(
    database_path: &Path,
    app_data_dir: &Path,
    request: &ApplyUpdateRequest,
) -> Result<PreparedTransaction, MutationError> {
    if request.current_release_id == request.replacement_release_id {
        return Err(MutationError::InvalidTarget(
            "current and replacement release IDs are identical".to_string(),
        ));
    }

    let target_relative_path = validate_relative_target(&request.target_relative_path)?;
    let source = validate_source_url(&request.source_kind, &request.source_url)?;
    let sanitized_source = sanitized_source_url(&source);
    let expected_sha256 = normalize_expected_sha256(request.expected_sha256.as_deref())?;

    let mut connection = storage::open(database_path)?;
    let mods_root = connection
        .query_row(
            "SELECT mods_root FROM installations WHERE id = ?1",
            [request.installation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(PathBuf::from)
        .ok_or(MutationError::MissingInstallation(request.installation_id))?;

    let indexed = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM local_files
            WHERE installation_id = ?1
              AND relative_path = ?2
              AND enabled = 1
              AND file_kind IN ('package', 'ts4script')
        )",
        params![
            request.installation_id,
            target_relative_path.to_string_lossy().to_string()
        ],
        |row| row.get::<_, i64>(0),
    )?;
    if indexed == 0 {
        return Err(MutationError::TargetNotIndexed(
            target_relative_path.to_string_lossy().to_string(),
        ));
    }

    let target = checked_target_path(&mods_root, &target_relative_path)?;
    if !target.is_file() {
        return Err(MutationError::InvalidTarget(
            "target must be an existing regular file".to_string(),
        ));
    }
    if fs::symlink_metadata(&target)?.file_type().is_symlink() {
        return Err(MutationError::InvalidTarget(
            "target may not be a symbolic link".to_string(),
        ));
    }

    let original_sha256 = hash_file(&target)?;
    let operation_key = operation_key();
    let restore_root = app_data_dir.join("restore-points");
    let backup_root = restore_root.join(&operation_key);
    ensure_directory_within(&restore_root, &backup_root)?;
    let backup = checked_backup_path(&backup_root, &target_relative_path)?;
    let backup_parent = backup
        .parent()
        .ok_or_else(|| MutationError::InvalidTarget("backup has no parent".to_string()))?;
    fs::create_dir_all(backup_parent)?;
    fs::copy(&target, &backup)?;

    let backup_sha = hash_file(&backup)?;
    if backup_sha != original_sha256 {
        return Err(MutationError::IntegrityMismatch {
            expected: original_sha256,
            observed: backup_sha,
        });
    }

    let manifest = serde_json::json!({
        "version": 1,
        "targetRelativePath": target_relative_path.to_string_lossy(),
        "backupRelativePath": Path::new("tree").join(&target_relative_path).to_string_lossy(),
        "originalSha256": original_sha256.clone(),
        "originalExisted": true
    });

    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO restore_points (
            installation_id, created_at, reason, location, manifest_json, status
         ) VALUES (
            ?1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            'staged_mod_update',
            ?2,
            ?3,
            'ready'
         )",
        params![
            request.installation_id,
            backup_root.to_string_lossy().to_string(),
            manifest.to_string()
        ],
    )?;
    let restore_point_id = transaction.last_insert_rowid();

    transaction.execute(
        "INSERT INTO update_transactions (
            installation_id,
            restore_point_id,
            target_relative_path,
            source_kind,
            source_url,
            current_release_id,
            replacement_release_id,
            expected_sha256,
            original_sha256,
            status,
            created_at,
            updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'prepared',
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         )",
        params![
            request.installation_id,
            restore_point_id,
            target_relative_path.to_string_lossy().to_string(),
            request.source_kind,
            sanitized_source,
            request.current_release_id,
            request.replacement_release_id,
            expected_sha256,
            original_sha256
        ],
    )?;
    let transaction_id = transaction.last_insert_rowid();

    transaction.execute(
        "INSERT INTO update_events (
            update_transaction_id, created_at, phase, detail
         ) VALUES
            (
                ?1,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                'restore_point_ready',
                'Verified restore point created before any Mods-folder mutation.'
            ),
            (
                ?1,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                'source_validated',
                ?2
            )",
        params![
            transaction_id,
            format!(
                "Allowlisted source accepted: {} via {}.",
                sanitized_source, request.source_kind
            )
        ],
    )?;
    transaction.commit()?;

    let staging_parent = app_data_dir.join("update-staging");
    let staging_root = staging_parent.join(transaction_id.to_string());
    ensure_directory_within(&staging_parent, &staging_root)?;

    Ok(PreparedTransaction {
        id: transaction_id,
        restore_point_id,
        installation_id: request.installation_id,
        mods_root,
        backup_root,
        staging_root,
        target_relative_path,
        original_sha256,
        expected_sha256,
    })
}

async fn download_and_stage(
    database_path: &Path,
    prepared: &PreparedTransaction,
    request: &ApplyUpdateRequest,
) -> Result<(), MutationError> {
    transition(
        database_path,
        prepared.id,
        "downloading",
        "download_started",
        "Downloading replacement into the app-controlled staging directory.",
        None,
    )?;

    let source_url = validate_source_url(&request.source_kind, &request.source_url)?;
    let source_kind = request.source_kind.clone();
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .redirect(Policy::custom(move |attempt| {
            if source_url_allowed(&source_kind, attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("redirect target is not allowlisted for this source adapter")
            }
        }))
        .build()?;

    let mut response = client.get(source_url).send().await?.error_for_status()?;
    if response
        .content_length()
        .map(|size| size > MAX_DOWNLOAD_BYTES)
        .unwrap_or(false)
    {
        return Err(MutationError::DownloadTooLarge);
    }

    let download_path = prepared.staging_root.join("replacement.download");
    let mut file = File::create(&download_path)?;
    let mut written = 0_u64;

    while let Some(chunk) = response.chunk().await? {
        written = written.saturating_add(chunk.len() as u64);
        if written > MAX_DOWNLOAD_BYTES {
            drop(file);
            let _ = fs::remove_file(&download_path);
            return Err(MutationError::DownloadTooLarge);
        }
        file.write_all(&chunk)?;
    }
    file.flush()?;
    drop(file);

    let observed_sha256 = hash_file(&download_path)?;
    if let Some(expected) = prepared.expected_sha256.as_ref() {
        if expected != &observed_sha256 {
            let _ = fs::remove_file(&download_path);
            return Err(MutationError::IntegrityMismatch {
                expected: expected.clone(),
                observed: observed_sha256,
            });
        }
    }

    let staged_path = prepared.staging_root.join("replacement.staged");
    if staged_path.exists() {
        fs::remove_file(&staged_path)?;
    }
    fs::rename(&download_path, &staged_path)?;

    let detail = match prepared.expected_sha256.as_ref() {
        Some(expected) => format!(
            "SHA-256 verified against expected value {expected}; staged bytes remain outside the Mods folder."
        ),
        None => format!(
            "No expected SHA-256 was available from the source; observed SHA-256 is {observed_sha256} and is recorded for install/rollback validation."
        ),
    };

    let connection = storage::open(database_path)?;
    connection.execute(
        "UPDATE update_transactions
         SET observed_sha256 = ?2
         WHERE id = ?1",
        params![prepared.id, observed_sha256],
    )?;
    transition(
        database_path,
        prepared.id,
        "staged",
        "integrity_verified",
        &detail,
        None,
    )?;

    Ok(())
}

async fn verify_dependency_safety(
    database_path: &Path,
    prepared: &PreparedTransaction,
    request: &ApplyUpdateRequest,
) -> Result<(), MutationError> {
    let connection = storage::open(database_path)?;
    let registry_url = configured_registry_url(&connection);
    drop(connection);

    let client = RegistryClient::new(registry_url)?;
    let resolution =
        resolve_installed_releases(database_path, prepared.installation_id, &client).await?;

    if resolution.unresolved_files > 0 {
        return Err(MutationError::DependencyGraphIncomplete {
            unresolved_files: resolution.unresolved_files,
        });
    }

    if !resolution.release_ids.contains(&request.current_release_id) {
        return Err(MutationError::CurrentReleaseNotInstalled(
            request.current_release_id.clone(),
        ));
    }

    let current = resolution.release_ids.iter().cloned().collect::<Vec<_>>();
    let baseline = client.evaluate_relationships(&current).await?;

    let mut hypothetical = resolution.release_ids;
    hypothetical.remove(&request.current_release_id);
    hypothetical.insert(request.replacement_release_id.clone());
    let hypothetical = hypothetical.into_iter().collect::<Vec<_>>();
    let replacement = client.evaluate_relationships(&hypothetical).await?;

    let regression = relationship_regression(&baseline, &replacement);
    if let Some(detail) = regression {
        return Err(MutationError::DependencyRegression(detail));
    }

    let used_by = baseline
        .reverse_usage
        .iter()
        .find(|usage| usage.dependency_release_id == request.current_release_id)
        .map(|usage| usage.used_by_count)
        .unwrap_or(0);

    transition(
        database_path,
        prepared.id,
        "dependency_checked",
        "dependency_check_passed",
        &format!(
            "Dependency graph checked before archive/remove. Current release is used by {used_by} resolved installed releases; the hypothetical replacement introduces no new dependency finding or known incompatibility."
        ),
        None,
    )?;

    Ok(())
}

fn install_staged(
    database_path: &Path,
    prepared: &PreparedTransaction,
) -> Result<(), MutationError> {
    ensure_restore_point_ready(database_path, prepared.restore_point_id)?;

    let target = checked_target_path(&prepared.mods_root, &prepared.target_relative_path)?;
    if !target.exists() {
        return Err(MutationError::TargetChanged);
    }

    let current_sha = hash_file(&target)?;
    if current_sha != prepared.original_sha256 {
        return Err(MutationError::TargetChanged);
    }

    let staged = prepared.staging_root.join("replacement.staged");
    let staged_sha = hash_file(&staged)?;
    let expected_staged = observed_sha(database_path, prepared.id)?;
    if expected_staged.as_deref() != Some(staged_sha.as_str()) {
        return Err(MutationError::IntegrityMismatch {
            expected: expected_staged.unwrap_or_else(|| "recorded staged SHA-256".to_string()),
            observed: staged_sha,
        });
    }

    transition(
        database_path,
        prepared.id,
        "archiving",
        "archive_started",
        "Restore point is ready and dependency checks passed; archiving the previous artifact.",
        None,
    )?;

    let archive = prepared.staging_root.join("archive.previous");
    fs::copy(&target, &archive)?;
    if hash_file(&archive)? != prepared.original_sha256 {
        return Err(MutationError::IntegrityMismatch {
            expected: prepared.original_sha256.clone(),
            observed: hash_file(&archive)?,
        });
    }

    fs::remove_file(&target)?;
    transition(
        database_path,
        prepared.id,
        "archived",
        "previous_archived",
        "Previous artifact was removed from the active Mods layout only after a verified restore copy and staging archive existed.",
        None,
    )?;

    let parent = target
        .parent()
        .ok_or_else(|| MutationError::InvalidTarget("target has no parent".to_string()))?;
    let temporary = parent.join(format!(".smh-install-{}.tmp", prepared.id));
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    fs::copy(&staged, &temporary)?;
    fs::rename(&temporary, &target)?;

    transition(
        database_path,
        prepared.id,
        "installed",
        "replacement_installed",
        "Replacement moved into the original target path using a same-directory temporary file.",
        None,
    )?;

    validate_installed_target(
        &prepared.mods_root,
        &prepared.target_relative_path,
        &staged_sha,
    )?;
    transition(
        database_path,
        prepared.id,
        "validated",
        "layout_validated",
        "Installed target is a regular supported mod file inside the Mods tree and its SHA-256 matches the staged artifact.",
        None,
    )?;

    transition(
        database_path,
        prepared.id,
        "completed",
        "update_completed",
        "Single-artifact staged update completed. Restore point remains available for rollback.",
        None,
    )?;

    Ok(())
}

fn validate_installed_target(
    mods_root: &Path,
    relative: &Path,
    expected_sha256: &str,
) -> Result<(), MutationError> {
    let target = checked_target_path(mods_root, relative)?;
    let metadata = fs::symlink_metadata(&target)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(MutationError::InvalidTarget(
            "installed target is not a regular non-symlink file".to_string(),
        ));
    }

    let observed = hash_file(&target)?;
    if observed != expected_sha256 {
        return Err(MutationError::IntegrityMismatch {
            expected: expected_sha256.to_string(),
            observed,
        });
    }

    Ok(())
}

fn ensure_restore_point_ready(
    database_path: &Path,
    restore_point_id: i64,
) -> Result<(), MutationError> {
    let connection = storage::open(database_path)?;
    let status = connection
        .query_row(
            "SELECT status FROM restore_points WHERE id = ?1",
            [restore_point_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    if status.as_deref() != Some("ready") {
        return Err(MutationError::RestorePointRequired);
    }

    Ok(())
}

struct InstalledResolution {
    release_ids: BTreeSet<String>,
    unresolved_files: usize,
}

fn load_installed_resolution_input(
    database_path: &Path,
    installation_id: i64,
) -> Result<(usize, Vec<RegistryArtifactProbe>), MutationError> {
    let connection = storage::open(database_path)?;
    let eligible_count = connection.query_row(
        "SELECT COUNT(*)
         FROM local_files
         WHERE installation_id = ?1
           AND enabled = 1
           AND file_kind IN ('package', 'ts4script')",
        [installation_id],
        |row| row.get::<_, i64>(0),
    )? as usize;

    let mut statement = connection.prepare(
        "SELECT
            lf.id,
            lf.relative_path,
            lf.file_kind,
            lf.size_bytes,
            f.kind,
            f.value,
            f.algorithm_version
         FROM local_files lf
         LEFT JOIN fingerprints f
           ON f.local_file_id = lf.id
          AND f.kind IN ('sha256', 'curseforge', 'resource_signature', 'script_signature')
         WHERE lf.installation_id = ?1
           AND lf.enabled = 1
           AND lf.file_kind IN ('package', 'ts4script')
         ORDER BY lf.id, f.kind",
    )?;

    let rows = statement.query_map([installation_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;

    let mut probes = Vec::<RegistryArtifactProbe>::new();
    let mut index_by_file = HashMap::<i64, usize>::new();

    for row in rows {
        let (
            local_file_id,
            relative_path,
            file_kind,
            size_bytes,
            fingerprint_kind,
            fingerprint_value,
            algorithm_version,
        ) = row?;

        let index = *index_by_file.entry(local_file_id).or_insert_with(|| {
            let filename = Path::new(&relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string);
            probes.push(RegistryArtifactProbe {
                client_ref: local_file_id.to_string(),
                artifact_kind: Some(file_kind),
                filename,
                size_bytes,
                identity_hints: None,
                fingerprints: Vec::new(),
            });
            probes.len() - 1
        });

        if let (Some(kind), Some(value), Some(algorithm_version)) =
            (fingerprint_kind, fingerprint_value, algorithm_version)
        {
            probes[index].fingerprints.push(RegistryFingerprintProbe {
                kind,
                value,
                algorithm_version,
            });
        }
    }

    probes.retain(|probe| !probe.fingerprints.is_empty());
    Ok((eligible_count, probes))
}

async fn resolve_installed_releases(
    database_path: &Path,
    installation_id: i64,
    client: &RegistryClient,
) -> Result<InstalledResolution, MutationError> {
    let (eligible_count, probes) = load_installed_resolution_input(database_path, installation_id)?;
    let resolutions = client.resolve_artifacts(&probes).await?;
    Ok(collect_deterministic_releases(eligible_count, &resolutions))
}

fn collect_deterministic_releases(
    eligible_count: usize,
    resolutions: &[ArtifactResolution],
) -> InstalledResolution {
    let mut resolved_files = HashSet::new();
    let mut release_ids = BTreeSet::new();

    for resolution in resolutions {
        if resolution.status != "resolved" {
            continue;
        }

        let selected = resolution.selected_artifact_id.as_deref();
        let matched = resolution.matches.iter().find(|candidate| {
            candidate.deterministic && Some(candidate.artifact_id.as_str()) == selected
        });

        if let Some(matched) = matched {
            resolved_files.insert(resolution.client_ref.clone());
            release_ids.insert(matched.release_id.clone());
        }
    }

    InstalledResolution {
        release_ids,
        unresolved_files: eligible_count.saturating_sub(resolved_files.len()),
    }
}

fn relationship_regression(
    baseline: &RelationshipResponse,
    replacement: &RelationshipResponse,
) -> Option<String> {
    let baseline_dependencies = baseline
        .dependency_findings
        .iter()
        .map(|finding| {
            (
                finding.rule_id.as_str(),
                finding.required_by_release_id.as_str(),
                finding.status.as_str(),
                finding.action.as_str(),
            )
        })
        .collect::<HashSet<_>>();

    if let Some(finding) = replacement.dependency_findings.iter().find(|finding| {
        !baseline_dependencies.contains(&(
            finding.rule_id.as_str(),
            finding.required_by_release_id.as_str(),
            finding.status.as_str(),
            finding.action.as_str(),
        ))
    }) {
        return Some(format!(
            "new {} dependency finding for release {}",
            finding.status, finding.required_by_release_id
        ));
    }

    let baseline_conflicts = baseline
        .known_incompatibilities
        .iter()
        .map(|finding| {
            (
                finding.rule_id.as_str(),
                finding.left_release_id.as_str(),
                finding.right_release_id.as_str(),
            )
        })
        .collect::<HashSet<_>>();

    replacement
        .known_incompatibilities
        .iter()
        .find(|finding| {
            !baseline_conflicts.contains(&(
                finding.rule_id.as_str(),
                finding.left_release_id.as_str(),
                finding.right_release_id.as_str(),
            ))
        })
        .map(|finding| {
            format!(
                "new known incompatibility between releases {} and {}",
                finding.left_release_id, finding.right_release_id
            )
        })
}

fn validate_relative_target(value: &str) -> Result<PathBuf, MutationError> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        return Err(MutationError::InvalidTarget(
            "path must be non-empty and relative to Mods".to_string(),
        ));
    }

    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(MutationError::InvalidTarget(
            "parent traversal, prefixes and dot components are not allowed".to_string(),
        ));
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    if !matches!(extension.as_deref(), Some("package" | "ts4script")) {
        return Err(MutationError::InvalidTarget(
            "automatic mutation supports only .package and .ts4script targets".to_string(),
        ));
    }

    Ok(path.to_path_buf())
}

fn checked_target_path(mods_root: &Path, relative: &Path) -> Result<PathBuf, MutationError> {
    let relative = validate_relative_target(&relative.to_string_lossy())?;
    let canonical_root = fs::canonicalize(mods_root)?;
    ensure_no_symlink_ancestors(&canonical_root, &relative)?;
    Ok(canonical_root.join(relative))
}

fn checked_backup_path(backup_root: &Path, relative: &Path) -> Result<PathBuf, MutationError> {
    let relative = validate_relative_target(&relative.to_string_lossy())?;
    Ok(backup_root.join("tree").join(relative))
}

fn ensure_no_symlink_ancestors(root: &Path, relative: &Path) -> Result<(), MutationError> {
    let mut current = root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();

    for component in components.iter().take(components.len().saturating_sub(1)) {
        let Component::Normal(part) = component else {
            return Err(MutationError::InvalidTarget(
                "unsafe path component".to_string(),
            ));
        };
        current.push(part);
        if current.exists() {
            let metadata = fs::symlink_metadata(&current)?;
            if metadata.file_type().is_symlink() {
                return Err(MutationError::InvalidTarget(
                    "target parent may not traverse a symbolic link".to_string(),
                ));
            }
            if !metadata.is_dir() {
                return Err(MutationError::InvalidTarget(
                    "target parent component is not a directory".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn ensure_directory_within(root: &Path, child: &Path) -> Result<(), MutationError> {
    fs::create_dir_all(root)?;
    fs::create_dir_all(child)?;
    ensure_existing_directory_within(root, child)
}

fn ensure_existing_directory_within(root: &Path, child: &Path) -> Result<(), MutationError> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_child = fs::canonicalize(child)?;
    if !canonical_child.starts_with(&canonical_root) {
        return Err(MutationError::InvalidTarget(
            "mutation workspace escaped the application data root".to_string(),
        ));
    }
    Ok(())
}

fn validate_source_url(source_kind: &str, value: &str) -> Result<Url, MutationError> {
    let url = Url::parse(value).map_err(|error| MutationError::InvalidSource(error.to_string()))?;

    if !source_url_allowed(source_kind, &url) {
        return Err(MutationError::InvalidSource(format!(
            "{} is not an allowlisted HTTPS download for source kind {source_kind}",
            sanitized_source_url(&url)
        )));
    }

    Ok(url)
}

fn source_url_allowed(source_kind: &str, url: &Url) -> bool {
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.port(), None | Some(443))
    {
        return false;
    }

    let Some(host) = url.host_str().map(|value| value.to_ascii_lowercase()) else {
        return false;
    };

    match source_kind {
        "github_releases" => matches!(
            host.as_str(),
            "github.com" | "objects.githubusercontent.com" | "release-assets.githubusercontent.com"
        ),
        "curseforge" => host == "forgecdn.net" || host.ends_with(".forgecdn.net"),
        _ => false,
    }
}

fn sanitized_source_url(url: &Url) -> String {
    let mut sanitized = url.clone();
    sanitized.set_query(None);
    sanitized.set_fragment(None);
    sanitized.to_string()
}

fn normalize_expected_sha256(value: Option<&str>) -> Result<Option<String>, MutationError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(MutationError::InvalidHash(
            "expected value must be exactly 64 hexadecimal characters".to_string(),
        ));
    }
    Ok(Some(normalized))
}

fn hash_file(path: &Path) -> Result<String, MutationError> {
    fingerprint::sha256_file(path, || false)?.ok_or_else(|| {
        MutationError::InvalidTarget("hashing was unexpectedly cancelled".to_string())
    })
}

fn operation_key() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{nanos}", process::id())
}

fn configured_registry_url(connection: &Connection) -> String {
    if let Ok(value) = std::env::var("SIMS_MOD_HEALTH_REGISTRY_URL") {
        if !value.trim().is_empty() {
            return value;
        }
    }

    let preference = connection
        .query_row(
            "SELECT value_json FROM preferences WHERE key = 'registry.base_url'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten();

    preference
        .and_then(|value| serde_json::from_str::<String>(&value).ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_string())
}

fn observed_sha(
    database_path: &Path,
    transaction_id: i64,
) -> Result<Option<String>, MutationError> {
    let connection = storage::open(database_path)?;
    Ok(connection
        .query_row(
            "SELECT observed_sha256 FROM update_transactions WHERE id = ?1",
            [transaction_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

fn load_stored_transaction(
    database_path: &Path,
    transaction_id: i64,
) -> Result<StoredTransaction, MutationError> {
    let connection = storage::open(database_path)?;
    connection
        .query_row(
            "SELECT
                ut.id,
                ut.restore_point_id,
                ut.installation_id,
                i.mods_root,
                rp.location,
                ut.target_relative_path,
                ut.original_sha256,
                ut.observed_sha256,
                ut.status
             FROM update_transactions ut
             JOIN restore_points rp ON rp.id = ut.restore_point_id
             JOIN installations i ON i.id = ut.installation_id
             WHERE ut.id = ?1",
            [transaction_id],
            |row| {
                Ok(StoredTransaction {
                    id: row.get(0)?,
                    restore_point_id: row.get(1)?,
                    installation_id: row.get(2)?,
                    mods_root: PathBuf::from(row.get::<_, String>(3)?),
                    backup_root: PathBuf::from(row.get::<_, String>(4)?),
                    target_relative_path: PathBuf::from(row.get::<_, String>(5)?),
                    original_sha256: row.get(6)?,
                    observed_sha256: row.get(7)?,
                    status: row.get(8)?,
                })
            },
        )
        .optional()?
        .ok_or(MutationError::MissingTransaction(transaction_id))
}

fn transition(
    database_path: &Path,
    transaction_id: i64,
    status: &str,
    phase: &str,
    detail: &str,
    error: Option<&str>,
) -> Result<(), MutationError> {
    let mut connection = storage::open(database_path)?;
    let transaction = connection.transaction()?;
    transaction.execute(
        "UPDATE update_transactions
         SET status = ?2,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             error = ?3
         WHERE id = ?1",
        params![transaction_id, status, error],
    )?;
    transaction.execute(
        "INSERT INTO update_events (
            update_transaction_id, created_at, phase, detail
         ) VALUES (
            ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?2, ?3
         )",
        params![transaction_id, phase, detail],
    )?;
    transaction.commit()?;
    Ok(())
}

fn error_detail_for_journal(error: &MutationError) -> String {
    match error {
        MutationError::Http(_) => {
            "Download failed after source validation; remote URL details were not persisted."
                .to_string()
        }
        MutationError::Registry(_) => {
            "Registry dependency safety check failed before Mods-folder mutation.".to_string()
        }
        _ => error.to_string(),
    }
}

fn mark_failed(
    database_path: &Path,
    transaction_id: i64,
    detail: &str,
) -> Result<(), MutationError> {
    transition(
        database_path,
        transaction_id,
        "failed",
        "update_failed_before_mutation",
        detail,
        Some(detail),
    )
}

fn mark_interrupted(
    database_path: &Path,
    transaction_id: i64,
    detail: &str,
) -> Result<(), MutationError> {
    transition(
        database_path,
        transaction_id,
        "interrupted",
        "update_interrupted",
        detail,
        Some(detail),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{DependencyFinding, KnownIncompatibilityFinding, ReverseDependencyUsage};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf, PathBuf, PathBuf) {
        let temp = TempDir::new().expect("mutation temp directory");
        let app_data = temp.path().join("app-data");
        let mods_root = temp.path().join("Mods");
        let target = mods_root.join("Gameplay/Example.package");
        fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
        fs::write(&target, b"old release").expect("write old release");

        let database_path = app_data.join("sims-mod-health.sqlite3");
        storage::initialize(&database_path).expect("initialize local database");
        let connection = storage::open(&database_path).expect("open local database");
        connection
            .execute(
                "INSERT INTO installations (
                    game_root, mods_root, platform, discovered_at, last_seen_at
                 ) VALUES ('game', ?1, 'windows', 'now', 'now')",
                [mods_root.to_string_lossy().to_string()],
            )
            .expect("insert installation");
        connection
            .execute(
                "INSERT INTO local_files (
                    installation_id, relative_path, file_kind, enabled
                 ) VALUES (1, 'Gameplay/Example.package', 'package', 1)",
                [],
            )
            .expect("insert local file");

        (temp, app_data, database_path, target)
    }

    fn request() -> ApplyUpdateRequest {
        ApplyUpdateRequest {
            installation_id: 1,
            target_relative_path: "Gameplay/Example.package".to_string(),
            source_kind: "github_releases".to_string(),
            source_url:
                "https://github.com/example/mod/releases/download/v2/mod.package?token=secret"
                    .to_string(),
            current_release_id: "00000000-0000-0000-0000-000000000001".to_string(),
            replacement_release_id: "00000000-0000-0000-0000-000000000002".to_string(),
            expected_sha256: None,
        }
    }

    fn stage_for_test(database_path: &Path, prepared: &PreparedTransaction, bytes: &[u8]) {
        let staged = prepared.staging_root.join("replacement.staged");
        fs::write(&staged, bytes).expect("write staged fixture");
        let sha = hash_file(&staged).expect("hash staged fixture");
        let connection = storage::open(database_path).expect("open database");
        connection
            .execute(
                "UPDATE update_transactions
                 SET observed_sha256 = ?2,
                     status = 'dependency_checked'
                 WHERE id = ?1",
                params![prepared.id, sha],
            )
            .expect("mark staged fixture dependency checked");
    }

    #[test]
    fn path_traversal_is_rejected_before_restore_point_creation() {
        assert!(validate_relative_target("../evil.package").is_err());
        assert!(validate_relative_target("C:\\evil.package").is_err());
        assert!(validate_relative_target("./evil.package").is_err());
        assert!(validate_relative_target("safe/mod.exe").is_err());
    }

    #[test]
    fn source_allowlist_requires_https_and_adapter_specific_hosts() {
        assert!(validate_source_url(
            "github_releases",
            "https://release-assets.githubusercontent.com/asset.package"
        )
        .is_ok());
        assert!(validate_source_url(
            "curseforge",
            "https://mediafilez.forgecdn.net/files/1/mod.package"
        )
        .is_ok());
        assert!(
            validate_source_url("github_releases", "http://github.com/example/mod.package")
                .is_err()
        );
        assert!(
            validate_source_url("github_releases", "https://evil.example/mod.package").is_err()
        );
        assert!(validate_source_url(
            "curseforge",
            "https://forgecdn.net.evil.example/mod.package"
        )
        .is_err());
    }

    #[test]
    fn restore_point_is_ready_before_any_target_mutation() {
        let (_temp, app_data, database_path, target) = fixture();
        let original = fs::read(&target).expect("read original");

        let prepared =
            prepare_transaction(&database_path, &app_data, &request()).expect("prepare update");
        let view = load_transaction(&database_path, prepared.id).expect("load transaction");

        assert_eq!(view.status, "prepared");
        assert_eq!(fs::read(&target).expect("target unchanged"), original);
        assert!(prepared
            .backup_root
            .join("tree/Gameplay/Example.package")
            .is_file());
        assert!(view
            .events
            .iter()
            .any(|event| event.phase == "restore_point_ready"));
        assert!(!view.source_url.contains("token=secret"));
    }

    #[test]
    fn install_refuses_to_mutate_without_ready_restore_point() {
        let (_temp, app_data, database_path, target) = fixture();
        let prepared =
            prepare_transaction(&database_path, &app_data, &request()).expect("prepare update");
        stage_for_test(&database_path, &prepared, b"new release");

        let connection = storage::open(&database_path).expect("open database");
        connection
            .execute(
                "UPDATE restore_points SET status = 'failed' WHERE id = ?1",
                [prepared.restore_point_id],
            )
            .expect("invalidate restore point");

        let error = install_staged(&database_path, &prepared)
            .expect_err("mutation must require a ready restore point");

        assert!(matches!(error, MutationError::RestorePointRequired));
        assert_eq!(fs::read(&target).expect("target unchanged"), b"old release");
    }

    #[test]
    fn rollback_restores_previous_artifact_and_layout() {
        let (_temp, app_data, database_path, target) = fixture();
        let prepared =
            prepare_transaction(&database_path, &app_data, &request()).expect("prepare update");
        stage_for_test(&database_path, &prepared, b"new release");

        install_staged(&database_path, &prepared).expect("install staged release");
        assert_eq!(fs::read(&target).expect("read new target"), b"new release");

        let rolled_back =
            rollback_update(&database_path, &app_data, prepared.id).expect("rollback update");

        assert_eq!(rolled_back.status, "rolled_back");
        assert_eq!(
            fs::read(&target).expect("read restored target"),
            b"old release"
        );
        assert!(rolled_back
            .events
            .iter()
            .any(|event| event.phase == "rollback_completed"));
    }

    #[test]
    fn interrupted_transaction_remains_recoverable_after_restart() {
        let (_temp, app_data, database_path, target) = fixture();
        let prepared =
            prepare_transaction(&database_path, &app_data, &request()).expect("prepare update");
        stage_for_test(&database_path, &prepared, b"new release");
        install_staged(&database_path, &prepared).expect("install staged release");

        let connection = storage::open(&database_path).expect("open database");
        connection
            .execute(
                "UPDATE update_transactions SET status = 'installed' WHERE id = ?1",
                [prepared.id],
            )
            .expect("simulate interrupted install");

        assert_eq!(
            recover_interrupted_transactions(&database_path).expect("recover interrupted"),
            1
        );
        let interrupted =
            load_transaction(&database_path, prepared.id).expect("load interrupted transaction");
        assert_eq!(interrupted.status, "interrupted");

        rollback_update(&database_path, &app_data, prepared.id)
            .expect("rollback interrupted update");
        assert_eq!(
            fs::read(&target).expect("read restored target"),
            b"old release"
        );
    }

    #[test]
    fn rollback_refuses_to_overwrite_an_independent_target_change() {
        let (_temp, app_data, database_path, target) = fixture();
        let prepared =
            prepare_transaction(&database_path, &app_data, &request()).expect("prepare update");
        stage_for_test(&database_path, &prepared, b"new release");
        install_staged(&database_path, &prepared).expect("install staged release");

        fs::write(&target, b"user changed this after update").expect("write independent change");

        let error = rollback_update(&database_path, &app_data, prepared.id)
            .expect_err("rollback must not overwrite an independent target change");

        assert!(matches!(error, MutationError::TargetChanged));
        assert_eq!(
            fs::read(&target).expect("read independent target"),
            b"user changed this after update"
        );
    }

    #[test]
    fn dependency_regression_blocks_new_findings() {
        let baseline = RelationshipResponse {
            dependency_findings: Vec::new(),
            known_incompatibilities: Vec::new(),
            reverse_usage: vec![ReverseDependencyUsage {
                dependency_release_id: "old".to_string(),
                used_by_count: 2,
                used_by_release_ids: vec!["a".to_string(), "b".to_string()],
            }],
        };
        let replacement = RelationshipResponse {
            dependency_findings: vec![DependencyFinding {
                rule_id: "rule".to_string(),
                required_by_release_id: "dependent".to_string(),
                status: "missing".to_string(),
                action: "install_dependency".to_string(),
            }],
            known_incompatibilities: Vec::new(),
            reverse_usage: Vec::new(),
        };

        let regression = relationship_regression(&baseline, &replacement)
            .expect("new dependency finding must block");

        assert!(regression.contains("missing dependency"));
    }

    #[test]
    fn existing_relationship_findings_do_not_block_an_unrelated_replacement() {
        let finding = DependencyFinding {
            rule_id: "existing".to_string(),
            required_by_release_id: "other".to_string(),
            status: "missing".to_string(),
            action: "install_dependency".to_string(),
        };
        let incompatibility = KnownIncompatibilityFinding {
            rule_id: "existing-conflict".to_string(),
            left_release_id: "a".to_string(),
            right_release_id: "b".to_string(),
        };
        let baseline = RelationshipResponse {
            dependency_findings: vec![finding.clone()],
            known_incompatibilities: vec![incompatibility.clone()],
            reverse_usage: Vec::new(),
        };
        let replacement = RelationshipResponse {
            dependency_findings: vec![finding],
            known_incompatibilities: vec![incompatibility],
            reverse_usage: Vec::new(),
        };

        assert!(relationship_regression(&baseline, &replacement).is_none());
    }

    #[test]
    fn bulk_update_capability_remains_disabled() {
        assert!(!BULK_UPDATE_ENABLED);
    }
}
