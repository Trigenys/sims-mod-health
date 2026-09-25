use std::{
    fmt::{Display, Formatter},
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    game::{self, ManualInspection, VersionState},
    storage::{self, StorageError},
};

const HASH_BUFFER_SIZE: usize = 64 * 1024;
const HASH_ALGORITHM_VERSION: &str = "sha256-v1";
const PROGRESS_INTERVAL: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ScanMode {
    Incremental,
    Full,
}

impl ScanMode {
    fn database_value(self) -> &'static str {
        match self {
            Self::Incremental => "incremental",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgress {
    pub(crate) files_seen: u64,
    pub(crate) files_hashed: u64,
    pub(crate) files_skipped: u64,
    pub(crate) observations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanSummary {
    pub(crate) scan_session_id: i64,
    pub(crate) status: ScanStatus,
    pub(crate) files_seen: u64,
    pub(crate) files_hashed: u64,
    pub(crate) files_skipped: u64,
    pub(crate) observations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ScanStatus {
    Completed,
    Cancelled,
}

#[derive(Debug)]
pub(crate) enum ScanError {
    AlreadyRunning,
    InvalidInstallation(String),
    ModsUnavailable,
    Io(io::Error),
    Sqlite(rusqlite::Error),
    Storage(StorageError),
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(formatter, "a Mods scan is already running"),
            Self::InvalidInstallation(reason) => {
                write!(formatter, "invalid Sims installation: {reason}")
            }
            Self::ModsUnavailable => write!(
                formatter,
                "the selected Sims installation has no Mods directory"
            ),
            Self::Io(error) => write!(formatter, "scanner I/O error: {error}"),
            Self::Sqlite(error) => write!(formatter, "scanner SQLite error: {error}"),
            Self::Storage(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ScanError {}

impl From<io::Error> for ScanError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for ScanError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<StorageError> for ScanError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

#[derive(Default)]
pub(crate) struct ScannerControl {
    running: AtomicBool,
    cancelled: AtomicBool,
}

impl ScannerControl {
    pub(crate) fn cancel(&self) -> bool {
        if !self.running.load(Ordering::SeqCst) {
            return false;
        }

        self.cancelled.store(true, Ordering::SeqCst);
        true
    }

    fn begin(&self) -> Result<RunningGuard<'_>, ScanError> {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| ScanError::AlreadyRunning)?;

        self.cancelled.store(false, Ordering::SeqCst);
        Ok(RunningGuard { control: self })
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

struct RunningGuard<'a> {
    control: &'a ScannerControl,
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.control.running.store(false, Ordering::SeqCst);
        self.control.cancelled.store(false, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct PendingObservation {
    relative_path: Option<String>,
    kind: &'static str,
    detail: String,
}

#[derive(Debug)]
struct CollectedFiles {
    files: Vec<(String, PathBuf, FileKind)>,
    observations: Vec<PendingObservation>,
    preserve_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Package,
    Ts4Script,
}

impl FileKind {
    fn database_value(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Ts4Script => "ts4script",
        }
    }
}

#[derive(Debug)]
struct LocalCache {
    size_bytes: Option<i64>,
    modified_ns: Option<i64>,
    has_sha256: bool,
}

pub(crate) fn scan_path<F>(
    database_path: &Path,
    selected_path: &Path,
    mode: ScanMode,
    control: &ScannerControl,
    mut on_progress: F,
) -> Result<ScanSummary, ScanError>
where
    F: FnMut(&ScanProgress),
{
    let _running = control.begin()?;

    let installation = match game::inspect_manual_path(selected_path) {
        ManualInspection::Available { installation } => installation,
        ManualInspection::Unavailable { reason } => {
            return Err(ScanError::InvalidInstallation(reason));
        }
    };

    if !installation.mods_available {
        return Err(ScanError::ModsUnavailable);
    }

    let mut connection = storage::open(database_path)?;
    let game_version = match &installation.version {
        VersionState::Available { version } => Some(version.normalized.as_str()),
        VersionState::Missing | VersionState::Invalid { .. } => None,
    };

    let installation_id = upsert_installation(
        &connection,
        &installation.root,
        &installation.mods_root,
        game_version,
    )?;
    let scan_session_id = create_scan_session(&connection, installation_id, mode)?;

    let result = scan_transaction(
        &mut connection,
        scan_session_id,
        installation_id,
        &installation.mods_root,
        mode,
        control,
        &mut on_progress,
    );

    match result {
        Ok(summary) => {
            finish_scan_session(&connection, &summary)?;
            Ok(summary)
        }
        Err(error) => {
            let _ = mark_scan_failed(&connection, scan_session_id);
            Err(error)
        }
    }
}

fn scan_transaction<F>(
    connection: &mut Connection,
    scan_session_id: i64,
    installation_id: i64,
    mods_root: &Path,
    mode: ScanMode,
    control: &ScannerControl,
    on_progress: &mut F,
) -> Result<ScanSummary, ScanError>
where
    F: FnMut(&ScanProgress),
{
    let transaction = connection.transaction()?;
    let CollectedFiles {
        mut files,
        observations,
        preserve_prefixes,
    } = collect_supported_files(mods_root, control);

    let mut progress = ScanProgress {
        files_seen: 0,
        files_hashed: 0,
        files_skipped: 0,
        observations: 0,
    };

    for observation in observations {
        insert_observation(&transaction, scan_session_id, &observation)?;
        progress.observations += 1;
    }

    files.sort_by(|left, right| left.0.cmp(&right.0));

    for (relative_path, absolute_path, file_kind) in files {
        if control.is_cancelled() {
            drop(transaction);
            let summary = ScanSummary {
                scan_session_id,
                status: ScanStatus::Cancelled,
                files_seen: progress.files_seen,
                files_hashed: progress.files_hashed,
                files_skipped: progress.files_skipped,
                observations: progress.observations,
            };
            on_progress(&progress);
            return Ok(summary);
        }

        progress.files_seen += 1;

        let metadata = match fs::metadata(&absolute_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                let observation_kind = observation_kind_for_io(&error);
                insert_observation(
                    &transaction,
                    scan_session_id,
                    &PendingObservation {
                        relative_path: Some(relative_path.clone()),
                        kind: observation_kind,
                        detail: error.to_string(),
                    },
                )?;

                if observation_kind != "disappeared" {
                    mark_existing_file_seen(
                        &transaction,
                        installation_id,
                        scan_session_id,
                        &relative_path,
                    )?;
                }

                progress.observations += 1;
                emit_progress_if_needed(&progress, on_progress);
                continue;
            }
        };

        let size_bytes = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
        let modified_ns = modified_ns(&metadata);
        let cache = load_cache(&transaction, installation_id, &relative_path)?;
        let cache_hit = mode == ScanMode::Incremental
            && cache.as_ref().is_some_and(|cached| {
                cached.size_bytes == Some(size_bytes)
                    && cached.modified_ns == modified_ns
                    && cached.has_sha256
            });

        let local_file_id = upsert_local_file(
            &transaction,
            installation_id,
            scan_session_id,
            &relative_path,
            file_kind,
            size_bytes,
            modified_ns,
        )?;

        if cache_hit {
            progress.files_skipped += 1;
        } else {
            delete_sha256(&transaction, local_file_id)?;

            match sha256_file(&absolute_path, control) {
                Ok(Some(hash)) => {
                    store_sha256(&transaction, local_file_id, &hash)?;
                    transaction.execute(
                        "UPDATE local_files
                         SET hashed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                             parse_status = 'pending'
                         WHERE id = ?1",
                        [local_file_id],
                    )?;
                    progress.files_hashed += 1;
                }
                Ok(None) => {
                    drop(transaction);
                    let summary = ScanSummary {
                        scan_session_id,
                        status: ScanStatus::Cancelled,
                        files_seen: progress.files_seen,
                        files_hashed: progress.files_hashed,
                        files_skipped: progress.files_skipped,
                        observations: progress.observations,
                    };
                    on_progress(&progress);
                    return Ok(summary);
                }
                Err(error) => {
                    if error.kind() == io::ErrorKind::NotFound {
                        transaction
                            .execute("DELETE FROM local_files WHERE id = ?1", [local_file_id])?;
                    }

                    insert_observation(
                        &transaction,
                        scan_session_id,
                        &PendingObservation {
                            relative_path: Some(relative_path.clone()),
                            kind: observation_kind_for_io(&error),
                            detail: error.to_string(),
                        },
                    )?;
                    progress.observations += 1;
                }
            }
        }

        record_depth_observation(
            &transaction,
            scan_session_id,
            &relative_path,
            file_kind,
            &mut progress,
        )?;
        emit_progress_if_needed(&progress, on_progress);
    }

    if control.is_cancelled() {
        drop(transaction);
        let summary = ScanSummary {
            scan_session_id,
            status: ScanStatus::Cancelled,
            files_seen: progress.files_seen,
            files_hashed: progress.files_hashed,
            files_skipped: progress.files_skipped,
            observations: progress.observations,
        };
        on_progress(&progress);
        return Ok(summary);
    }

    finalize_inventory(
        &transaction,
        installation_id,
        scan_session_id,
        &preserve_prefixes,
    )?;

    transaction.commit()?;
    on_progress(&progress);

    Ok(ScanSummary {
        scan_session_id,
        status: ScanStatus::Completed,
        files_seen: progress.files_seen,
        files_hashed: progress.files_hashed,
        files_skipped: progress.files_skipped,
        observations: progress.observations,
    })
}

fn collect_supported_files(mods_root: &Path, control: &ScannerControl) -> CollectedFiles {
    let mut pending_directories = vec![mods_root.to_path_buf()];
    let mut files = Vec::new();
    let mut observations = Vec::new();
    let mut preserve_prefixes = Vec::new();

    while let Some(directory) = pending_directories.pop() {
        if control.is_cancelled() {
            break;
        }

        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                let relative_path = relative_string(mods_root, &directory);
                if observation_kind_for_io(&error) != "disappeared" {
                    if let Some(prefix) = relative_path.clone() {
                        preserve_prefixes.push(prefix);
                    }
                }
                observations.push(PendingObservation {
                    relative_path,
                    kind: observation_kind_for_io(&error),
                    detail: error.to_string(),
                });
                continue;
            }
        };

        let mut collected_entries = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => collected_entries.push(entry),
                Err(error) => {
                    let relative_path = relative_string(mods_root, &directory);
                    if observation_kind_for_io(&error) != "disappeared" {
                        if let Some(prefix) = relative_path.clone() {
                            preserve_prefixes.push(prefix);
                        }
                    }
                    observations.push(PendingObservation {
                        relative_path,
                        kind: observation_kind_for_io(&error),
                        detail: error.to_string(),
                    });
                }
            }
        }

        collected_entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

        for entry in collected_entries.into_iter().rev() {
            if control.is_cancelled() {
                break;
            }

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    let relative_path = relative_string(mods_root, &path);
                    if observation_kind_for_io(&error) != "disappeared" {
                        if let Some(prefix) = relative_path.clone() {
                            preserve_prefixes.push(prefix);
                        }
                    }
                    observations.push(PendingObservation {
                        relative_path,
                        kind: observation_kind_for_io(&error),
                        detail: error.to_string(),
                    });
                    continue;
                }
            };

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("Mods"))
                    && path != mods_root
                {
                    observations.push(PendingObservation {
                        relative_path: relative_string(mods_root, &path),
                        kind: "nestedModsDirectory",
                        detail: "A Mods directory is nested inside the active Mods directory."
                            .to_string(),
                    });
                }
                pending_directories.push(path);
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let Some(kind) = file_kind(&path) else {
                continue;
            };
            let Some(relative_path) = relative_string(mods_root, &path) else {
                continue;
            };

            files.push((relative_path, path, kind));
        }
    }

    preserve_prefixes.sort();
    preserve_prefixes.dedup();

    CollectedFiles {
        files,
        observations,
        preserve_prefixes,
    }
}

