//! Mesh (3D) generation pipeline — the Rust-side orchestrator for the
//! Graphic3D "Generate 3D" flow.
//!
//! Mirrors [`crate::depth_pipeline`]: all public methods dispatch through
//! the shared [`AiRouter`](crate::ai_router::AiRouter) so they inherit
//! cache, budget, retry, and fallback behaviour. The distinguishing piece
//! vs. other pipelines is the GLB-download-to-cache step inside
//! [`RouterMeshPipeline`]: Meshy returns a remote URL (resolved via T8/T9
//! polling), and Three.js needs a local `asset://` path to load it without
//! CORS/HTTP hiccups.
//!
//! Exposes:
//! - `generate_from_text(prompt)` — routes [`TaskKind::Text3D`] to
//!   [`Model::MeshyText3D`](crate::ai_router::Model::MeshyText3D).
//! - `generate_from_image(image_url, prompt?)` — routes
//!   [`TaskKind::Image3D`] to
//!   [`Model::MeshyImage3D`](crate::ai_router::Model::MeshyImage3D).
//!
//! [`TaskKind::Text3D`]: crate::ai_router::TaskKind::Text3D
//! [`TaskKind::Image3D`]: crate::ai_router::TaskKind::Image3D

pub mod commands;
mod pipeline;
mod stub;
mod types;

pub use pipeline::RouterMeshPipeline;
pub use stub::StubMeshPipeline;
pub use types::{MeshImageInput, MeshPipeline, MeshPipelineError, MeshResult, MeshTextInput};
