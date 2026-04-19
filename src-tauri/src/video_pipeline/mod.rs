//! Video generation pipeline — the Rust-side orchestrator for the Video
//! module's "Generate clip" flow.
//!
//! Mirrors [`crate::mesh_pipeline`]: all public methods dispatch through
//! the shared [`AiRouter`](crate::ai_router::AiRouter) so they inherit
//! cache, budget, retry, and fallback behaviour. The distinguishing piece
//! vs. other pipelines is the MP4-download-to-cache step inside
//! [`RouterVideoPipeline`]: Kling (+ Runway / Higgsfield via Phase 2
//! polling) returns a remote URL, and the Tauri webview benefits from a
//! local `asset://` path to load it without CORS/HTTP hiccups on first
//! paint.
//!
//! Exposes:
//! - `generate_from_text(prompt, duration_s?)` — routes
//!   [`TaskKind::TextToVideo`] to
//!   [`Model::FalKlingV2Master`](crate::ai_router::Model::FalKlingV2Master)
//!   with `FalKlingV15` + Runway + Higgsfield as fallbacks
//!   (Variante A: Kling via fal.ai aggregator).
//! - `generate_from_image(image_url, prompt?, duration_s?)` — routes
//!   [`TaskKind::ImageToVideo`] along the same chain.
//!
//! [`TaskKind::TextToVideo`]: crate::ai_router::TaskKind::TextToVideo
//! [`TaskKind::ImageToVideo`]: crate::ai_router::TaskKind::ImageToVideo

pub mod commands;
mod pipeline;
mod stub;
mod types;

pub use pipeline::RouterVideoPipeline;
pub use stub::StubVideoPipeline;
pub use types::{VideoImageInput, VideoPipeline, VideoPipelineError, VideoResult, VideoTextInput};