fn mark_existing_file_seen(
    connection: &Connection,
    installation_id: i64,
    scan_session_id: i64,
    relative_path: &str,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "UPDATE local_files
         SET last_seen_scan_id = ?2
         WHERE installation_id = ?1
           AND relative_path = ?3",
        params![installation_id, scan_session_id, relative_path],
    )?;
    Ok(())
}

fn finalize_inventory(
    connection: &Connection,
    installation_id: i64,
    scan_session_id: i64,
    preserve_prefixes: &[String],
) -> Result<(), rusqlite::Error> {
    for prefix in preserve_prefixes {
        connection.execute(
            "UPDATE local_files
             SET last_seen_scan_id = ?2
             WHERE installation_id = ?1
               AND (
                    ?3 = ''
                    OR relative_path = ?3
                    OR substr(relative_path, 1, length(?3) + 1) = ?3 || '/'
               )",
            params![installation_id, scan_session_id, prefix],
        )?;
    }

    connection.execute(
        "DELETE FROM local_files
         WHERE installation_id = ?1
           AND (last_seen_scan_id IS NULL OR last_seen_scan_id <> ?2)",
        params![installation_id, scan_session_id],
    )?;

    Ok(())
}

fn file_kind(path: &Path) -> Option<FileKind> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();

    match extension.as_str() {
        "package" => Some(FileKind::Package),
        "ts4script" => Some(FileKind::Ts4Script),
        _ => None,
    }
}

