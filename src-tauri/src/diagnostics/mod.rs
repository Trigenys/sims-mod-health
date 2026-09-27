use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    error::Error,
    fmt::{Display, Formatter},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    dbpf::{self, ResourceKey},
    fingerprint,
    registry::{
        ArtifactResolution, RegistryArtifactProbe, RegistryClient, RegistryFingerprintProbe,
        RegistryIdentityHints,
    },
    storage::{self, StorageError},
    ts4script,
};

const MAX_REPORT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_REPORTS: usize = 20;
const MAX_OBSERVATIONS: usize = 128;
const MAX_CONTEXT_BYTES: usize = 260;
const DEFAULT_REGISTRY_URL: &str = "http://127.0.0.1:8000";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticObservation {
    pub(crate) kind: String,
    pub(crate) value: String,
    pub(crate) context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticEvidence {
    pub(crate) kind: String,
    pub(crate) reference: String,
    pub(crate) explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CanonicalDiagnosticIdentity {
    pub(crate) mod_id: String,
    pub(crate) release_id: String,
    pub(crate) confidence: String,
    pub(crate) deterministic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticCandidate {
    pub(crate) local_file_id: i64,
    pub(crate) relative_path: String,
    pub(crate) confidence: String,
    pub(crate) confidence_score: u8,
    pub(crate) relationship: String,
    pub(crate) evidence: Vec<DiagnosticEvidence>,
    pub(crate) canonical: Option<CanonicalDiagnosticIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticReportView {
    pub(crate) diagnostic_id: Option<i64>,
    pub(crate) source_kind: String,
    pub(crate) report_name: String,
    pub(crate) parse_status: String,
    pub(crate) observed_at: String,
    pub(crate) content_hash: Option<String>,
    pub(crate) observations: Vec<DiagnosticObservation>,
    pub(crate) candidates: Vec<DiagnosticCandidate>,
    pub(crate) note: String,
    pub(crate) telemetry_preview: DiagnosticTelemetryPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticTelemetryPreview {
    pub(crate) source_kind: String,
    pub(crate) observation_kinds: Vec<String>,
    pub(crate) candidate_count: usize,
    pub(crate) redactions_applied: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticsSnapshot {
    pub(crate) installation_id: Option<i64>,
    pub(crate) registry_state: String,
    pub(crate) registry_detail: String,
    pub(crate) reports: Vec<DiagnosticReportView>,
}

#[derive(Debug, Clone)]
struct InstallationContext {
    id: i64,
    root: PathBuf,
    mods_root: PathBuf,
    registry_url: String,
}

#[derive(Debug, Clone)]
struct ReportFile {
    path: PathBuf,
    report_name: String,
    source_kind: String,
    modified_ms: u128,
}

#[derive(Debug, Clone)]
struct LocalArtifact {
    id: i64,
    relative_path: String,
    absolute_path: PathBuf,
    file_kind: String,
    size_bytes: Option<i64>,
    creator_hint: Option<String>,
    embedded_version: Option<String>,
    fingerprints: Vec<RegistryFingerprintProbe>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedDiagnostics {
    installation_id: i64,
    registry_url: String,
    reports: Vec<DiagnosticReportView>,
    probes: Vec<RegistryArtifactProbe>,
}

#[derive(Debug)]
pub(crate) enum DiagnosticError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Storage(StorageError),
    Registry(crate::registry::RegistryError),
    Serialization(serde_json::Error),
}

impl Display for DiagnosticError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "diagnostic I/O error: {error}"),
            Self::Sqlite(error) => write!(formatter, "diagnostic SQLite error: {error}"),
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::Registry(error) => write!(formatter, "{error}"),
            Self::Serialization(error) => {
                write!(formatter, "diagnostic serialization error: {error}")
            }
        }
    }
}

impl Error for DiagnosticError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Registry(error) => Some(error),
            Self::Serialization(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for DiagnosticError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<rusqlite::Error> for DiagnosticError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<StorageError> for DiagnosticError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<crate::registry::RegistryError> for DiagnosticError {
    fn from(value: crate::registry::RegistryError) -> Self {
        Self::Registry(value)
    }
}

impl From<serde_json::Error> for DiagnosticError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

pub(crate) fn prepare_latest(
    database_path: &Path,
) -> Result<Option<PreparedDiagnostics>, DiagnosticError> {
    let connection = storage::open(database_path)?;
    let Some(installation) = latest_installation(&connection)? else {
        return Ok(None);
    };

    let artifacts = load_local_artifacts(&connection, &installation)?;
    drop(connection);

    let reports = discover_reports(&installation.root)?;
    let mut prepared_reports = Vec::new();
    let mut implicated_ids = BTreeSet::new();

    for report in reports {
        let parsed = analyze_report_file(&report, &installation, &artifacts);
        for candidate in &parsed.candidates {
            implicated_ids.insert(candidate.local_file_id);
        }
        prepared_reports.push(parsed);
    }

    let probes = artifacts
        .iter()
        .filter(|artifact| implicated_ids.contains(&artifact.id))
        .filter_map(artifact_probe)
        .collect::<Vec<_>>();

    Ok(Some(PreparedDiagnostics {
        installation_id: installation.id,
        registry_url: installation.registry_url,
        reports: prepared_reports,
        probes,
    }))
}

pub(crate) async fn resolve_registry(mut prepared: PreparedDiagnostics) -> DiagnosticsSnapshot {
    if prepared.probes.is_empty() {
        return DiagnosticsSnapshot {
            installation_id: Some(prepared.installation_id),
            registry_state: "ready".to_string(),
            registry_detail:
                "Local diagnostic evidence is available; no implicated artifact required registry resolution."
                    .to_string(),
            reports: prepared.reports,
        };
    }

    let client = match RegistryClient::new(prepared.registry_url.clone()) {
        Ok(client) => client,
        Err(error) => {
            return DiagnosticsSnapshot {
                installation_id: Some(prepared.installation_id),
                registry_state: "offline".to_string(),
                registry_detail: error.to_string(),
                reports: prepared.reports,
            };
        }
    };

    match client.resolve_artifacts(&prepared.probes).await {
        Ok(resolutions) => {
            attach_canonical_identities(&mut prepared.reports, &resolutions);
            DiagnosticsSnapshot {
                installation_id: Some(prepared.installation_id),
                registry_state: "ready".to_string(),
                registry_detail:
                    "Local diagnostic candidates were checked against canonical artifact identities."
                        .to_string(),
                reports: prepared.reports,
            }
        }
        Err(error) => DiagnosticsSnapshot {
            installation_id: Some(prepared.installation_id),
            registry_state: if error.is_transport() {
                "offline".to_string()
            } else {
                "partial".to_string()
            },
            registry_detail: error.to_string(),
            reports: prepared.reports,
        },
    }
}

pub(crate) fn empty_snapshot() -> DiagnosticsSnapshot {
    DiagnosticsSnapshot {
        installation_id: None,
        registry_state: "offline".to_string(),
        registry_detail: "No scanned Sims installation is available.".to_string(),
        reports: Vec::new(),
    }
}

pub(crate) fn persist_snapshot(
    database_path: &Path,
    snapshot: &mut DiagnosticsSnapshot,
) -> Result<(), DiagnosticError> {
    let Some(installation_id) = snapshot.installation_id else {
        return Ok(());
    };

    let mut connection = storage::open(database_path)?;
    let transaction = connection.transaction()?;

    for report in &mut snapshot.reports {
        let summary = serde_json::to_string(report)?;

        let existing = transaction
            .query_row(
                "SELECT id
                 FROM diagnostics
                 WHERE installation_id = ?1
                   AND source_kind = ?2
                   AND relative_path = ?3
                   AND content_hash IS ?4
                 ORDER BY id DESC
                 LIMIT 1",
                params![
                    installation_id,
                    report.source_kind,
                    report.report_name,
                    report.content_hash
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        let diagnostic_id = if let Some(id) = existing {
            transaction.execute(
                "UPDATE diagnostics
                 SET parse_status = ?2,
                     summary_json = ?3,
                     observed_at = ?4,
                     parsed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                params![id, report.parse_status, summary, report.observed_at],
            )?;
            id
        } else {
            transaction.execute(
                "INSERT INTO diagnostics (
                    installation_id,
                    scan_session_id,
                    source_kind,
                    relative_path,
                    content_hash,
                    parse_status,
                    summary_json,
                    observed_at,
                    parsed_at
                 ) VALUES (
                    ?1,
                    NULL,
                    ?2,
                    ?3,
                    ?4,
                    ?5,
                    ?6,
                    ?7,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )",
                params![
                    installation_id,
                    report.source_kind,
                    report.report_name,
                    report.content_hash,
                    report.parse_status,
                    summary,
                    report.observed_at
                ],
            )?;
            transaction.last_insert_rowid()
        };

        report.diagnostic_id = Some(diagnostic_id);
    }

    transaction.commit()?;
    Ok(())
}

fn latest_installation(
    connection: &Connection,
) -> Result<Option<InstallationContext>, rusqlite::Error> {
    let registry_url = configured_registry_url(connection);

    connection
        .query_row(
            "SELECT id, game_root, mods_root
             FROM installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| {
                Ok(InstallationContext {
                    id: row.get(0)?,
                    root: PathBuf::from(row.get::<_, String>(1)?),
                    mods_root: PathBuf::from(row.get::<_, String>(2)?),
                    registry_url: registry_url.clone(),
                })
            },
        )
        .optional()
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

fn discover_reports(root: &Path) -> Result<Vec<ReportFile>, DiagnosticError> {
    let mut files = Vec::new();
    inspect_report_directory(root, &mut files)?;

    for name in ["MCCC", "BetterExceptions", "Better Exceptions"] {
        let path = root.join(name);
        if path.is_dir() {
            inspect_report_directory(&path, &mut files)?;
        }
    }

    files.sort_by(|left, right| {
        right
            .modified_ms
            .cmp(&left.modified_ms)
            .then_with(|| left.report_name.cmp(&right.report_name))
    });
    files.truncate(MAX_REPORTS);
    Ok(files)
}

fn inspect_report_directory(
    directory: &Path,
    files: &mut Vec<ReportFile>,
) -> Result<(), DiagnosticError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    for entry in entries.take(256) {
        let entry = entry?;
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            continue;
        }

        let report_name = entry.file_name().to_string_lossy().to_string();
        let Some(source_kind) = classify_report_name(&report_name) else {
            continue;
        };

        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_millis())
            .unwrap_or(0);

        files.push(ReportFile {
            path: entry.path(),
            report_name,
            source_kind: source_kind.to_string(),
            modified_ms,
        });
    }

    Ok(())
}

fn classify_report_name(name: &str) -> Option<&'static str> {
    let normalized = name.to_ascii_lowercase();

    if normalized.contains("lastuiexception") {
        Some("last_ui_exception")
    } else if normalized.contains("mc_lastexception") || normalized.contains("mccc") {
        Some("mccc")
    } else if normalized.contains("betterexception") || normalized.contains("exceptionreport") {
        Some("better_exceptions")
    } else if normalized.contains("lastexception") {
        Some("last_exception")
    } else {
        None
    }
}

fn load_local_artifacts(
    connection: &Connection,
    installation: &InstallationContext,
) -> Result<Vec<LocalArtifact>, DiagnosticError> {
    let mut statement = connection.prepare(
        "SELECT
            lf.id,
            lf.relative_path,
            lf.file_kind,
            lf.size_bytes,
            la.creator_hint,
            la.embedded_version,
            f.kind,
            f.value,
            f.algorithm_version
         FROM local_files lf
         LEFT JOIN local_artifacts la ON la.local_file_id = lf.id
         LEFT JOIN fingerprints f ON f.local_file_id = lf.id
         WHERE lf.installation_id = ?1
           AND lf.enabled = 1
           AND lf.file_kind IN ('package', 'ts4script')
         ORDER BY lf.id, f.kind",
    )?;

    let rows = statement.query_map([installation.id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
        ))
    })?;

    let mut artifacts = Vec::<LocalArtifact>::new();
    let mut by_id = HashMap::<i64, usize>::new();

    for row in rows {
        let (
            id,
            relative_path,
            file_kind,
            size_bytes,
            creator_hint,
            embedded_version,
            fingerprint_kind,
            fingerprint_value,
            algorithm_version,
        ) = row?;

        let index = *by_id.entry(id).or_insert_with(|| {
            artifacts.push(LocalArtifact {
                id,
                absolute_path: installation.mods_root.join(&relative_path),
                relative_path: relative_path.clone(),
                file_kind: file_kind.clone(),
                size_bytes,
                creator_hint: creator_hint.clone(),
                embedded_version: embedded_version.clone(),
                fingerprints: Vec::new(),
            });
            artifacts.len() - 1
        });

        if let (Some(kind), Some(value), Some(algorithm_version)) =
            (fingerprint_kind, fingerprint_value, algorithm_version)
        {
            artifacts[index]
                .fingerprints
                .push(RegistryFingerprintProbe {
                    kind,
                    value,
                    algorithm_version,
                });
        }
    }

    Ok(artifacts)
}

