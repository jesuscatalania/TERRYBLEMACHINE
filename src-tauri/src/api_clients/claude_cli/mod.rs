//! Claude CLI Bridge — wraps the locally-installed `claude` binary so we
//! can use the user's Claude Pro/Max subscription instead of paying for
//! API credits. See `docs/superpowers/plans/2026-04-19-phase-9-claude-bridge-and-tool-ux.md`.

pub mod discovery;
pub mod stream_parser;
