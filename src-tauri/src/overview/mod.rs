use std::{
    collections::{BTreeSet, HashMap, HashSet},
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::{
    conflicts,
    registry::{
        configured_registry_url, ArtifactResolution, HealthAssessment, RegistryArtifactProbe,
        RegistryClient, RegistryFingerprintProbe, RegistryIdentityHints, RelationshipResponse,
    },
    storage::{self, StorageError},
};

const ATTENTION_LIMIT: usize = 6;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewHealthCounts {
    pub(crate) healthy: u64,
    pub(crate) updates: u64,
    pub(crate) conflicts: u64,
    pub(crate) unknown: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewInstallationCounts {
    pub(crate) script_mods: u64,
    pub(crate) package_files: u64,
    pub(crate) unidentified: Option<u64>,
    pub(crate) exact_duplicates: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewScanState {
    pub(crate) scan_session_id: Option<i64>,
    pub(crate) status: String,
    pub(crate) started_at: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) files_seen: u64,
    pub(crate) files_hashed: u64,
    pub(crate) files_skipped: u64,
    pub(crate) observations: u64,
    pub(crate) stale: bool,
    pub(crate) partial: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewAttentionItem {
    pub(crate) name: String,
    pub(crate) creator: String,
    pub(crate) detail: String,
    pub(crate) badge: String,
    pub(crate) tone: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverviewSnapshot {
    pub(crate) has_installation: bool,
    pub(crate) game_version: Option<String>,
    pub(crate) platform: String,
    pub(crate) indexed_count: u64,
    pub(crate) health_score: Option<u8>,
    pub(crate) health_score_explanation: String,
    pub(crate) health_counts: OverviewHealthCounts,
    pub(crate) attention_count: u64,
    pub(crate) attention: Vec<OverviewAttentionItem>,
    pub(crate) installation: OverviewInstallationCounts,
    pub(crate) registry_state: String,
    pub(crate) registry_detail: String,
    pub(crate) scan: OverviewScanState,
}

#[derive(Debug, Clone)]
struct LocalInstallation {
    id: i64,
    game_root: PathBuf,
    mods_root: PathBuf,
    platform: String,
    game_version: Option<String>,
}

#[derive(Debug, Clone)]
struct ProbeBinding {
    local_file_id: i64,
    relative_path: String,
    probe: RegistryArtifactProbe,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalOverviewContext {
    installation: Option<LocalInstallation>,
    indexed_count: u64,
    enabled_file_count: u64,
    script_mods: u64,
    package_files: u64,
    scan: OverviewScanState,
    probes: Vec<ProbeBinding>,
    exact_duplicates: Vec<crate::fingerprint::ExactDuplicateGroup>,
    resource_overlaps: Vec<conflicts::ResourceOverlapFinding>,
    local_analysis_partial: bool,
    registry_base_url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoverySeed {
    pub(crate) patch_version: String,
    pub(crate) registry_base_url: String,
    pub(crate) installed_release_ids: Vec<String>,
    pub(crate) registry_checked: bool,
}

#[derive(Debug)]
pub(crate) enum OverviewError {
    Storage(StorageError),
    Sqlite(rusqlite::Error),
}

impl Display for OverviewError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "{error}"),
            Self::Sqlite(error) => write!(formatter, "overview SQLite error: {error}"),
        }
    }
}

impl Error for OverviewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Sqlite(error) => Some(error),
        }
    }
}

impl From<StorageError> for OverviewError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<rusqlite::Error> for OverviewError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

