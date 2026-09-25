mod dbpf;
mod game;
mod scanner;
mod storage;

use std::{path::PathBuf, sync::Arc};

use game::{InstallationCandidate, ManualInspection};
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
            cancel_mod_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
