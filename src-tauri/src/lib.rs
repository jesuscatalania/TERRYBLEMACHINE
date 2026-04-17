pub mod ai_router;
pub mod api_clients;
pub mod code_generator;
pub mod exporter;
pub mod image_pipeline;
pub mod keychain;
pub mod projects;
pub mod taste_engine;
pub mod website_analyzer;

use std::sync::Arc;

use tauri::Manager;

use ai_router::commands::AiRouterState;
use ai_router::{AiRouter, DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
use code_generator::commands::CodeGeneratorState;
use code_generator::{CodeGenerator, StubCodeGenerator};
use image_pipeline::commands::ImagePipelineState;
use image_pipeline::{ImagePipeline, RouterImagePipeline};
use keychain::commands::KeyStoreState;
use projects::commands::{resolve_default_root, ProjectStoreState};
use taste_engine::commands::TasteEngineState;
use taste_engine::{StubVisionAnalyzer, TasteEngine};
use website_analyzer::commands::WebsiteAnalyzerState;
use website_analyzer::{PlaywrightUrlAnalyzer, UrlAnalyzer};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let keystore: Arc<dyn keychain::KeyStore> = Arc::from(keychain::default_store());

    // All 9 provider clients are registered. Each resolves its API key from
    // the keychain at execute time; missing keys surface as Auth errors and
    // the router's fallback chain picks the next provider.
    let clients = api_clients::registry::build_default_clients(keystore.clone());
    let ai_router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));

    let ai_router_for_setup = Arc::clone(&ai_router);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let root = resolve_default_root(app.handle());
            app.manage(ProjectStoreState::new(root));

            // Taste engine scoped to the workspace-local meingeschmack/.
            // Uses a stub vision analyzer for now — Claude Vision wiring
            // follows when modules actually ship image analysis.
            let meingeschmack_root = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("meingeschmack");
            let engine = Arc::new(TasteEngine::new(
                meingeschmack_root,
                Arc::new(StubVisionAnalyzer::new()),
            ));
            app.manage(TasteEngineState::new(Arc::clone(&engine)));

            // Website analyzer sidecar — expects scripts/url_analyzer.mjs
            // in the app's working directory + `node` on PATH.
            let script_path = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("scripts")
                .join("url_analyzer.mjs");
            let analyzer: Arc<dyn UrlAnalyzer> = Arc::new(PlaywrightUrlAnalyzer::new(script_path));
            app.manage(WebsiteAnalyzerState::new(analyzer));

            // Code generator — default to the deterministic stub until the
            // frontend wires a real Claude key through. Swapping in
            // ClaudeCodeGenerator requires an AiClient + keychain lookup.
            let generator: Arc<dyn CodeGenerator> = Arc::new(StubCodeGenerator::new());
            app.manage(CodeGeneratorState::new(generator));

            // Image pipeline — routed through the production AiRouter with
            // taste-engine enrichment. Missing provider keys bubble up as
            // routing errors rather than stub URLs.
            let pipeline: Arc<dyn ImagePipeline> = Arc::new(
                RouterImagePipeline::new(Arc::clone(&ai_router_for_setup))
                    .with_taste_engine(Arc::clone(&engine)),
            );
            app.manage(ImagePipelineState::new(pipeline));
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
            ai_router::commands::get_cache_stats,
            ai_router::commands::get_budget_status,
            ai_router::commands::set_budget_limit,
            ai_router::commands::export_usage,
            taste_engine::commands::refresh_taste,
            taste_engine::commands::get_taste_profile,
            taste_engine::commands::enrich_taste_prompt,
            taste_engine::commands::get_negative_prompt,
            website_analyzer::commands::analyze_url,
            code_generator::commands::generate_website,
            exporter::commands::export_website,
            image_pipeline::commands::text_to_image,
            image_pipeline::commands::image_to_image,
            image_pipeline::commands::upscale_image,
            image_pipeline::commands::generate_variants,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