pub(crate) fn load_local_context(
    database_path: &Path,
) -> Result<LocalOverviewContext, OverviewError> {
    let connection = storage::open(database_path)?;
    let registry_base_url = configured_registry_url(&connection);

    let installation = latest_installation(&connection)?;
    let Some(installation) = installation else {
        return Ok(LocalOverviewContext {
            installation: None,
            indexed_count: 0,
            enabled_file_count: 0,
            script_mods: 0,
            package_files: 0,
            scan: empty_scan(),
            probes: Vec::new(),
            exact_duplicates: Vec::new(),
            resource_overlaps: Vec::new(),
            local_analysis_partial: false,
            registry_base_url,
        });
    };

    let scan = latest_scan(&connection, installation.id)?;
    let (indexed_count, enabled_file_count, script_mods, package_files) =
        local_file_counts(&connection, installation.id)?;
    let probes = load_registry_probes(&connection, installation.id)?;

    let (exact_duplicates, resource_overlaps, analysis_partial) =
        match conflicts::analyze_installation(&connection, installation.id, &installation.mods_root)
        {
            Ok(analysis) => (
                analysis.exact_duplicates,
                analysis.resource_overlaps,
                !analysis.parse_failures.is_empty() || analysis.overlap_pairs_truncated,
            ),
            Err(_) => (
                crate::fingerprint::exact_duplicate_groups(&connection, installation.id)
                    .unwrap_or_default(),
                Vec::new(),
                true,
            ),
        };

    Ok(LocalOverviewContext {
        installation: Some(installation),
        indexed_count,
        enabled_file_count,
        script_mods,
        package_files,
        scan,
        probes,
        exact_duplicates,
        resource_overlaps,
        local_analysis_partial: analysis_partial,
        registry_base_url,
    })
}

pub(crate) fn latest_installation_root(
    database_path: &Path,
) -> Result<Option<PathBuf>, OverviewError> {
    let connection = storage::open(database_path)?;
    Ok(latest_installation(&connection)?.map(|installation| installation.game_root))
}

pub(crate) async fn discovery_seed(database_path: &Path) -> Result<Option<DiscoverySeed>, String> {
    let local = load_local_context(database_path).map_err(|error| error.to_string())?;
    let Some(installation) = local.installation.clone() else {
        return Ok(None);
    };
    let Some(patch_version) = installation.game_version.clone() else {
        return Ok(None);
    };

    if local.probes.is_empty() {
        return Ok(Some(DiscoverySeed {
            patch_version,
            registry_base_url: local.registry_base_url,
            installed_release_ids: Vec::new(),
            registry_checked: false,
        }));
    }

    let client =
        RegistryClient::new(local.registry_base_url.clone()).map_err(|error| error.to_string())?;
    let probes = local
        .probes
        .iter()
        .map(|binding| binding.probe.clone())
        .collect::<Vec<_>>();
    let resolutions = client
        .resolve_artifacts(&probes)
        .await
        .map_err(|error| error.to_string())?;
    let resolution = resolve_registry_identities(&local, &resolutions);
    let installed_release_ids = resolution
        .release_names
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(Some(DiscoverySeed {
        patch_version,
        registry_base_url: local.registry_base_url,
        installed_release_ids,
        registry_checked: true,
    }))
}

