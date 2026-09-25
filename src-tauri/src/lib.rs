mod storage;

use tauri::Manager;

#[tauri::command]
fn health() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let database_path = app.path().app_data_dir()?.join("sims-mod-health.sqlite3");

            storage::initialize(&database_path)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![health])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
