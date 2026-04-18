//! Depth-map generation pipeline — the Rust-side orchestrator for the
//! pseudo-3D module.
//!
//! Mirrors the structure of [`crate::image_pipeline`]: all public methods go
//! through the shared [`AiRouter`](crate::ai_router::AiRouter) so they
//! inherit the cache, budget, retry, and fallback machinery built in
//! Phase 2.
//!
//! For Phase 5 / Task 6 the module exposes a single method:
//! - `generate(image_url)` — routes [`TaskKind::DepthMap`] through the
//!   default strategy to
//!   [`Model::ReplicateDepthAnythingV2`](crate::ai_router::Model::ReplicateDepthAnythingV2).
//!
//! [`TaskKind::DepthMap`]: crate::ai_router::TaskKind::DepthMap

pub mod commands;
mod pipeline;
mod stub;
mod types;

pub use pipeline::RouterDepthPipeline;
pub use stub::StubDepthPipeline;
pub use types::{DepthInput, DepthPipeline, DepthPipelineError, DepthResult};
