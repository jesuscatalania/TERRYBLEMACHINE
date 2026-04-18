//! Shotstack video-assembly pipeline — the Rust-side orchestrator for the
//! Video module's "Assemble timeline" flow.
//!
//! Mirrors [`crate::mesh_pipeline`] in shape (typed inputs, `async_trait`
//! pipeline, stub backend, IPC commands) but **does not** route through
//! [`AiRouter`](crate::ai_router::AiRouter): Shotstack's timeline JSON is
//! provider-specific and there is no cross-provider fallback for it. Instead
//! the [`ShotstackAssembler`] holds an `Arc<ShotstackClient>` directly, calls
//! [`assemble_timeline`](crate::api_clients::shotstack::ShotstackClient::assemble_timeline)
//! followed by [`poll_render`](crate::api_clients::shotstack::ShotstackClient::poll_render),
//! and then downloads the finished MP4 into the platform cache dir so the
//! Remotion preview can load it from a local path without HTTP round-trips.
//!
//! Exposes:
//! - `assemble(input)` — builds a Shotstack timeline body from
//!   [`AssemblyInput`] + [`AssemblyClip`] and resolves to an
//!   [`AssemblyResult`] with the remote URL, the local cache path (best-effort),
//!   and the render id for observability.

pub mod commands;
mod pipeline;
mod stub;
mod types;

pub use pipeline::ShotstackAssembler;
pub use stub::StubAssembler;
pub use types::{AssemblyClip, AssemblyError, AssemblyInput, AssemblyResult, VideoAssembler};
