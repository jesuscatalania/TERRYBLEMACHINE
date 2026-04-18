//! Logo generation pipeline — the Rust-side orchestrator for the
//! Typography "Generate Logo" flow.
//!
//! Mirrors [`crate::image_pipeline`] / [`crate::mesh_pipeline`]: all public
//! methods dispatch through the shared [`AiRouter`](crate::ai_router::AiRouter)
//! so they inherit cache, budget, retry, and fallback behaviour. The
//! distinguishing piece vs. `image_pipeline` is the parallel-variants
//! orchestration: one call to [`LogoPipeline::generate_variants`] fires up
//! to ten seed-salted requests in parallel via [`futures::future::join_all`]
//! and downloads each PNG into the platform cache dir for offline preview.
//!
//! Routes [`TaskKind::Logo`] → [`Model::IdeogramV3`] via the default
//! strategy wired in Phase 2.
//!
//! [`TaskKind::Logo`]: crate::ai_router::TaskKind::Logo
//! [`Model::IdeogramV3`]: crate::ai_router::Model::IdeogramV3

pub mod commands;
pub mod pipeline;
pub mod stub;
pub mod types;

pub use pipeline::RouterLogoPipeline;
pub use stub::StubLogoPipeline;
pub use types::{LogoInput, LogoPipeline, LogoPipelineError, LogoStyle, LogoVariant};