fn analyze_report_file(
    report: &ReportFile,
    installation: &InstallationContext,
    artifacts: &[LocalArtifact],
) -> DiagnosticReportView {
    let observed_at = report_observed_at(report.modified_ms);
    let metadata = match fs::metadata(&report.path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return failed_report(
                report,
                observed_at,
                "error",
                &format!("Unable to inspect report metadata: {error}"),
            );
        }
    };

    if metadata.len() > MAX_REPORT_BYTES {
        return failed_report(
            report,
            observed_at,
            "unsupported",
            "Report exceeds the 4 MiB parser limit.",
        );
    }

    let bytes = match fs::read(&report.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return failed_report(
                report,
                observed_at,
                "error",
                &format!("Unable to read report: {error}"),
            );
        }
    };

    if bytes.is_empty() {
        return failed_report(report, observed_at, "malformed", "Report is empty.");
    }

    let content_hash = fingerprint::sha256_file(&report.path, || false)
        .ok()
        .flatten();

    let raw = String::from_utf8_lossy(&bytes);
    let (sanitized, redactions) = sanitize_report_text(&raw, &installation.root);
    let observations = parse_observations(&sanitized);
    let candidates = resolve_local_candidates(&observations, artifacts);

    let note = if observations.is_empty() {
        "The report parsed, but no supported mod/module/resource reference was found.".to_string()
    } else if candidates.is_empty() {
        "Diagnostic references were normalized, but none map to an enabled local artifact."
            .to_string()
    } else {
        "Candidates are correlated with the report evidence only; this is not a causality claim."
            .to_string()
    };

    DiagnosticReportView {
        diagnostic_id: None,
        source_kind: report.source_kind.clone(),
        report_name: report.report_name.clone(),
        parse_status: "parsed".to_string(),
        observed_at,
        content_hash,
        telemetry_preview: DiagnosticTelemetryPreview {
            source_kind: report.source_kind.clone(),
            observation_kinds: observations
                .iter()
                .map(|observation| observation.kind.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            candidate_count: candidates.len(),
            redactions_applied: redactions,
        },
        observations,
        candidates,
        note,
    }
}