fn relative_string(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn modified_ns(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX))
}

fn load_cache(
    connection: &Connection,
    installation_id: i64,
    relative_path: &str,
) -> Result<Option<LocalCache>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT
                lf.size_bytes,
                lf.modified_ns,
                EXISTS(
                    SELECT 1
                    FROM fingerprints f
                    WHERE f.local_file_id = lf.id
                      AND f.kind = 'sha256'
                )
             FROM local_files lf
             WHERE lf.installation_id = ?1
               AND lf.relative_path = ?2",
            params![installation_id, relative_path],
            |row| {
                Ok(LocalCache {
                    size_bytes: row.get(0)?,
                    modified_ns: row.get(1)?,
                    has_sha256: row.get::<_, i64>(2)? != 0,
                })
            },
        )
        .optional()
}

fn upsert_local_file(
    connection: &Connection,
    installation_id: i64,
    scan_session_id: i64,
    relative_path: &str,
    file_kind: FileKind,
    size_bytes: i64,
    modified_ns: Option<i64>,
) -> Result<i64, rusqlite::Error> {
    let quick_fingerprint = format!(
        "metadata-v1:{size_bytes}:{}",
        modified_ns.map_or_else(|| "unknown".to_string(), |value| value.to_string())
    );

    connection.query_row(
        "INSERT INTO local_files (
            installation_id,
            relative_path,
            file_kind,
            first_seen_scan_id,
            last_seen_scan_id,
            size_bytes,
            modified_ns,
            quick_fingerprint
         )
         VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7)
         ON CONFLICT(installation_id, relative_path) DO UPDATE SET
            file_kind = excluded.file_kind,
            last_seen_scan_id = excluded.last_seen_scan_id,
            size_bytes = excluded.size_bytes,
            modified_ns = excluded.modified_ns,
            quick_fingerprint = excluded.quick_fingerprint
         RETURNING id",
        params![
            installation_id,
            relative_path,
            file_kind.database_value(),
            scan_session_id,
            size_bytes,
            modified_ns,
            quick_fingerprint
        ],
        |row| row.get(0),
    )
}

