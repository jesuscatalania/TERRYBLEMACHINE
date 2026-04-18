pub mod ai_router;
pub mod api_clients;
pub mod code_generator;
pub mod depth_pipeline;
pub mod exporter;
pub mod image_pipeline;
pub mod keychain;
pub mod mesh_pipeline;
pub mod projects;
pub mod remotion;
pub mod shotstack_assembly;
pub mod storyboard_generator;
pub mod taste_engine;
pub mod video_pipeline;
pub mod website_analyzer;

use std::sync::Arc;

use tauri::Manager;

use ai_router::commands::AiRouterState;
use ai_router::{AiRouter, DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
use code_generator::commands::CodeGeneratorState;
use code_generator::{CodeGenerator, StubCodeGenerator};
use depth_pipeline::commands::DepthPipelineState;
use depth_pipeline::{DepthPipeline, RouterDepthPipeline};
use image_pipeline::commands::ImagePipelineState;
use image_pipeline::{ImagePipeline, RouterImagePipeline};
use keychain::commands::KeyStoreState;
use mesh_pipeline::commands::MeshPipelineState;
use mesh_pipeline::{MeshPipeline, RouterMeshPipeline};
use projects::commands::{resolve_default_root, ProjectStoreState};
use taste_engine::commands::TasteEngineState;
use taste_engine::{ClaudeVisionAnalyzer, TasteEngine};
use video_pipeline::commands::VideoPipelineState;
use video_pipeline::{RouterVideoPipeline, VideoPipeline};
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
    // Clone held for the `setup` closure: the `ShotstackAssembler` owns its
    // own `ShotstackClient` (see below) rather than going through the
    // AiRouter, so it needs its own handle on the keystore.
    let keystore_for_setup = Arc::clone(&keystore);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let root = resolve_default_root(app.handle());
            app.manage(ProjectStoreState::new(root));

            // Taste engine scoped to the workspace-local meingeschmack/.
            // Reference images are routed through the production
            // ClaudeVisionAnalyzer so the live profile reflects the user's
            // actual moodboard, not deterministic stubs.
            let meingeschmack_root = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("meingeschmack");
            let engine = Arc::new(TasteEngine::new(
                meingeschmack_root.clone(),
                Arc::new(ClaudeVisionAnalyzer::new(Arc::clone(&ai_router_for_setup))),
            ));
            app.manage(TasteEngineState::new(Arc::clone(&engine)));

            // Watcher loop: on any meingeschmack/ change, re-parse rules and
            // re-run image analyses so generative prompts always see a fresh
            // StyleProfile. The task terminates cleanly once the engine Arc
            // is dropped and the channel senders are released.
            let watch_engine = Arc::clone(&engine);
            let watch_root = meingeschmack_root.clone();
            tauri::async_runtime::spawn(async move {
                match taste_engine::watcher::TasteWatcher::new(watch_root) {
                    Ok(mut w) => loop {
                        if w.next_event().await.is_none() {
                            break;
                        }
                        if let Err(e) = watch_engine.refresh().await {
                            eprintln!("[taste-engine] refresh failed: {e}");
                        }
                    },
                    Err(e) => eprintln!("[taste-engine] watcher init failed: {e}"),
                }
            });

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

            // Storyboard generator — routes through the shared AiRouter
            // (Claude) and enriches prompts with the live taste profile.
            // Returns strict JSON shot breakdowns for the video module.
            let storyboard: Arc<dyn storyboard_generator::StoryboardGenerator> = Arc::new(
                storyboard_generator::ClaudeStoryboardGenerator::new(Arc::clone(
                    &ai_router_for_setup,
                ))
                .with_taste_engine(Arc::clone(&engine)),
            );
            app.manage(storyboard_generator::commands::StoryboardGeneratorState::new(storyboard));

            // Image pipeline — routed through the production AiRouter with
            // taste-engine enrichment. Missing provider keys bubble up as
            // routing errors rather than stub URLs.
            let pipeline: Arc<dyn ImagePipeline> = Arc::new(
                RouterImagePipeline::new(Arc::clone(&ai_router_for_setup))
                    .with_taste_engine(Arc::clone(&engine)),
            );
            app.manage(ImagePipelineState::new(pipeline));

            // Depth-map pipeline — routed through the same AiRouter. Task
            // DepthMap → Model ReplicateDepthAnythingV2 in the default
            // strategy. No taste-engine enrichment: depth inference is
            // deterministic wrt. the input image.
            let depth: Arc<dyn DepthPipeline> =
                Arc::new(RouterDepthPipeline::new(Arc::clone(&ai_router_for_setup)));
            app.manage(DepthPipelineState::new(depth));

            // Mesh pipeline — routes Text3D/Image3D through the AiRouter to
            // Meshy, then downloads the resulting GLB into the platform
            // cache dir. Frontend loads via Tauri `convertFileSrc`; when
            // the download fails, it falls back to the remote URL.
            let mesh: Arc<dyn MeshPipeline> =
                Arc::new(RouterMeshPipeline::new(Arc::clone(&ai_router_for_setup)));
            app.manage(MeshPipelineState::new(mesh));

            // Video pipeline — routes TextToVideo/ImageToVideo through the
            // AiRouter to Kling (Runway + Higgsfield as fallbacks, polling
            // wired in T4), then downloads the resulting MP4 into the
            // platform cache dir. Frontend loads via Tauri `convertFileSrc`;
            // when the download fails, it falls back to the remote URL.
            let video: Arc<dyn VideoPipeline> =
                Arc::new(RouterVideoPipeline::new(Arc::clone(&ai_router_for_setup)));
            app.manage(VideoPipelineState::new(video));

            // Remotion render pipeline — spawns `npx remotion render` in the
            // workspace-local remotion/ subpackage. Output lands in
            // <cache-dir>/terryblemachine/remotion-renders/<composition>-<hash>.mp4
            // so repeat renders of the same (composition, props) are free.
            let remotion_root = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("remotion");
            app.manage(remotion::RemotionState::new(remotion_root));

            // Shotstack timeline assembler — shares the keystore via its own
            // `ShotstackClient` (not via the AiRouter, which would gain us
            // nothing: Shotstack's timeline JSON has no cross-provider
            // fallback). Downloads the finished MP4 to the platform cache so
            // Remotion preview doesn't hit the CDN on every play.
            let shotstack_client = Arc::new(api_clients::shotstack::ShotstackClient::new(
                keystore_for_setup.clone(),
            ));
            let assembler: Arc<dyn shotstack_assembly::VideoAssembler> = Arc::new(
                shotstack_assembly::ShotstackAssembler::new(shotstack_client),
            );
            app.manage(shotstack_assembly::commands::VideoAssemblerState::new(
                assembler,
            ));
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
            projects::history_commands::read_project_history,
            projects::history_commands::write_project_history,
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
            code_generator::assist::modify_code_selection,
            storyboard_generator::commands::generate_storyboard,
            exporter::commands::export_website,
            image_pipeline::commands::text_to_image,
            image_pipeline::commands::image_to_image,
            image_pipeline::commands::upscale_image,
            image_pipeline::commands::generate_variants,
            image_pipeline::commands::inpaint_image,
            depth_pipeline::commands::generate_depth,
            mesh_pipeline::commands::generate_mesh_from_text,
            mesh_pipeline::commands::generate_mesh_from_image,
            mesh_pipeline::commands::export_mesh,
            video_pipeline::commands::generate_video_from_text,
            video_pipeline::commands::generate_video_from_image,
            remotion::commands::render_remotion,
            shotstack_assembly::commands::assemble_video,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