fn failed_report(
    report: &ReportFile,
    observed_at: String,
    status: &str,
    note: &str,
) -> DiagnosticReportView {
    DiagnosticReportView {
        diagnostic_id: None,
        source_kind: report.source_kind.clone(),
        report_name: report.report_name.clone(),
        parse_status: status.to_string(),
        observed_at,
        content_hash: None,
        observations: Vec::new(),
        candidates: Vec::new(),
        note: note.to_string(),
        telemetry_preview: DiagnosticTelemetryPreview {
            source_kind: report.source_kind.clone(),
            observation_kinds: Vec::new(),
            candidate_count: 0,
            redactions_applied: 0,
        },
    }
}

fn parse_observations(text: &str) -> Vec<DiagnosticObservation> {
    let normalized = strip_markup(text);
    let mut observations = Vec::new();
    let mut seen = HashSet::<(String, String)>::new();

    let filename_re = Regex::new(
        r#"(?i)(?:[A-Z]:[\\/][^\r\n"'<>]*[\\/]|(?:\.\.?[\\/])?[^\r\n"'<>]*[\\/])?([A-Za-z0-9_ .@+()-]{1,140}\.(?:package|ts4script))"#,
    )
    .expect("static filename regex");

    for capture in filename_re.captures_iter(&normalized) {
        let Some(value) = capture.get(1).map(|item| item.as_str().trim()) else {
            continue;
        };
        push_observation(
            &mut observations,
            &mut seen,
            "file",
            value,
            context_for(&normalized, capture.get(0).expect("whole match")),
        );
    }

    let python_file_re =
        Regex::new(r#"(?i)(?:File\s+["'][^"']*[\\/])?([A-Za-z_][A-Za-z0-9_]{1,80})\.pyc?["']?"#)
            .expect("static Python module regex");
    for capture in python_file_re.captures_iter(&normalized) {
        let Some(value) = capture.get(1).map(|item| item.as_str()) else {
            continue;
        };
        if is_game_module(value) {
            continue;
        }
        push_observation(
            &mut observations,
            &mut seen,
            "module",
            value,
            context_for(&normalized, capture.get(0).expect("whole match")),
        );
    }

    let module_re = Regex::new(
        r#"(?i)\b(?:module|module_name|script)\s*[:=]\s*["']?([A-Za-z_][A-Za-z0-9_.]{1,120})"#,
    )
    .expect("static module-label regex");
    for capture in module_re.captures_iter(&normalized) {
        let Some(value) = capture.get(1).map(|item| item.as_str()) else {
            continue;
        };
        if is_game_module(value) {
            continue;
        }
        push_observation(
            &mut observations,
            &mut seen,
            "module",
            value,
            context_for(&normalized, capture.get(0).expect("whole match")),
        );
    }

    let resource_re = Regex::new(
        r#"(?i)0x([0-9a-f]{1,8})\s*[:/-]\s*0x([0-9a-f]{1,8})\s*[:/-]\s*0x([0-9a-f]{1,16})"#,
    )
    .expect("static resource-key regex");
    for capture in resource_re.captures_iter(&normalized) {
        let value = format!(
            "0x{}:0x{}:0x{}",
            capture.get(1).expect("type").as_str().to_ascii_lowercase(),
            capture.get(2).expect("group").as_str().to_ascii_lowercase(),
            capture
                .get(3)
                .expect("instance")
                .as_str()
                .to_ascii_lowercase()
        );
        push_observation(
            &mut observations,
            &mut seen,
            "resource",
            &value,
            context_for(&normalized, capture.get(0).expect("whole match")),
        );
    }

    observations.truncate(MAX_OBSERVATIONS);
    observations
}

fn push_observation(
    observations: &mut Vec<DiagnosticObservation>,
    seen: &mut HashSet<(String, String)>,
    kind: &str,
    value: &str,
    context: String,
) {
    let key = (kind.to_string(), value.to_ascii_lowercase());
    if seen.insert(key) {
        observations.push(DiagnosticObservation {
            kind: kind.to_string(),
            value: value.to_string(),
            context,
        });
    }
}

fn strip_markup(text: &str) -> String {
    let tag_re = Regex::new(r"(?is)<[^>]{1,2048}>").expect("static markup regex");
    let entity_re = Regex::new(r"(?i)&(?:nbsp|lt|gt|amp|quot);").expect("static entity regex");
    let without_tags = tag_re.replace_all(text, " ");
    entity_re.replace_all(&without_tags, " ").to_string()
}

fn sanitize_report_text(text: &str, installation_root: &Path) -> (String, usize) {
    let mut sanitized = text.to_string();
    let mut redactions = 0_usize;

    let root_text = installation_root.to_string_lossy();
    if !root_text.is_empty() && sanitized.contains(root_text.as_ref()) {
        redactions += sanitized.matches(root_text.as_ref()).count();
        sanitized = sanitized.replace(root_text.as_ref(), "<sims-root>");
    }

    for pattern in [
        r#"(?i)[A-Z]:\\Users\\[^\\\s"'<>]+"#,
        r#"(?i)/Users/[^/\s"'<>]+"#,
        r#"(?i)/home/[^/\s"'<>]+"#,
        r#"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b"#,
        r#"(?i)\b(?:account|user|player|persona)[ _-]?id\s*[:=]\s*[A-Za-z0-9_-]{6,}\b"#,
    ] {
        let regex = Regex::new(pattern).expect("static redaction regex");
        let count = regex.find_iter(&sanitized).count();
        if count > 0 {
            redactions += count;
            sanitized = regex.replace_all(&sanitized, "<redacted>").to_string();
        }
    }

    (sanitized, redactions)
}

fn context_for(text: &str, matched: regex::Match<'_>) -> String {
    let start = text[..matched.start()]
        .rfind(['\n', '\r'])
        .map(|index| index + 1)
        .unwrap_or(0);
    let end = text[matched.end()..]
        .find(['\n', '\r'])
        .map(|index| matched.end() + index)
        .unwrap_or_else(|| text.len());

    truncate_context(text[start..end].trim(), MAX_CONTEXT_BYTES)
}

fn truncate_context(value: &str, maximum: usize) -> String {
    if value.len() <= maximum {
        return value.to_string();
    }

    let mut end = maximum;
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}…", &value[..end])
}