pub(crate) async fn build_snapshot(local: LocalOverviewContext) -> OverviewSnapshot {
    let Some(installation) = local.installation.clone() else {
        return empty_snapshot(local.scan);
    };

    let mut ranked_attention = local_attention(&local);
    let mut health_counts = OverviewHealthCounts {
        healthy: 0,
        updates: 0,
        conflicts: (local.exact_duplicates.len() + local.resource_overlaps.len()) as u64,
        unknown: 0,
    };

    let base_installation = OverviewInstallationCounts {
        script_mods: local.script_mods,
        package_files: local.package_files,
        unidentified: None,
        exact_duplicates: local.exact_duplicates.len() as u64,
    };

    let patch_version = installation.game_version.clone();
    let Some(patch_version) = patch_version else {
        ranked_attention.push(RankedAttention::new(
            5,
            "Game patch not detected",
            "Local scan",
            "Compatibility cannot be evaluated until the installed game patch is available.",
            "Patch unknown",
            "muted",
        ));
        return finalize_snapshot(
            &local,
            &installation,
            None,
            health_counts,
            base_installation,
            "partial",
            "Local scan is available, but the installed game patch could not be read.",
            ranked_attention,
        );
    };

    if local.probes.is_empty() {
        return finalize_snapshot(
            &local,
            &installation,
            None,
            health_counts,
            OverviewInstallationCounts {
                unidentified: Some(local.enabled_file_count),
                ..base_installation
            },
            "ready",
            "No fingerprinted enabled artifacts are available for registry evaluation yet.",
            ranked_attention,
        );
    }

    let client = match RegistryClient::new(local.registry_base_url.clone()) {
        Ok(client) => client,
        Err(error) => {
            return finalize_snapshot(
                &local,
                &installation,
                None,
                OverviewHealthCounts {
                    unknown: local.enabled_file_count,
                    ..health_counts
                },
                OverviewInstallationCounts {
                    unidentified: None,
                    ..base_installation
                },
                "offline",
                &error.to_string(),
                ranked_attention,
            );
        }
    };

    let resolutions = match client
        .resolve_artifacts(
            &local
                .probes
                .iter()
                .map(|binding| binding.probe.clone())
                .collect::<Vec<_>>(),
        )
        .await
    {
        Ok(resolutions) => resolutions,
        Err(error) => {
            return finalize_snapshot(
                &local,
                &installation,
                None,
                OverviewHealthCounts {
                    unknown: local.enabled_file_count,
                    ..health_counts
                },
                OverviewInstallationCounts {
                    unidentified: None,
                    ..base_installation
                },
                if error.is_transport() {
                    "offline"
                } else {
                    "partial"
                },
                &error.to_string(),
                ranked_attention,
            );
        }
    };

    let resolution = resolve_registry_identities(&local, &resolutions);
    health_counts.unknown = resolution.unidentified_files;

    for unresolved in resolution.unresolved_attention.iter().take(2) {
        ranked_attention.push(unresolved.clone());
    }

    let release_ids = resolution
        .release_names
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let installation_counts = OverviewInstallationCounts {
        unidentified: Some(resolution.unidentified_files),
        ..base_installation
    };

    if release_ids.is_empty() {
        return finalize_snapshot(
            &local,
            &installation,
            Some(0),
            health_counts,
            installation_counts,
            "ready",
            "Registry lookup completed, but no enabled artifact resolved to a canonical release.",
            ranked_attention,
        );
    }

    let health = match client.evaluate_health(&patch_version, &release_ids).await {
        Ok(health) => health,
        Err(error) => {
            return finalize_snapshot(
                &local,
                &installation,
                None,
                health_counts,
                installation_counts,
                "partial",
                &format!(
                    "Artifact identity resolved, but health evaluation is incomplete: {error}"
                ),
                ranked_attention,
            );
        }
    };

    let relationships = match client.evaluate_relationships(&release_ids).await {
        Ok(relationships) => Some(relationships),
        Err(error) => {
            let score = apply_health(
                &health,
                &resolution.release_names,
                resolution.unidentified_files,
                &mut health_counts,
                &mut ranked_attention,
            );
            return finalize_snapshot(
                &local,
                &installation,
                score,
                health_counts,
                installation_counts,
                "partial",
                &format!(
                    "Compatibility is current, but dependency/conflict data is partial: {error}"
                ),
                ranked_attention,
            );
        }
    };

    let score = apply_health(
        &health,
        &resolution.release_names,
        resolution.unidentified_files,
        &mut health_counts,
        &mut ranked_attention,
    );

    if let Some(relationships) = relationships.as_ref() {
        apply_relationships(
            relationships,
            &resolution.release_names,
            &mut health_counts,
            &mut ranked_attention,
        );
    }

    finalize_snapshot(
        &local,
        &installation,
        score,
        health_counts,
        installation_counts,
        if local.local_analysis_partial || local.scan.partial {
            "partial"
        } else {
            "ready"
        },
        if local.local_analysis_partial || local.scan.partial {
            "Registry health is available, but the current local scan contains partial observations."
        } else {
            "Local scan, artifact identity and registry health are current."
        },
        ranked_attention,
    )
}

