mod game;
mod storage;

use std::path::PathBuf;

use game::{InstallationCandidate, ManualInspection};
use tauri::Manager;

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

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database_path = app.path().app_data_dir()?.join("sims-mod-health.sqlite3");

            storage::initialize(&database_path)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health,
            discover_sims_installations,
            inspect_sims_installation
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
