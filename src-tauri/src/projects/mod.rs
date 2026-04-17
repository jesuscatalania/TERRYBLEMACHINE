//! Project management: filesystem-backed CRUD over
//! `~/Documents/TERRYBLEMACHINE/projects/[slug]/project.json`.
//!
//! Tests pass a temporary root via [`FileProjectStore::new`]; production code
//! calls [`default_root`] (resolved from the user's documents directory).

mod errors;
mod storage;

pub mod commands;
pub mod history_commands;

pub use errors::ProjectError;
pub use storage::{FileProjectStore, NewProject, Project};

use std::path::PathBuf;

/// Default projects root: `<documents_dir>/TERRYBLEMACHINE/projects`.
///
/// Returns `None` when the documents directory cannot be resolved (e.g. in
/// unusual sandboxed environments).
pub fn default_root(documents_dir: PathBuf) -> PathBuf {
    documents_dir.join("TERRYBLEMACHINE").join("projects")
}
