//! Website code generator — turns a user brief into a multi-file React +
//! Tailwind project.
//!
//! The code generator is trait-based so it can be driven by:
//! - [`StubCodeGenerator`] in tests (deterministic, no network),
//! - [`ClaudeCodeGenerator`] in production (dispatches through the shared
//!   [`AiClient`](crate::ai_router::AiClient)),
//! - anything else that implements [`CodeGenerator`] in Phase 3+ follow-ups.
//!
//! The prompt is built from (a) the user's brief, (b) the chosen
//! [`Template`], (c) optional `AnalysisResult` from a reference URL, and
//! (d) the live `meingeschmack/` taste profile when available.

pub mod assist;
mod claude;
pub mod commands;
pub mod optimize;
pub mod preview;
mod prompt;
pub mod refine;
pub(crate) mod stub;
mod templates;
mod types;

pub use assist::{modify_code_selection, ModifyRequest, ModifyResponse};
pub use claude::ClaudeCodeGenerator;
pub use preview::open_project_in_browser;
pub use prompt::build_prompt;
pub use refine::refine_website;
pub use stub::StubCodeGenerator;
pub use templates::{Template, TEMPLATE_LANDING, TEMPLATE_PORTFOLIO};
pub use types::{CodeGenError, CodeGenerator, GeneratedFile, GeneratedProject, GenerationInput};
