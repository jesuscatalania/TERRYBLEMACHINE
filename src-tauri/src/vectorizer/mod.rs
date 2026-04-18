//! Raster→SVG vectorization via the [`vtracer`] crate.
//!
//! Mirrors [`crate::logo_pipeline`] / [`crate::mesh_pipeline`]: an async
//! trait plus a real [`VtracerPipeline`] and a [`StubVectorizer`] for tests.
//! The production pipeline runs vtracer off the Tokio runtime via
//! `spawn_blocking` (vtracer itself is synchronous) and returns the SVG
//! markup inlined so the frontend can hand it directly to the SvgEditor.

pub mod commands;
pub mod pipeline;
pub mod stub;
pub mod types;

pub use pipeline::VtracerPipeline;
pub use stub::StubVectorizer;
pub use types::{ColorMode, VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};
