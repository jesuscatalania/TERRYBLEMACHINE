//! Storyboard generator — turns a user brief + template into a structured
//! shot list, parsed from Claude's JSON response.
//!
//! Mirrors [`crate::code_generator`]'s architecture:
//! - [`StubStoryboardGenerator`] for deterministic tests.
//! - [`ClaudeStoryboardGenerator`] for production (routes through
//!   [`crate::ai_router::AiRouter`]).
//! - Taste-engine enrichment is opt-in via `with_taste_engine`.

pub mod commands;
pub mod generator;
pub mod prompt;
pub mod stub;
pub mod types;

pub use generator::ClaudeStoryboardGenerator;
pub use prompt::build_prompt;
pub use stub::StubStoryboardGenerator;
pub use types::{
    Shot, Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput, StoryboardTemplate,
};