fn latest_installation(
    connection: &Connection,
) -> Result<Option<LocalInstallation>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT id, game_root, mods_root, platform, game_version
             FROM installations
             ORDER BY last_seen_at DESC, id DESC
             LIMIT 1",
            [],
            |row| {
                Ok(LocalInstallation {
                    id: row.get(0)?,
                    game_root: PathBuf::from(row.get::<_, String>(1)?),
                    mods_root: PathBuf::from(row.get::<_, String>(2)?),
                    platform: row.get(3)?,
                    game_version: row.get(4)?,
                })
            },
        )
        .optional()
}

fn latest_scan(
    connection: &Connection,
    installation_id: i64,
) -> Result<OverviewScanState, rusqlite::Error> {
    connection
        .query_row(
            "SELECT
                id,
                started_at,
                completed_at,
                status,
                files_seen,
                files_hashed,
                files_skipped,
                observation_count,
                COALESCE(
                    CASE
                        WHEN status = 'running' THEN 0
                        WHEN completed_at IS NULL THEN 1
                        WHEN julianday('now') - julianday(completed_at) > 1 THEN 1
                        ELSE 0
                    END,
                    1
                )
             FROM scan_sessions
             WHERE installation_id = ?1
             ORDER BY started_at DESC, id DESC
             LIMIT 1",
            [installation_id],
            |row| {
                let status: String = row.get(3)?;
                let observations = row.get::<_, i64>(7)?.max(0) as u64;
                Ok(OverviewScanState {
                    scan_session_id: Some(row.get(0)?),
                    started_at: row.get(1)?,
                    completed_at: row.get(2)?,
                    files_seen: row.get::<_, i64>(4)?.max(0) as u64,
                    files_hashed: row.get::<_, i64>(5)?.max(0) as u64,
                    files_skipped: row.get::<_, i64>(6)?.max(0) as u64,
                    observations,
                    stale: row.get::<_, i64>(8)? != 0,
                    partial: status != "completed" || observations > 0,
                    status,
                })
            },
        )
        .optional()
        .map(|scan| scan.unwrap_or_else(empty_scan))
}

fn empty_scan() -> OverviewScanState {
    OverviewScanState {
        scan_session_id: None,
        status: "empty".to_string(),
        started_at: None,
        completed_at: None,
        files_seen: 0,
        files_hashed: 0,
        files_skipped: 0,
        observations: 0,
        stale: false,
        partial: false,
    }
}

fn local_file_counts(
    connection: &Connection,
    installation_id: i64,
) -> Result<(u64, u64, u64, u64), rusqlite::Error> {
    connection.query_row(
        "SELECT
            COUNT(*),
            SUM(CASE WHEN enabled = 1 THEN 1 ELSE 0 END),
            SUM(CASE WHEN enabled = 1 AND file_kind = 'ts4script' THEN 1 ELSE 0 END),
            SUM(CASE WHEN enabled = 1 AND file_kind = 'package' THEN 1 ELSE 0 END)
         FROM local_files
         WHERE installation_id = ?1",
        [installation_id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?.max(0) as u64,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0).max(0) as u64,
                row.get::<_, Option<i64>>(2)?.unwrap_or(0).max(0) as u64,
                row.get::<_, Option<i64>>(3)?.unwrap_or(0).max(0) as u64,
            ))
        },
    )
}

