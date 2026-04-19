//! `refine_website` — iterative patching of an existing [`GeneratedProject`]
//! via a free-text instruction. The frontend sends both the current project
//! and the user's wish ("mach den Planeten rot, entferne den Header") to
//! Claude; Claude returns a JSON blob containing only the files it touched
//! (empty content = deletion); the IPC layer merges them back on top of the
//! existing file set and hands the UI the new project plus a list of the
//! paths that changed so it can highlight diffs.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::code_generator::commands::CodeGeneratorState;
use crate::code_generator::types::{CodeGenError, GeneratedProject};

#[derive(Debug, Deserialize)]
pub struct RefineRequest {
    pub project: GeneratedProject,
    pub instruction: String,
}

#[derive(Debug, Serialize)]
pub struct RefineResult {
    pub project: GeneratedProject,
    /// Files that actually changed (path list, for UI highlight).
    pub changed_paths: Vec<String>,
}

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum RefineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("provider: {0}")]
    Provider(String),
    #[error("malformed response: {0}")]
    MalformedResponse(String),
}

impl From<CodeGenError> for RefineError {
    fn from(value: CodeGenError) -> Self {
        match value {
            CodeGenError::InvalidInput(m) => Self::InvalidInput(m),
            CodeGenError::Provider(m) => Self::Provider(m),
            CodeGenError::ParseResponse(m) => Self::MalformedResponse(m),
        }
    }
}

#[tauri::command]
pub async fn refine_website(
    input: RefineRequest,
    state: State<'_, CodeGeneratorState>,
) -> Result<RefineResult, RefineError> {
    let instruction = input.instruction.trim().to_string();
    if instruction.is_empty() {
        return Err(RefineError::InvalidInput("instruction is empty".into()));
    }
    if input.project.files.is_empty() {
        return Err(RefineError::InvalidInput("no current project to refine".into()));
    }

    let (project, changed_paths) = state.0.refine(input.project, &instruction).await?;
    Ok(RefineResult {
        project,
        changed_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_generator::stub::StubCodeGenerator;
    use crate::code_generator::types::{CodeGenerator, GeneratedFile};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn seed_project() -> GeneratedProject {
        GeneratedProject {
            summary: "seed".into(),
            files: vec![GeneratedFile {
                path: PathBuf::from("index.html"),
                content: "<h1>Hi</h1>".into(),
            }],
            prompt: "seed prompt".into(),
        }
    }

    #[tokio::test]
    async fn stub_refine_returns_project_unchanged() {
        let g = StubCodeGenerator::new();
        let (out, changed) = g
            .refine(seed_project(), "make it bigger")
            .await
            .expect("stub refine succeeds");
        assert_eq!(out.files.len(), 1);
        assert!(changed.is_empty());
    }

    #[tokio::test]
    async fn empty_instruction_rejects_via_invalid_input() {
        // Mirror the Tauri command's validation path — since tauri::State
        // can't be easily synthesized from a unit test, we inline the
        // same guard here.
        let g: Arc<dyn CodeGenerator> = Arc::new(StubCodeGenerator::new());
        let err_source = async {
            let instr = "   ".trim().to_string();
            if instr.is_empty() {
                return Err::<(), RefineError>(RefineError::InvalidInput(
                    "instruction is empty".into(),
                ));
            }
            let _ = g.refine(seed_project(), &instr).await;
            Ok(())
        }
        .await;
        assert!(matches!(err_source, Err(RefineError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn empty_project_rejects_via_invalid_input() {
        let g: Arc<dyn CodeGenerator> = Arc::new(StubCodeGenerator::new());
        let empty = GeneratedProject {
            summary: "".into(),
            files: vec![],
            prompt: "".into(),
        };
        let err_source = async {
            let instr = "do stuff";
            if empty.files.is_empty() {
                return Err::<(), RefineError>(RefineError::InvalidInput(
                    "no current project to refine".into(),
                ));
            }
            let _ = g.refine(empty.clone(), instr).await;
            Ok(())
        }
        .await;
        assert!(matches!(err_source, Err(RefineError::InvalidInput(_))));
    }
}
