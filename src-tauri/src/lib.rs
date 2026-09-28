mod conflicts;
mod dbpf;
mod delta;
mod diagnostics;
mod discovery;
mod fingerprint;
mod game;
mod library;
mod mutation;
mod overview;
mod privacy;
mod registry;
mod scanner;
mod storage;
mod ts4script;

use std::{path::PathBuf, sync::Arc};

use serde::Serialize;

use conflicts::LocalConflictAnalysis;
use diagnostics::DiagnosticsSnapshot;
use discovery::DiscoverySnapshot;
use fingerprint::ExactDuplicateGroup;
use game::{
    GameContentHealthSnapshot, GameContentInstallation, GameContentSnapshot, InstallationCandidate,
    ManualInspection, ProviderUpdateCapability, ProviderUpdateSessionView,
    ProviderUpdateTargetKind,
};
use library::LibrarySnapshot;
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
async fn refresh_game_content_inventory(
    state: State<'_, AppState>,
) -> Result<GameContentSnapshot, String> {
    let database_path = state.database_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        game::refresh_and_persist_game_content(&database_path)
    })
    .await
    .map_err(|error| format!("game content inventory worker failed: {error}"))?
}

#[tauri::command]
fn inspect_game_content_installation(path: String) -> Result<GameContentInstallation, String> {
    game::inspect_game_content_path(&PathBuf::from(path))
}

#[tauri::command]
async fn get_game_content_health(
    state: State<'_, AppState>,
) -> Result<GameContentHealthSnapshot, String> {
    game::evaluate_game_content_health(&state.database_path).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderUpdateVerificationView {
    session: ProviderUpdateSessionView,
    game_content_health: GameContentHealthSnapshot,
    mod_scan: Option<ScanSummary>,
    mod_health: OverviewSnapshot,
}

#[tauri::command]
fn get_provider_update_capability(
    state: State<'_, AppState>,
) -> Result<ProviderUpdateCapability, String> {
    game::get_provider_update_capability(&state.database_path)
}

#[tauri::command]
fn start_game_content_provider_update(
    state: State<'_, AppState>,
    target_kind: ProviderUpdateTargetKind,
    target_id: String,
) -> Result<ProviderUpdateSessionView, String> {
    game::start_provider_update(&state.database_path, target_kind, &target_id)
}

#[tauri::command]
fn get_game_content_provider_update_session(
    state: State<'_, AppState>,
    session_id: i64,
) -> Result<ProviderUpdateSessionView, String> {
    game::get_provider_update_session(&state.database_path, session_id)
}

#[tauri::command]
fn get_pending_game_content_provider_update_session(
    state: State<'_, AppState>,
) -> Result<Option<ProviderUpdateSessionView>, String> {
    game::get_pending_provider_update_session(&state.database_path)
}

#[tauri::command]
async fn verify_game_content_provider_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: i64,
) -> Result<ProviderUpdateVerificationView, String> {
    game::begin_provider_update_verification(&state.database_path, session_id)?;

    let verification = async {
        let inventory_database_path = state.database_path.clone();
        let inventory = tauri::async_runtime::spawn_blocking(move || {
            game::refresh_and_persist_game_content(&inventory_database_path)
        })
        .await
        .map_err(|error| format!("game content inventory worker failed: {error}"))??;

        if inventory.installations.is_empty() {
            return Err(
                "The Sims 4 program installation could not be rediscovered after provider handoff."
                    .to_string(),
            );
        }

        let lookup_database_path = state.database_path.clone();
        let (program_version, mod_user_root) = tauri::async_runtime::spawn_blocking(move || {
            let connection =
                storage::open(&lookup_database_path).map_err(|error| error.to_string())?;
            let program_version =
                game::current_program_version_for_session(&connection, session_id)?;
            let mod_user_root = game::latest_mod_user_root(&connection)?;
            Ok::<_, String>((program_version, mod_user_root))
        })
        .await
        .map_err(|error| format!("post-update lookup worker failed: {error}"))??;

        let mod_scan = if let Some(mod_user_root) = mod_user_root {
            let scan_database_path = state.database_path.clone();
            let scanner = Arc::clone(&state.scanner);
            let app_for_scan = app.clone();

            let summary = tauri::async_runtime::spawn_blocking(move || {
                scanner::scan_path(
                    &scan_database_path,
                    &mod_user_root,
                    ScanMode::Incremental,
                    scanner.as_ref(),
                    |progress| {
                        let _ = app_for_scan.emit("scanner://progress", progress);
                    },
                )
                .map_err(|error| error.to_string())
            })
            .await
            .map_err(|error| format!("post-update Mods scan worker failed: {error}"))??;

            Some(summary)
        } else {
            None
        };

        if let Some(program_version) = program_version.as_deref() {
            let sync_database_path = state.database_path.clone();
            let version = program_version.to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let connection =
                    storage::open(&sync_database_path).map_err(|error| error.to_string())?;
                game::sync_latest_mod_game_version(&connection, &version)
            })
            .await
            .map_err(|error| format!("game-version synchronization worker failed: {error}"))??;
        }

        let game_content_health = game::evaluate_game_content_health(&state.database_path).await?;

        let overview_database_path = state.database_path.clone();
        let local_overview = tauri::async_runtime::spawn_blocking(move || {
            overview::load_local_context(&overview_database_path).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| format!("post-update overview worker failed: {error}"))??;
        let mod_health = overview::build_snapshot(local_overview).await;

        Ok::<_, String>((game_content_health, mod_scan, mod_health))
    }
    .await;

    match verification {
        Ok((game_content_health, mod_scan, mod_health)) => {
            let session = game::complete_provider_update_verification(
                &state.database_path,
                session_id,
                &game_content_health,
            )?;

            Ok(ProviderUpdateVerificationView {
                session,
                game_content_health,
                mod_scan,
                mod_health,
            })
        }
        Err(error) => {
            let _ =
                game::fail_provider_update_verification(&state.database_path, session_id, &error);
            Err(error)
        }
    }
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

#[tauri::command]
async fn get_library_snapshot(state: State<'_, AppState>) -> Result<LibrarySnapshot, String> {
    let database_path = state.database_path.clone();

    tauri::async_runtime::spawn_blocking(move || library::load(&database_path))
        .await
        .map_err(|error| format!("library worker failed: {error}"))?
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
        .plugin(tauri_plugin_dialog::init())
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
            refresh_game_content_inventory,
            inspect_game_content_installation,
            get_game_content_health,
            get_provider_update_capability,
            start_game_content_provider_update,
            get_game_content_provider_update_session,
            get_pending_game_content_provider_update_session,
            verify_game_content_provider_update,
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
            get_library_snapshot,
            list_exact_duplicates,
            analyze_local_conflicts
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