fn load_registry_probes(
    connection: &Connection,
    installation_id: i64,
) -> Result<Vec<ProbeBinding>, rusqlite::Error> {
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
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
        ))
    })?;

    let mut bindings: Vec<ProbeBinding> = Vec::new();
    let mut index_by_file: HashMap<i64, usize> = HashMap::new();

    for row in rows {
        let (
            local_file_id,
            relative_path,
            file_kind,
            size_bytes,
            creator_hint,
            embedded_version,
            fingerprint_kind,
            fingerprint_value,
            algorithm_version,
        ) = row?;

        let index = *index_by_file.entry(local_file_id).or_insert_with(|| {
            let filename = Path::new(&relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string);

            let identity_hints = if creator_hint.is_some() || embedded_version.is_some() {
                Some(RegistryIdentityHints {
                    creator: creator_hint.clone(),
                    mod_name: None,
                    version: embedded_version.clone(),
                })
            } else {
                None
            };

            bindings.push(ProbeBinding {
                local_file_id,
                relative_path: relative_path.clone(),
                probe: RegistryArtifactProbe {
                    client_ref: local_file_id.to_string(),
                    artifact_kind: Some(file_kind.clone()),
                    filename,
                    size_bytes,
                    identity_hints,
                    fingerprints: Vec::new(),
                },
            });
            bindings.len() - 1
        });

        if let (Some(kind), Some(value), Some(version)) =
            (fingerprint_kind, fingerprint_value, algorithm_version)
        {
            bindings[index]
                .probe
                .fingerprints
                .push(RegistryFingerprintProbe {
                    kind,
                    value,
                    algorithm_version: version,
                });
        }
    }

    bindings.retain(|binding| !binding.probe.fingerprints.is_empty());
    Ok(bindings)
}

#[derive(Debug)]
struct ResolutionSummary {
    release_names: HashMap<String, String>,
    unidentified_files: u64,
    unresolved_attention: Vec<RankedAttention>,
}

fn resolve_registry_identities(
    local: &LocalOverviewContext,
    resolutions: &[ArtifactResolution],
) -> ResolutionSummary {
    let binding_by_ref = local
        .probes
        .iter()
        .map(|binding| (binding.probe.client_ref.as_str(), binding))
        .collect::<HashMap<_, _>>();

    let mut resolved_file_ids = HashSet::new();
    let mut release_names = HashMap::new();
    let mut unresolved_attention = Vec::new();

    for resolution in resolutions {
        let Some(binding) = binding_by_ref.get(resolution.client_ref.as_str()) else {
            continue;
        };

        if resolution.status == "resolved" {
            let selected = resolution.selected_artifact_id.as_deref();
            let matched = resolution
                .matches
                .iter()
                .find(|candidate| Some(candidate.artifact_id.as_str()) == selected)
                .or_else(|| resolution.matches.first());

            if let Some(matched) = matched {
                resolved_file_ids.insert(binding.local_file_id);
                release_names
                    .entry(matched.release_id.clone())
                    .or_insert_with(|| display_name(&binding.relative_path));
                continue;
            }
        }

        unresolved_attention.push(RankedAttention::new(
            7,
            &display_name(&binding.relative_path),
            "Local library",
            if resolution.status == "ambiguous" {
                "Multiple registry candidates remain plausible; no identity was selected."
            } else {
                "No canonical registry identity could be established from the current evidence."
            },
            if resolution.status == "ambiguous" {
                "Ambiguous"
            } else {
                "Unknown"
            },
            "muted",
        ));
    }

    let unidentified_files = local
        .enabled_file_count
        .saturating_sub(resolved_file_ids.len() as u64);

    ResolutionSummary {
        release_names,
        unidentified_files,
        unresolved_attention,
    }
}

