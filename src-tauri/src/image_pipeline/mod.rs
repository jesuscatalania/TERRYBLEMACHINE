//! Image generation pipeline — the Rust-side orchestrator for the
//! 2D-graphics module.
//!
//! All public methods go through the shared [`AiRouter`](crate::ai_router::AiRouter)
//! so they inherit the cache, budget, retry, and fallback machinery built in
//! Phase 2. Taste-engine integration happens here: prompts are enriched with
//! the live `meingeschmack/` profile + optional per-module tags before
//! dispatch.
//!
//! For Schritt 4.1 the module exposes:
//! - `text_to_image(prompt, complexity)` — primary via fal.ai Flux Pro,
//!   falls back through the router's default strategy to Replicate.
//! - `image_to_image(prompt, image_url)` — stylize / variations.
//! - `upscale(image_url)` — Real-ESRGAN via fal.ai.
//! - `variants(prompt, n)` — n parallel text-to-image calls for picker UIs.

pub mod commands;
mod pipeline;
mod stub;
mod types;

pub use pipeline::RouterImagePipeline;
pub use stub::StubImagePipeline;
pub use types::{
    GenerateVariantsInput, Image2ImageInput, ImagePipeline, ImagePipelineError, ImageResult,
    InpaintInput, Text2ImageInput, UpscaleInput,
};