fn is_game_module(value: &str) -> bool {
    let root = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_lowercase();
    matches!(
        root.as_str(),
        "sims4"
            | "simulation"
            | "server"
            | "services"
            | "objects"
            | "interactions"
            | "relationships"
            | "careers"
            | "statistics"
            | "event_testing"
            | "routing"
    )
}

#[derive(Debug)]
struct CandidateAccumulator {
    artifact: LocalArtifact,
    score: u8,
    evidence: Vec<DiagnosticEvidence>,
    evidence_keys: HashSet<(String, String)>,
}

fn resolve_local_candidates(
    observations: &[DiagnosticObservation],
    artifacts: &[LocalArtifact],
) -> Vec<DiagnosticCandidate> {
    if observations.is_empty() {
        return Vec::new();
    }

    let filename_refs = observations
        .iter()
        .filter(|item| item.kind == "file")
        .map(|item| item.value.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let module_refs = observations
        .iter()
        .filter(|item| item.kind == "module")
        .map(|item| item.value.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let resource_refs = observations
        .iter()
        .filter(|item| item.kind == "resource")
        .filter_map(|item| parse_resource_key(&item.value))
        .collect::<BTreeSet<_>>();

    let mut candidates = HashMap::<i64, CandidateAccumulator>::new();

    for artifact in artifacts {
        let filename = Path::new(&artifact.relative_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&artifact.relative_path)
            .to_ascii_lowercase();

        if filename_refs.contains(&filename) {
            add_candidate_evidence(
                &mut candidates,
                artifact,
                98,
                "filename",
                &filename,
                "Diagnostic report names this installed artifact file directly.",
            );
        }
    }

    if !module_refs.is_empty() {
        for artifact in artifacts
            .iter()
            .filter(|item| item.file_kind == "ts4script")
        {
            let metadata = match ts4script::inspect_path(&artifact.absolute_path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            let module_names = metadata
                .module_names
                .into_iter()
                .map(|value| value.to_ascii_lowercase())
                .collect::<BTreeSet<_>>();
            let package_names = metadata
                .package_names
                .into_iter()
                .map(|value| value.to_ascii_lowercase())
                .collect::<BTreeSet<_>>();

            for reference in &module_refs {
                if module_names.contains(reference) {
                    add_candidate_evidence(
                        &mut candidates,
                        artifact,
                        90,
                        "module",
                        reference,
                        "Diagnostic module exactly matches a module exposed by this installed TS4Script archive.",
                    );
                    continue;
                }

                if package_names.iter().any(|package| {
                    reference == package || reference.starts_with(&(package.clone() + "."))
                }) {
                    add_candidate_evidence(
                        &mut candidates,
                        artifact,
                        82,
                        "module_package",
                        reference,
                        "Diagnostic module falls under a Python package exposed by this installed TS4Script archive.",
                    );
                }
            }
        }
    }

    if !resource_refs.is_empty() {
        for artifact in artifacts.iter().filter(|item| item.file_kind == "package") {
            let metadata = match dbpf::parse_path(&artifact.absolute_path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let keys = metadata.resource_keys().collect::<BTreeSet<_>>();

            for reference in resource_refs.intersection(&keys) {
                let formatted = format_resource_key(*reference);
                add_candidate_evidence(
                    &mut candidates,
                    artifact,
                    84,
                    "resource",
                    &formatted,
                    "Diagnostic resource key exists in this installed DBPF package.",
                );
            }
        }
    }

    let mut result = candidates
        .into_values()
        .map(|mut accumulator| {
            accumulator.evidence.sort_by(|left, right| {
                left.kind
                    .cmp(&right.kind)
                    .then(left.reference.cmp(&right.reference))
            });
            let evidence_bonus = accumulator
                .evidence
                .len()
                .saturating_sub(1)
                .saturating_mul(3)
                .min(9) as u8;
            let score = accumulator.score.saturating_add(evidence_bonus).min(99);

            DiagnosticCandidate {
                local_file_id: accumulator.artifact.id,
                relative_path: accumulator.artifact.relative_path,
                confidence: if score >= 95 {
                    "exact".to_string()
                } else if score >= 80 {
                    "high".to_string()
                } else {
                    "medium".to_string()
                },
                confidence_score: score,
                relationship: "implicated_by_diagnostic_evidence".to_string(),
                evidence: accumulator.evidence,
                canonical: None,
            }
        })
        .collect::<Vec<_>>();

    result.sort_by(|left, right| {
        right
            .confidence_score
            .cmp(&left.confidence_score)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    result
}

fn add_candidate_evidence(
    candidates: &mut HashMap<i64, CandidateAccumulator>,
    artifact: &LocalArtifact,
    score: u8,
    kind: &str,
    reference: &str,
    explanation: &str,
) {
    let accumulator = candidates
        .entry(artifact.id)
        .or_insert_with(|| CandidateAccumulator {
            artifact: artifact.clone(),
            score,
            evidence: Vec::new(),
            evidence_keys: HashSet::new(),
        });
    accumulator.score = accumulator.score.max(score);

    let key = (kind.to_string(), reference.to_ascii_lowercase());
    if accumulator.evidence_keys.insert(key) {
        accumulator.evidence.push(DiagnosticEvidence {
            kind: kind.to_string(),
            reference: reference.to_string(),
            explanation: explanation.to_string(),
        });
    }
}

fn parse_resource_key(value: &str) -> Option<ResourceKey> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return None;
    }

    Some(ResourceKey {
        resource_type: u32::from_str_radix(parts[0].trim_start_matches("0x"), 16).ok()?,
        group: u32::from_str_radix(parts[1].trim_start_matches("0x"), 16).ok()?,
        instance: u64::from_str_radix(parts[2].trim_start_matches("0x"), 16).ok()?,
    })
}

fn format_resource_key(key: ResourceKey) -> String {
    format!(
        "0x{:08x}:0x{:08x}:0x{:016x}",
        key.resource_type, key.group, key.instance
    )
}

fn artifact_probe(artifact: &LocalArtifact) -> Option<RegistryArtifactProbe> {
    if artifact.fingerprints.is_empty() {
        return None;
    }

    let filename = Path::new(&artifact.relative_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string);
    let identity_hints = if artifact.creator_hint.is_some() || artifact.embedded_version.is_some() {
        Some(RegistryIdentityHints {
            creator: artifact.creator_hint.clone(),
            mod_name: None,
            version: artifact.embedded_version.clone(),
        })
    } else {
        None
    };

    Some(RegistryArtifactProbe {
        client_ref: artifact.id.to_string(),
        artifact_kind: Some(artifact.file_kind.clone()),
        filename,
        size_bytes: artifact.size_bytes,
        identity_hints,
        fingerprints: artifact.fingerprints.clone(),
    })
}

fn attach_canonical_identities(
    reports: &mut [DiagnosticReportView],
    resolutions: &[ArtifactResolution],
) {
    let resolution_by_ref = resolutions
        .iter()
        .map(|resolution| (resolution.client_ref.as_str(), resolution))
        .collect::<HashMap<_, _>>();

    for report in reports {
        for candidate in &mut report.candidates {
            let client_ref = candidate.local_file_id.to_string();
            let Some(resolution) = resolution_by_ref.get(client_ref.as_str()) else {
                continue;
            };
            if resolution.status != "resolved" {
                continue;
            }

            let selected = resolution.selected_artifact_id.as_deref();
            let matched = resolution
                .matches
                .iter()
                .find(|item| Some(item.artifact_id.as_str()) == selected);

            if let Some(matched) = matched {
                candidate.canonical = Some(CanonicalDiagnosticIdentity {
                    mod_id: matched.mod_id.clone(),
                    release_id: matched.release_id.clone(),
                    confidence: matched.confidence.clone(),
                    deterministic: matched.deterministic,
                });
            }
        }
    }
}

fn report_observed_at(modified_ms: u128) -> String {
    if modified_ms == 0 {
        return "unknown".to_string();
    }
    format!("unix-ms:{modified_ms}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::TempDir;
    use zip::{
        write::{SimpleFileOptions, ZipWriter},
        CompressionMethod,
    };

    fn ts4script(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::DEFLATE);

        for (name, content) in entries {
            writer.start_file(*name, options).expect("start ZIP entry");
            writer.write_all(content).expect("write ZIP entry");
        }

        writer.finish().expect("finish ZIP").into_inner()
    }

    fn artifact(id: i64, root: &Path, relative_path: &str, kind: &str) -> LocalArtifact {
        LocalArtifact {
            id,
            relative_path: relative_path.to_string(),
            absolute_path: root.join(relative_path),
            file_kind: kind.to_string(),
            size_bytes: None,
            creator_hint: None,
            embedded_version: None,
            fingerprints: Vec::new(),
        }
    }

    #[test]
    fn supported_reports_normalize_file_module_and_resource_observations() {
        let text = r#"
            <report>
              File "C:\Users\Alice\Documents\Electronic Arts\The Sims 4\Mods\Creator\cool_mod.py", line 42
              module: creator.cool_mod
              implicated: My Awesome Mod.package
              resource=0x545a6b4a:0x00000000:0x0102030405060708
            </report>
        "#;

        let (sanitized, redactions) = sanitize_report_text(
            text,
            Path::new(r"C:\Users\Alice\Documents\Electronic Arts\The Sims 4"),
        );
        let observations = parse_observations(&sanitized);

        assert!(redactions > 0);
        assert!(observations.iter().any(|item| item.kind == "module"));
        assert!(observations.iter().any(|item| item.kind == "file"));
        assert!(observations.iter().any(|item| item.kind == "resource"));
        assert!(!serde_json::to_string(&observations)
            .expect("serialize observations")
            .contains("Alice"));
    }

    #[test]
    fn module_reference_resolves_to_installed_ts4script_with_correlation_wording() {
        let temp = TempDir::new().expect("diagnostic temp");
        let relative = "Creator/CoolMod.ts4script";
        let path = temp.path().join(relative);
        fs::create_dir_all(path.parent().expect("script parent")).expect("create script parent");
        fs::write(
            &path,
            ts4script(&[
                ("creator/cool_mod.pyc", b"compiled"),
                ("creator/__init__.pyc", b"init"),
            ]),
        )
        .expect("write TS4Script");

        let observations = vec![DiagnosticObservation {
            kind: "module".to_string(),
            value: "creator.cool_mod".to_string(),
            context: "module: creator.cool_mod".to_string(),
        }];

        let candidates = resolve_local_candidates(
            &observations,
            &[artifact(7, temp.path(), relative, "ts4script")],
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].local_file_id, 7);
        assert_eq!(
            candidates[0].relationship,
            "implicated_by_diagnostic_evidence"
        );
        assert!(candidates[0].confidence_score >= 80);
    }

    #[test]
    fn direct_filename_reference_is_exact_local_evidence() {
        let observations = vec![DiagnosticObservation {
            kind: "file".to_string(),
            value: "Example.package".to_string(),
            context: "Example.package".to_string(),
        }];
        let artifacts = vec![LocalArtifact {
            id: 3,
            relative_path: "Folder/Example.package".to_string(),
            absolute_path: PathBuf::from("Folder/Example.package"),
            file_kind: "package".to_string(),
            size_bytes: None,
            creator_hint: None,
            embedded_version: None,
            fingerprints: Vec::new(),
        }];

        let candidates = resolve_local_candidates(&observations, &artifacts);

        assert_eq!(candidates[0].confidence, "exact");
        assert_eq!(candidates[0].evidence[0].kind, "filename");
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
    fn resource_reference_resolves_to_installed_package() {
        let temp = TempDir::new().expect("resource diagnostic temp");
        let relative = "Overrides/Example.package";
        let path = temp.path().join(relative);
        fs::create_dir_all(path.parent().expect("package parent")).expect("create package parent");

        let key = ResourceKey {
            resource_type: 0x545a_6b4a,
            group: 0,
            instance: 0x0102_0304_0506_0708,
        };
        fs::write(&path, minimal_dbpf(&[key])).expect("write package");

        let observations = vec![DiagnosticObservation {
            kind: "resource".to_string(),
            value: format_resource_key(key),
            context: "resource reference".to_string(),
        }];

        let candidates = resolve_local_candidates(
            &observations,
            &[artifact(11, temp.path(), relative, "package")],
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].local_file_id, 11);
        assert_eq!(candidates[0].evidence[0].kind, "resource");
        assert!(candidates[0].confidence_score >= 80);
    }

    #[test]
    fn report_families_are_classified_without_generic_overrides() {
        assert_eq!(
            classify_report_name("lastException_123.txt"),
            Some("last_exception")
        );
        assert_eq!(
            classify_report_name("lastUIException.txt"),
            Some("last_ui_exception")
        );
        assert_eq!(classify_report_name("mc_lastexception.html"), Some("mccc"));
        assert_eq!(
            classify_report_name("BetterExceptions_ExceptionReport.html"),
            Some("better_exceptions")
        );
    }

    #[test]
    fn empty_report_is_malformed_without_panicking() {
        let temp = TempDir::new().expect("empty diagnostic temp");
        let root = temp.path();
        let report_path = root.join("lastException.txt");
        fs::write(&report_path, b"").expect("write empty report");
        let report = ReportFile {
            path: report_path,
            report_name: "lastException.txt".to_string(),
            source_kind: "last_exception".to_string(),
            modified_ms: 1,
        };
        let installation = InstallationContext {
            id: 1,
            root: root.to_path_buf(),
            mods_root: root.join("Mods"),
            registry_url: DEFAULT_REGISTRY_URL.to_string(),
        };

        let parsed = analyze_report_file(&report, &installation, &[]);

        assert_eq!(parsed.parse_status, "malformed");
        assert!(parsed.candidates.is_empty());
    }

    #[test]
    fn oversized_report_is_rejected_before_content_parsing() {
        let temp = TempDir::new().expect("oversized diagnostic temp");
        let root = temp.path();
        let report_path = root.join("lastException.txt");
        let file = std::fs::File::create(&report_path).expect("create oversized report");
        file.set_len(MAX_REPORT_BYTES + 1)
            .expect("size oversized report");

        let report = ReportFile {
            path: report_path,
            report_name: "lastException.txt".to_string(),
            source_kind: "last_exception".to_string(),
            modified_ms: 1,
        };
        let installation = InstallationContext {
            id: 1,
            root: root.to_path_buf(),
            mods_root: root.join("Mods"),
            registry_url: DEFAULT_REGISTRY_URL.to_string(),
        };

        let parsed = analyze_report_file(&report, &installation, &[]);

        assert_eq!(parsed.parse_status, "unsupported");
        assert!(parsed.observations.is_empty());
        assert!(parsed.candidates.is_empty());
    }

    #[test]
    fn telemetry_preview_does_not_contain_paths_email_or_report_content() {
        let text = r#"
            C:\Users\Jennifer\Documents\Electronic Arts\The Sims 4
            email=jennifer@example.com
            playerId=1234567890
            module: creator_mod
        "#;
        let (sanitized, redactions) = sanitize_report_text(
            text,
            Path::new(r"C:\Users\Jennifer\Documents\Electronic Arts\The Sims 4"),
        );
        let observations = parse_observations(&sanitized);
        let preview = DiagnosticTelemetryPreview {
            source_kind: "last_exception".to_string(),
            observation_kinds: observations.iter().map(|item| item.kind.clone()).collect(),
            candidate_count: 0,
            redactions_applied: redactions,
        };
        let serialized = serde_json::to_string(&preview).expect("serialize telemetry preview");

        assert!(!serialized.contains("Jennifer"));
        assert!(!serialized.contains("jennifer@example.com"));
        assert!(!serialized.contains("1234567890"));
        assert!(redactions >= 3);
    }
}
