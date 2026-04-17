//! Project exporter — turns a [`GeneratedProject`](crate::code_generator::GeneratedProject)
//! into a deployable artefact (currently a ZIP file; React / Next.js
//! wrappers follow in Phase 3 follow-ups).

pub mod commands;
mod zip_export;

pub use zip_export::{export_to_zip, DeployTarget, ExportError, ExportFormat, ExportRequest};
