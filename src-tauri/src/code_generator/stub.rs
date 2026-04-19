//! Deterministic test double for the code generator.

use std::path::PathBuf;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::types::{CodeGenError, CodeGenerator, GeneratedFile, GeneratedProject, GenerationInput};

#[derive(Default)]
pub struct StubCodeGenerator {
    last_input: Mutex<Option<GenerationInput>>,
    force_error: Mutex<Option<String>>,
}

impl StubCodeGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_input(&self) -> Option<GenerationInput> {
        self.last_input.lock().clone()
    }

    pub fn force_error(&self, message: impl Into<String>) {
        *self.force_error.lock() = Some(message.into());
    }
}

#[async_trait]
impl CodeGenerator for StubCodeGenerator {
    async fn generate(&self, input: GenerationInput) -> Result<GeneratedProject, CodeGenError> {
        if let Some(msg) = self.force_error.lock().clone() {
            return Err(CodeGenError::Provider(msg));
        }
        *self.last_input.lock() = Some(input.clone());
        Ok(GeneratedProject {
            summary: format!("Stub project for \"{}\"", input.prompt.trim()),
            files: vec![
                GeneratedFile {
                    path: PathBuf::from("index.html"),
                    content: format!(
                        "<!doctype html>\n<html><head><meta charset=\"utf-8\"/><title>{}</title></head>\n<body><h1>{}</h1></body></html>",
                        input.prompt.trim(),
                        input.prompt.trim(),
                    ),
                },
                GeneratedFile {
                    path: PathBuf::from("styles.css"),
                    content: "body { font-family: system-ui; margin: 0; }\n".to_string(),
                },
            ],
            prompt: format!("(stub) {}", input.prompt.trim()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_generator::templates::Template;

    fn input(prompt: &str) -> GenerationInput {
        GenerationInput {
            prompt: prompt.into(),
            template: Template::LandingPage,
            reference: None,
            image_path: None,
            module: "website".into(),
            model_override: None,
        }
    }

    #[tokio::test]
    async fn returns_deterministic_files_for_a_brief() {
        let g = StubCodeGenerator::new();
        let project = g.generate(input("Coffee shop")).await.unwrap();
        assert_eq!(project.files.len(), 2);
        assert!(project.files[0].path.ends_with("index.html"));
        assert!(project.files[0].content.contains("Coffee shop"));
        assert_eq!(project.summary, "Stub project for \"Coffee shop\"");
    }

    #[tokio::test]
    async fn captures_last_input_for_assertions() {
        let g = StubCodeGenerator::new();
        let _ = g.generate(input("A")).await.unwrap();
        let _ = g.generate(input("B")).await.unwrap();
        let last = g.last_input().expect("should have captured");
        assert_eq!(last.prompt, "B");
    }

    #[tokio::test]
    async fn force_error_short_circuits_generation() {
        let g = StubCodeGenerator::new();
        g.force_error("upstream blew up");
        let err = g.generate(input("x")).await.unwrap_err();
        assert!(matches!(err, CodeGenError::Provider(_)));
    }
}