fn apply_health(
    health: &[HealthAssessment],
    release_names: &HashMap<String, String>,
    unidentified_files: u64,
    counts: &mut OverviewHealthCounts,
    attention: &mut Vec<RankedAttention>,
) -> Option<u8> {
    let mut compatible_for_score = 0_u64;

    for item in health {
        if item.compatibility_state == "compatible" {
            compatible_for_score += 1;
        }

        match item.state.as_str() {
            "compatible" => counts.healthy += 1,
            "update_available" => {
                counts.updates += 1;
                let target = item
                    .update
                    .target_version
                    .as_deref()
                    .unwrap_or("a newer release");
                attention.push(RankedAttention::new(
                    5,
                    release_name(release_names, &item.release_id),
                    "Registry health",
                    &format!(
                        "Update available to {target}; installed compatibility is {}.",
                        item.compatibility_state
                    ),
                    "Update",
                    "update",
                ));
            }
            "potential_conflict" => {
                counts.conflicts += 1;
                attention.push(RankedAttention::new(
                    3,
                    release_name(release_names, &item.release_id),
                    "Registry health",
                    "Current-patch evidence reports a potential conflict. Review the evidence before changing files.",
                    "Potential conflict",
                    "warning",
                ));
            }
            "broken" => {
                counts.conflicts += 1;
                attention.push(RankedAttention::new(
                    0,
                    release_name(release_names, &item.release_id),
                    "Registry health",
                    "Current-patch evidence marks this installed release as broken.",
                    "Broken",
                    "danger",
                ));
            }
            "abandoned" => {
                counts.conflicts += 1;
                attention.push(RankedAttention::new(
                    1,
                    release_name(release_names, &item.release_id),
                    "Registry health",
                    "The installed release is marked abandoned by current sourced evidence.",
                    "Abandoned",
                    "danger",
                ));
            }
            "unknown" => {
                counts.unknown += 1;
                attention.push(RankedAttention::new(
                    if item.disputed { 4 } else { 8 },
                    release_name(release_names, &item.release_id),
                    "Registry health",
                    if item.disputed {
                        "Current evidence sources disagree, so compatibility remains Unknown."
                    } else {
                        "No current-patch compatibility evidence is available."
                    },
                    "Unknown",
                    "muted",
                ));
            }
            _ => {}
        }
    }

    let denominator = health.len() as u64 + unidentified_files;
    if denominator == 0 {
        return None;
    }

    Some(
        ((compatible_for_score.saturating_mul(100) + denominator / 2) / denominator).min(100) as u8,
    )
}

fn apply_relationships(
    relationships: &RelationshipResponse,
    release_names: &HashMap<String, String>,
    counts: &mut OverviewHealthCounts,
    attention: &mut Vec<RankedAttention>,
) {
    counts.conflicts += relationships.known_incompatibilities.len() as u64;

    for finding in &relationships.known_incompatibilities {
        let left = release_name(release_names, &finding.left_release_id);
        let right = release_name(release_names, &finding.right_release_id);
        attention.push(RankedAttention::new(
            1,
            left,
            "Dependency graph",
            &format!("Known incompatibility with {right} is active for the installed versions."),
            "Incompatible",
            "danger",
        ));
    }

    for finding in &relationships.dependency_findings {
        let (priority, detail, badge) = match finding.status.as_str() {
            "missing" => (
                2,
                "A required dependency is not installed.",
                "Missing dependency",
            ),
            "outdated" => (
                2,
                "An installed dependency is below the required version.",
                "Dependency update",
            ),
            _ => (
                4,
                "An installed dependency does not satisfy the declared version range.",
                "Dependency review",
            ),
        };

        attention.push(RankedAttention::new(
            priority,
            release_name(release_names, &finding.required_by_release_id),
            "Dependency graph",
            detail,
            badge,
            "warning",
        ));
    }
}

fn local_attention(local: &LocalOverviewContext) -> Vec<RankedAttention> {
    let mut attention = Vec::new();

    for group in &local.exact_duplicates {
        let name = group
            .files
            .first()
            .map(|file| display_name(&file.relative_path))
            .unwrap_or_else(|| "Duplicate files".to_string());
        attention.push(RankedAttention::new(
            1,
            &name,
            "Local scan",
            &format!(
                "{} exact copies share the same SHA-256 fingerprint.",
                group.files.len()
            ),
            "Duplicate",
            "warning",
        ));
    }

    for overlap in &local.resource_overlaps {
        attention.push(RankedAttention::new(
            4,
            &display_name(&overlap.left_relative_path),
            "Local scan",
            &format!(
                "Potential conflict with {} across {} shared DBPF resource keys.",
                display_name(&overlap.right_relative_path),
                overlap.shared_resource_count
            ),
            "Potential conflict",
            "warning",
        ));
    }

    if local.scan.stale {
        attention.push(RankedAttention::new(
            3,
            "Scan data is stale",
            "Local scan",
            "The latest completed scan is older than 24 hours. Run an incremental scan before relying on the current picture.",
            "Stale scan",
            "muted",
        ));
    }

    if local.scan.status == "failed" || local.scan.status == "cancelled" {
        attention.push(RankedAttention::new(
            2,
            "Latest scan is partial",
            "Local scan",
            "The previous scan did not complete, so the Overview may not include every installed file.",
            "Partial",
            "warning",
        ));
    }

    attention
}

