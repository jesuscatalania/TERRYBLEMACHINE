pub mod keychain;

use std::sync::Arc;

use keychain::commands::KeyStoreState;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store: Arc<dyn keychain::KeyStore> = Arc::from(keychain::default_store());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(KeyStoreState::new(store))
        .invoke_handler(tauri::generate_handler![
            greet,
            keychain::commands::store_api_key,
            keychain::commands::get_api_key,
            keychain::commands::delete_api_key,
            keychain::commands::list_api_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
