//! Tauri IPC command for the code generator.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::types::{CodeGenError, CodeGenerator, GeneratedProject, GenerationInput};

pub struct CodeGeneratorState(pub Arc<dyn CodeGenerator>);

impl CodeGeneratorState {
    pub fn new(generator: Arc<dyn CodeGenerator>) -> Self {
        Self(generator)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum CodeGenIpcError {
    InvalidInput(String),
    Provider(String),
    ParseResponse(String),
}

impl From<CodeGenError> for CodeGenIpcError {
    fn from(value: CodeGenError) -> Self {
        match value {
            CodeGenError::InvalidInput(m) => Self::InvalidInput(m),
            CodeGenError::Provider(m) => Self::Provider(m),
            CodeGenError::ParseResponse(m) => Self::ParseResponse(m),
        }
    }
}

#[tauri::command]
pub async fn generate_website(
    input: GenerationInput,
    state: State<'_, CodeGeneratorState>,
) -> Result<GeneratedProject, CodeGenIpcError> {
    state.0.generate(input).await.map_err(Into::into)
}
