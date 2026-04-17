pub mod ai_router;
pub mod api_clients;
pub mod keychain;
pub mod projects;

use std::collections::HashMap;
use std::sync::Arc;

use tauri::Manager;

use ai_router::commands::AiRouterState;
use ai_router::{AiRouter, DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
use keychain::commands::KeyStoreState;
use projects::commands::{resolve_default_root, ProjectStoreState};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let keystore: Arc<dyn keychain::KeyStore> = Arc::from(keychain::default_store());

    // No provider clients yet — they ship in Schritt 2.2.
    // The router is already functional; any `route_request` call will return
    // `NoClient` until 2.2 wires real implementations.
    let ai_router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        HashMap::new(),
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = resolve_default_root(app.handle());
            app.manage(ProjectStoreState::new(root));
            Ok(())
        })
        .manage(KeyStoreState::new(keystore))
        .manage(AiRouterState::new(ai_router))
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
            ai_router::commands::route_request,
            ai_router::commands::get_queue_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