fn delete_sha256(connection: &Connection, local_file_id: i64) -> Result<(), rusqlite::Error> {
    connection.execute(
        "DELETE FROM fingerprints
         WHERE local_file_id = ?1 AND kind = 'sha256'",
        [local_file_id],
    )?;
    Ok(())
}

fn store_sha256(
    connection: &Connection,
    local_file_id: i64,
    hash: &str,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "INSERT INTO fingerprints (
            local_file_id,
            kind,
            value,
            algorithm_version,
            computed_at
         )
         VALUES (?1, 'sha256', ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![local_file_id, hash, HASH_ALGORITHM_VERSION],
    )?;
    Ok(())
}

fn sha256_file(path: &Path, control: &ScannerControl) -> Result<Option<String>, io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_SIZE];

    loop {
        if control.is_cancelled() {
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

fn record_depth_observation(
    connection: &Connection,
    scan_session_id: i64,
    relative_path: &str,
    kind: FileKind,
    progress: &mut ScanProgress,
) -> Result<(), rusqlite::Error> {
    let depth = Path::new(relative_path)
        .parent()
        .map(|parent| parent.components().count())
        .unwrap_or(0);

    let invalid = match kind {
        FileKind::Ts4Script => depth > 1,
        FileKind::Package => depth > 5,
    };

    if invalid {
        insert_observation(
            connection,
            scan_session_id,
            &PendingObservation {
                relative_path: Some(relative_path.to_string()),
                kind: "invalidDepth",
                detail: match kind {
                    FileKind::Ts4Script => {
                        format!("Script mod is {depth} folders deep; supported depth is at most 1.")
                    }
                    FileKind::Package => {
                        format!("Package is {depth} folders deep; supported depth is at most 5.")
                    }
                },
            },
        )?;
        progress.observations += 1;
    }

    Ok(())
}

fn observation_kind_for_io(error: &io::Error) -> &'static str {
    match error.kind() {
        io::ErrorKind::NotFound => "disappeared",
        io::ErrorKind::PermissionDenied => "permissionDenied",
        _ => "ioError",
    }
}

fn insert_observation(
    connection: &Connection,
    scan_session_id: i64,
    observation: &PendingObservation,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "INSERT INTO scan_observations (
            scan_session_id,
            relative_path,
            kind,
            detail
         )
         VALUES (?1, ?2, ?3, ?4)",
        params![
            scan_session_id,
            observation.relative_path.as_deref(),
            observation.kind,
            observation.detail.as_str()
        ],
    )?;
    Ok(())
}