#[derive(Debug, Clone)]
struct RankedAttention {
    priority: u8,
    item: OverviewAttentionItem,
}

impl RankedAttention {
    fn new(
        priority: u8,
        name: impl Into<String>,
        creator: impl Into<String>,
        detail: impl Into<String>,
        badge: impl Into<String>,
        tone: impl Into<String>,
    ) -> Self {
        Self {
            priority,
            item: OverviewAttentionItem {
                name: name.into(),
                creator: creator.into(),
                detail: detail.into(),
                badge: badge.into(),
                tone: tone.into(),
            },
        }
    }
}

fn finalize_snapshot(
    local: &LocalOverviewContext,
    installation: &LocalInstallation,
    health_score: Option<u8>,
    health_counts: OverviewHealthCounts,
    installation_counts: OverviewInstallationCounts,
    registry_state: &str,
    registry_detail: &str,
    mut ranked_attention: Vec<RankedAttention>,
) -> OverviewSnapshot {
    ranked_attention.sort_by(|left, right| {
        (
            left.priority,
            left.item.name.as_str(),
            left.item.badge.as_str(),
        )
            .cmp(&(
                right.priority,
                right.item.name.as_str(),
                right.item.badge.as_str(),
            ))
    });

    let attention_count = ranked_attention.len() as u64;
    let attention = ranked_attention
        .into_iter()
        .take(ATTENTION_LIMIT)
        .map(|entry| entry.item)
        .collect();

    OverviewSnapshot {
        has_installation: true,
        game_version: installation.game_version.clone(),
        platform: installation.platform.clone(),
        indexed_count: local.indexed_count,
        health_score,
        health_score_explanation:
            "Overall health is the percentage of current canonical releases with verified compatible patch evidence. Update-available releases still count as compatible when their installed release is compatible; unresolved files remain in the denominator."
                .to_string(),
        health_counts,
        attention_count,
        attention,
        installation: installation_counts,
        registry_state: registry_state.to_string(),
        registry_detail: registry_detail.to_string(),
        scan: OverviewScanState {
            partial: local.scan.partial || local.local_analysis_partial,
            ..local.scan.clone()
        },
    }
}

fn empty_snapshot(scan: OverviewScanState) -> OverviewSnapshot {
    OverviewSnapshot {
        has_installation: false,
        game_version: None,
        platform: "windows".to_string(),
        indexed_count: 0,
        health_score: None,
        health_score_explanation:
            "Health becomes available after an installation has been scanned and resolved against the registry."
                .to_string(),
        health_counts: OverviewHealthCounts {
            healthy: 0,
            updates: 0,
            conflicts: 0,
            unknown: 0,
        },
        attention_count: 0,
        attention: Vec::new(),
        installation: OverviewInstallationCounts {
            script_mods: 0,
            package_files: 0,
            unidentified: None,
            exact_duplicates: 0,
        },
        registry_state: "offline".to_string(),
        registry_detail: "No scanned Sims installation is stored locally yet.".to_string(),
        scan,
    }
}

fn release_name<'a>(release_names: &'a HashMap<String, String>, release_id: &str) -> &'a str {
    release_names
        .get(release_id)
        .map(String::as_str)
        .unwrap_or("Resolved installed mod")
}

