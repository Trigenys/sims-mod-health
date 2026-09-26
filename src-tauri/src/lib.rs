mod conflicts;
mod dbpf;
mod fingerprint;
mod game;
mod overview;
mod registry;
mod scanner;
mod storage;
mod ts4script;

use std::{path::PathBuf, sync::Arc};

use conflicts::LocalConflictAnalysis;
use fingerprint::ExactDuplicateGroup;
use game::{InstallationCandidate, ManualInspection};
use overview::OverviewSnapshot;
use rusqlite::OptionalExtension;
use scanner::{ScanMode, ScanSummary, ScannerControl};
use tauri::{Emitter, Manager, State};

struct AppState {
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
            let database_path = app.path().app_data_dir()?.join("sims-mod-health.sqlite3");
            storage::initialize(&database_path)?;

            app.manage(AppState {
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
            get_overview_snapshot,
            list_exact_duplicates,
            analyze_local_conflicts
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