fn upsert_installation(
    connection: &Connection,
    root: &Path,
    mods_root: &Path,
    game_version: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    let root = root.to_string_lossy().into_owned();
    let mods_root = mods_root.to_string_lossy().into_owned();

    connection.query_row(
        "INSERT INTO installations (
            game_root,
            mods_root,
            platform,
            game_version,
            discovered_at,
            last_seen_at
         )
         VALUES (
            ?1,
            ?2,
            'windows',
            ?3,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         )
         ON CONFLICT(game_root, mods_root) DO UPDATE SET
            game_version = excluded.game_version,
            last_seen_at = excluded.last_seen_at
         RETURNING id",
        params![root, mods_root, game_version],
        |row| row.get(0),
    )
}

fn create_scan_session(
    connection: &Connection,
    installation_id: i64,
    mode: ScanMode,
) -> Result<i64, rusqlite::Error> {
    connection.query_row(
        "INSERT INTO scan_sessions (
            installation_id,
            started_at,
            status,
            mode
         )
         VALUES (
            ?1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
            'running',
            ?2
         )
         RETURNING id",
        params![installation_id, mode.database_value()],
        |row| row.get(0),
    )
}

fn finish_scan_session(
    connection: &Connection,
    summary: &ScanSummary,
) -> Result<(), rusqlite::Error> {
    let status = match summary.status {
        ScanStatus::Completed => "completed",
        ScanStatus::Cancelled => "cancelled",
    };

    connection.execute(
        "UPDATE scan_sessions
         SET completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             status = ?2,
             files_seen = ?3,
             files_hashed = ?4,
             error_count = ?5
         WHERE id = ?1",
        params![
            summary.scan_session_id,
            status,
            counter_to_i64(summary.files_seen),
            counter_to_i64(summary.files_hashed),
            counter_to_i64(summary.observations)
        ],
    )?;
    Ok(())
}

fn mark_scan_failed(connection: &Connection, scan_session_id: i64) -> Result<(), rusqlite::Error> {
    connection.execute(
        "UPDATE scan_sessions
         SET completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             status = 'failed',
             error_count = error_count + 1
         WHERE id = ?1",
        [scan_session_id],
    )?;
    Ok(())
}