fn display_name(relative_path: &str) -> String {
    let filename = Path::new(relative_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(relative_path);

    Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename)
        .replace(['_', '-'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;
    use tempfile::TempDir;

    fn database() -> (TempDir, PathBuf) {
        let temp = TempDir::new().expect("overview temp directory");
        let path = temp.path().join("overview.sqlite3");
        storage::initialize(&path).expect("initialize overview database");
        (temp, path)
    }

    #[test]
    fn local_context_uses_real_scan_counts_and_duplicates() {
        let (_temp, path) = database();
        let connection = storage::open(&path).expect("open overview database");

        connection.execute(
            "INSERT INTO installations (
                game_root, mods_root, platform, game_version, discovered_at, last_seen_at
             ) VALUES ('game', 'mods', 'windows', '1.128.90', '2026-09-26T10:00:00Z', '2026-09-26T10:00:00Z')",
            [],
        ).expect("insert installation");
        connection
            .execute(
                "INSERT INTO scan_sessions (
                installation_id, started_at, completed_at, status, mode,
                files_seen, files_hashed, files_skipped, observation_count
             ) VALUES (
                1, '2026-09-26T10:00:00Z', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                'completed', 'incremental', 3, 2, 1, 0
             )",
                [],
            )
            .expect("insert scan");

        for (path, kind) in [
            ("Mods/A.package", "package"),
            ("Mods/B.package", "package"),
            ("Mods/C.ts4script", "ts4script"),
        ] {
            connection
                .execute(
                    "INSERT INTO local_files (
                    installation_id, relative_path, file_kind, enabled
                 ) VALUES (1, ?1, ?2, 1)",
                    params![path, kind],
                )
                .expect("insert local file");
        }
        connection
            .execute(
                "INSERT INTO fingerprints (
                local_file_id, kind, value, algorithm_version, computed_at
             ) VALUES
                (1, 'sha256', 'same', 'sha256-v1', 'now'),
                (2, 'sha256', 'same', 'sha256-v1', 'now'),
                (3, 'sha256', 'third', 'sha256-v1', 'now')",
                [],
            )
            .expect("insert fingerprints");
        drop(connection);

        let context = load_local_context(&path).expect("load overview");

        assert_eq!(context.indexed_count, 3);
        assert_eq!(context.package_files, 2);
        assert_eq!(context.script_mods, 1);
        assert_eq!(context.exact_duplicates.len(), 1);
        assert_eq!(context.scan.files_skipped, 1);
        assert!(!context.scan.stale);
    }

    #[test]
    fn health_score_uses_compatibility_not_update_maintenance_state() {
        let health = vec![
            HealthAssessment {
                release_id: "release-a".to_string(),
                mod_id: "mod-a".to_string(),
                state: "update_available".to_string(),
                compatibility_state: "compatible".to_string(),
                disputed: false,
                reason: "newer_release_available".to_string(),
                update: crate::registry::UpdateAssessment {
                    available: true,
                    target_version: Some("2.0".to_string()),
                    target_compatibility_state: Some("compatible".to_string()),
                    target_disputed: false,
                },
            },
            HealthAssessment {
                release_id: "release-b".to_string(),
                mod_id: "mod-b".to_string(),
                state: "unknown".to_string(),
                compatibility_state: "unknown".to_string(),
                disputed: false,
                reason: "no_current_patch_evidence".to_string(),
                update: crate::registry::UpdateAssessment {
                    available: false,
                    target_version: None,
                    target_compatibility_state: None,
                    target_disputed: false,
                },
            },
        ];
        let names = HashMap::from([
            ("release-a".to_string(), "A".to_string()),
            ("release-b".to_string(), "B".to_string()),
        ]);
        let mut counts = OverviewHealthCounts {
            healthy: 0,
            updates: 0,
            conflicts: 0,
            unknown: 0,
        };
        let mut attention = Vec::new();

        let score = apply_health(&health, &names, 0, &mut counts, &mut attention);

        assert_eq!(score, Some(50));
        assert_eq!(counts.updates, 1);
        assert_eq!(counts.unknown, 1);
        assert_eq!(counts.healthy, 0);
    }
}
