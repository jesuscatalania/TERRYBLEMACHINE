pub mod keychain;
pub mod projects;

use std::sync::Arc;

use tauri::Manager;

use keychain::commands::KeyStoreState;
use projects::commands::{resolve_default_root, ProjectStoreState};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store: Arc<dyn keychain::KeyStore> = Arc::from(keychain::default_store());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = resolve_default_root(app.handle());
            app.manage(ProjectStoreState::new(root));
            Ok(())
        })
        .manage(KeyStoreState::new(store))
        .invoke_handler(tauri::generate_handler![
            greet,
            keychain::commands::store_api_key,
            keychain::commands::get_api_key,
            keychain::commands::delete_api_key,
            keychain::commands::list_api_keys,
            projects::commands::create_project,
            projects::commands::open_project,
            projects::commands::list_projects,
            projects::commands::delete_project,
            projects::commands::projects_root,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
