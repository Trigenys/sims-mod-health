mod conflicts;
mod dbpf;
mod diagnostics;
mod discovery;
mod fingerprint;
mod game;
mod mutation;
mod overview;
mod privacy;
mod registry;
mod scanner;
mod storage;
mod ts4script;

use std::{path::PathBuf, sync::Arc};

use conflicts::LocalConflictAnalysis;
use diagnostics::DiagnosticsSnapshot;
use discovery::DiscoverySnapshot;
use fingerprint::ExactDuplicateGroup;
use game::{InstallationCandidate, ManualInspection};
use mutation::{ApplyUpdateRequest, UpdateTransactionView};
use overview::OverviewSnapshot;
use privacy::PrivacyPreferences;
use rusqlite::OptionalExtension;
use scanner::{ScanMode, ScanSummary, ScannerControl};
use tauri::{Emitter, Manager, State};

struct AppState {
    app_data_dir: PathBuf,
    database_path: PathBuf,
    scanner: Arc<ScannerControl>,
}

#[tauri::command]
fn health() -> &'static str {
    "ok"
}

#[tauri::command]
fn discover_sims_installations() -> Vec<InstallationCandidate> {
    game::discover_installations()
}

#[tauri::command]
fn inspect_sims_installation(path: String) -> ManualInspection {
    game::inspect_manual_path(&PathBuf::from(path))
}

#[tauri::command]
async fn scan_sims_mods(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    mode: ScanMode,
) -> Result<ScanSummary, String> {
    let database_path = state.database_path.clone();
    let scanner = Arc::clone(&state.scanner);
    let selected_path = PathBuf::from(path);

    tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_path(
            &database_path,
            &selected_path,
            mode,
            scanner.as_ref(),
            |progress| {
                let _ = app.emit("scanner://progress", progress);
            },
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("scanner worker failed: {error}"))?
}

#[tauri::command]
fn cancel_mod_scan(state: State<'_, AppState>) -> bool {
    state.scanner.cancel()
}

#[tauri::command]
fn get_privacy_preferences(state: State<'_, AppState>) -> Result<PrivacyPreferences, String> {
    privacy::load_preferences(&state.database_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_diagnostic_telemetry_consent(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<PrivacyPreferences, String> {
    privacy::set_diagnostic_telemetry(&state.database_path, enabled)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_discovery_recommendations(
    state: State<'_, AppState>,
) -> Result<DiscoverySnapshot, String> {
    Ok(discovery::load(&state.database_path).await)
}

#[tauri::command]
async fn analyze_latest_diagnostics(
    state: State<'_, AppState>,
) -> Result<DiagnosticsSnapshot, String> {
    let database_path = state.database_path.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        diagnostics::prepare_latest(&database_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("diagnostic preparation worker failed: {error}"))??;

    let Some(prepared) = prepared else {
        return Ok(diagnostics::empty_snapshot());
    };

    let mut snapshot = diagnostics::resolve_registry(prepared).await;
    let database_path = state.database_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        diagnostics::persist_snapshot(&database_path, &mut snapshot)
            .map_err(|error| error.to_string())?;
        Ok::<DiagnosticsSnapshot, String>(snapshot)
    })
    .await
    .map_err(|error| format!("diagnostic persistence worker failed: {error}"))?
}

#[tauri::command]
async fn apply_mod_update(
    state: State<'_, AppState>,
    request: ApplyUpdateRequest,
) -> Result<UpdateTransactionView, String> {
    mutation::apply_update(&state.database_path, &state.app_data_dir, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn rollback_mod_update(
    state: State<'_, AppState>,
    transaction_id: i64,
) -> Result<UpdateTransactionView, String> {
    mutation::rollback_update(&state.database_path, &state.app_data_dir, transaction_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_mod_update_transaction(
    state: State<'_, AppState>,
    transaction_id: i64,
) -> Result<UpdateTransactionView, String> {
    mutation::load_transaction(&state.database_path, transaction_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn scan_current_sims_mods(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mode: ScanMode,
) -> Result<ScanSummary, String> {
    let lookup_database_path = state.database_path.clone();
    let selected_path = tauri::async_runtime::spawn_blocking(move || {
        overview::latest_installation_root(&lookup_database_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("installation lookup worker failed: {error}"))??
    .ok_or_else(|| "no scanned Sims installation is available".to_string())?;

    let database_path = state.database_path.clone();
    let scanner = Arc::clone(&state.scanner);

    tauri::async_runtime::spawn_blocking(move || {
        scanner::scan_path(
            &database_path,
            &selected_path,
            mode,
            scanner.as_ref(),
            |progress| {
                let _ = app.emit("scanner://progress", progress);
            },
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("scanner worker failed: {error}"))?
}

#[tauri::command]
async fn get_overview_snapshot(state: State<'_, AppState>) -> Result<OverviewSnapshot, String> {
    let database_path = state.database_path.clone();
    let local = tauri::async_runtime::spawn_blocking(move || {
        overview::load_local_context(&database_path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("overview worker failed: {error}"))??;

    Ok(overview::build_snapshot(local).await)
}

fn installed_id(
    connection: &rusqlite::Connection,
    installation: &InstallationCandidate,
) -> Result<Option<i64>, String> {
    connection
        .query_row(
            "SELECT id
             FROM installations
             WHERE game_root = ?1 AND mods_root = ?2",
            (
                installation.root.to_string_lossy().to_string(),
                installation.mods_root.to_string_lossy().to_string(),
            ),
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_exact_duplicates(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<ExactDuplicateGroup>, String> {
    let installation = match game::inspect_manual_path(&PathBuf::from(path)) {
        ManualInspection::Available { installation } => installation,
        ManualInspection::Unavailable { reason } => return Err(reason),
    };

    let connection = storage::open(&state.database_path).map_err(|error| error.to_string())?;
    let Some(installation_id) = installed_id(&connection, &installation)? else {
        return Ok(Vec::new());
    };

    fingerprint::exact_duplicate_groups(&connection, installation_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn analyze_local_conflicts(
    state: State<'_, AppState>,
    path: String,
) -> Result<LocalConflictAnalysis, String> {
    let installation = match game::inspect_manual_path(&PathBuf::from(path)) {
        ManualInspection::Available { installation } => installation,
        ManualInspection::Unavailable { reason } => return Err(reason),
    };

    let connection = storage::open(&state.database_path).map_err(|error| error.to_string())?;
    let Some(installation_id) = installed_id(&connection, &installation)? else {
        return Ok(LocalConflictAnalysis {
            exact_duplicates: Vec::new(),
            resource_overlaps: Vec::new(),
            parse_failures: Vec::new(),
            overlap_pairs_truncated: false,
        });
    };

    conflicts::analyze_installation(&connection, installation_id, &installation.mods_root)
        .map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let database_path = app_data_dir.join("sims-mod-health.sqlite3");
            storage::initialize(&database_path)?;
            mutation::recover_interrupted_transactions(&database_path)?;

            app.manage(AppState {
                app_data_dir,
                database_path,
                scanner: Arc::new(ScannerControl::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health,
            discover_sims_installations,
            inspect_sims_installation,
            scan_sims_mods,
            scan_current_sims_mods,
            cancel_mod_scan,
            get_privacy_preferences,
            set_diagnostic_telemetry_consent,
            get_discovery_recommendations,
            analyze_latest_diagnostics,
            apply_mod_update,
            rollback_mod_update,
            get_mod_update_transaction,
            get_overview_snapshot,
            list_exact_duplicates,
            analyze_local_conflicts
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