fn counter_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn emit_progress_if_needed<F>(progress: &ScanProgress, on_progress: &mut F)
where
    F: FnMut(&ScanProgress),
{
    if progress.files_seen == 1 || progress.files_seen % PROGRESS_INTERVAL == 0 {
        on_progress(progress);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new().expect("create scanner temp directory");
        let sims_root = temp.path().join("The Sims 4");
        let mods_root = sims_root.join("Mods");
        fs::create_dir_all(&mods_root).expect("create Mods directory");
        fs::write(sims_root.join("GameVersion.txt"), "1.128.90.1030").expect("write game version");
        let database_path = temp.path().join("scanner.sqlite3");
        storage::initialize(&database_path).expect("initialize scanner database");
        (temp, sims_root, database_path)
    }

    fn write_mod(mods_root: &Path, relative: &str, content: &[u8]) {
        let path = mods_root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create mod parent");
        }
        fs::write(path, content).expect("write mod fixture");
    }

    fn current_files(database_path: &Path) -> Vec<String> {
        let connection = storage::open(database_path).expect("open scanner database");
        let mut statement = connection
            .prepare("SELECT relative_path FROM local_files ORDER BY relative_path")
            .expect("prepare inventory query");

        statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query inventory")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect inventory")
    }

    #[test]
    fn inventory_is_recursive_and_deterministic() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");
        write_mod(&mods_root, "z.package", b"z");
        write_mod(&mods_root, "Creator/a.ts4script", b"a");
        write_mod(&mods_root, "Creator/Nested/b.package", b"b");
        write_mod(&mods_root, "ignore.txt", b"ignored");

        let control = ScannerControl::default();
        let summary = scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("scan succeeds");

        assert_eq!(summary.status, ScanStatus::Completed);
        assert_eq!(summary.files_seen, 3);
        assert_eq!(
            current_files(&database_path),
            vec![
                "Creator/Nested/b.package",
                "Creator/a.ts4script",
                "z.package"
            ]
        );
    }

    #[test]
    fn incremental_rescan_skips_unchanged_hashes() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");
        write_mod(&mods_root, "a.package", b"alpha");
        write_mod(&mods_root, "b.ts4script", b"beta");

        let control = ScannerControl::default();
        let first = scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("initial scan");
        let second = scan_path(
            &database_path,
            &sims_root,
            ScanMode::Incremental,
            &control,
            |_| {},
        )
        .expect("incremental scan");

        assert_eq!(first.files_hashed, 2);
        assert_eq!(second.files_hashed, 0);
        assert_eq!(second.files_skipped, 2);
    }

    #[test]
    fn full_scan_bypasses_incremental_cache() {
        let (_temp, sims_root, database_path) = fixture();
        write_mod(&sims_root.join("Mods"), "a.package", b"alpha");

        let control = ScannerControl::default();
        scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("initial scan");
        let verification = scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("full verification scan");

        assert_eq!(verification.files_hashed, 1);
        assert_eq!(verification.files_skipped, 0);
    }

    #[test]
    fn cancelled_scan_rolls_back_partial_inventory() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");
        write_mod(&mods_root, "stable.package", b"stable");

        let control = ScannerControl::default();
        scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("baseline scan");

        write_mod(&mods_root, "new-a.package", b"a");
        write_mod(&mods_root, "new-b.package", b"b");

        let cancelled = scan_path(
            &database_path,
            &sims_root,
            ScanMode::Full,
            &control,
            |progress| {
                if progress.files_seen >= 1 {
                    control.cancel();
                }
            },
        )
        .expect("cancelled scan returns summary");

        assert_eq!(cancelled.status, ScanStatus::Cancelled);
        assert_eq!(current_files(&database_path), vec!["stable.package"]);

        let connection = storage::open(&database_path).expect("open scanner database");
        let status: String = connection
            .query_row(
                "SELECT status FROM scan_sessions WHERE id = ?1",
                [cancelled.scan_session_id],
                |row| row.get(0),
            )
            .expect("cancelled session status");
        assert_eq!(status, "cancelled");
    }

    #[test]
    fn completed_scan_removes_files_that_disappeared_since_previous_scan() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");
        write_mod(&mods_root, "keep.package", b"keep");
        write_mod(&mods_root, "remove.package", b"remove");

        let control = ScannerControl::default();
        scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("initial scan");

        fs::remove_file(mods_root.join("remove.package")).expect("remove fixture");
        scan_path(
            &database_path,
            &sims_root,
            ScanMode::Incremental,
            &control,
            |_| {},
        )
        .expect("second scan");

        assert_eq!(current_files(&database_path), vec!["keep.package"]);
    }

    #[test]
    fn unreadable_prefix_preserves_last_known_inventory_during_cleanup() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");
        write_mod(&mods_root, "Locked/keep.package", b"keep");
        write_mod(&mods_root, "Elsewhere/remove.package", b"remove");

        let control = ScannerControl::default();
        scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("baseline scan");

        let mut connection = storage::open(&database_path).expect("open scanner database");
        let installation_id: i64 = connection
            .query_row("SELECT id FROM installations LIMIT 1", [], |row| row.get(0))
            .expect("installation id");
        let scan_session_id =
            create_scan_session(&connection, installation_id, ScanMode::Incremental)
                .expect("create cleanup session");

        let transaction = connection.transaction().expect("start cleanup transaction");
        finalize_inventory(
            &transaction,
            installation_id,
            scan_session_id,
            &["Locked".to_string()],
        )
        .expect("finalize with protected prefix");
        transaction.commit().expect("commit cleanup transaction");

        assert_eq!(current_files(&database_path), vec!["Locked/keep.package"]);
    }

    #[test]
    fn invalid_depths_become_observations_not_fatal_errors() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");
        write_mod(&mods_root, "A/B/deep.ts4script", b"script");
        write_mod(&mods_root, "1/2/3/4/5/6/deep.package", b"package");

        let control = ScannerControl::default();
        let summary = scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("scan succeeds with observations");

        assert_eq!(summary.files_seen, 2);
        assert_eq!(summary.observations, 2);

        let connection = storage::open(&database_path).expect("open scanner database");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM scan_observations WHERE kind = 'invalidDepth'",
                [],
                |row| row.get(0),
            )
            .expect("count depth observations");
        assert_eq!(count, 2);
    }

    #[test]
    fn io_failures_are_classified_as_recoverable_observations() {
        assert_eq!(
            observation_kind_for_io(&io::Error::from(io::ErrorKind::NotFound)),
            "disappeared"
        );
        assert_eq!(
            observation_kind_for_io(&io::Error::from(io::ErrorKind::PermissionDenied)),
            "permissionDenied"
        );
        assert_eq!(
            observation_kind_for_io(&io::Error::other("other")),
            "ioError"
        );
    }

    #[test]
    #[ignore = "large-library benchmark; run in dedicated scanner benchmark workflow"]
    fn benchmark_5000_file_full_and_incremental_scan() {
        let (_temp, sims_root, database_path) = fixture();
        let mods_root = sims_root.join("Mods");

        for index in 0..5_000 {
            let relative = format!("Creator{:02}/item-{index:05}.package", index % 25);
            write_mod(&mods_root, &relative, format!("fixture-{index}").as_bytes());
        }

        let control = ScannerControl::default();

        let full_started = Instant::now();
        let full = scan_path(&database_path, &sims_root, ScanMode::Full, &control, |_| {})
            .expect("full benchmark scan");
        let full_elapsed = full_started.elapsed();

        let incremental_started = Instant::now();
        let incremental = scan_path(
            &database_path,
            &sims_root,
            ScanMode::Incremental,
            &control,
            |_| {},
        )
        .expect("incremental benchmark scan");
        let incremental_elapsed = incremental_started.elapsed();

        eprintln!(
            "SCANNER_BENCHMARK files=5000 full_ms={} incremental_ms={} full_hashed={} incremental_hashed={} incremental_skipped={}",
            full_elapsed.as_millis(),
            incremental_elapsed.as_millis(),
            full.files_hashed,
            incremental.files_hashed,
            incremental.files_skipped
        );

        assert_eq!(full.files_hashed, 5_000);
        assert_eq!(incremental.files_hashed, 0);
        assert_eq!(incremental.files_skipped, 5_000);
    }
}
