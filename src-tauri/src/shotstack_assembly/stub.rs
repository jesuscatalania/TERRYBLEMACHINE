//! Deterministic stub used by tests and as the default backend before a
//! Shotstack API key is configured. Mirrors [`crate::mesh_pipeline::StubMeshPipeline`].

use async_trait::async_trait;

use super::types::{AssemblyError, AssemblyInput, AssemblyResult, VideoAssembler};

#[derive(Default)]
pub struct StubAssembler;

impl StubAssembler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VideoAssembler for StubAssembler {
    async fn assemble(&self, input: AssemblyInput) -> Result<AssemblyResult, AssemblyError> {
        if input.clips.is_empty() {
            return Err(AssemblyError::InvalidInput("clips list is empty".into()));
        }
        Ok(AssemblyResult {
            render_id: "stub-render-id".into(),
            video_url: format!("stub://assembly/{}.mp4", input.clips.len()),
            local_path: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::AssemblyClip;
    use super::*;

    fn clip() -> AssemblyClip {
        AssemblyClip {
            src: "https://a/1.mp4".into(),
            start_s: 0.0,
            length_s: 1.0,
            transition_in: None,
            transition_out: None,
        }
    }

    #[tokio::test]
    async fn returns_deterministic_result() {
        let a = StubAssembler::new();
        let r = a
            .assemble(AssemblyInput {
                clips: vec![clip(), clip()],
                soundtrack: None,
                format: "mp4".into(),
                resolution: "hd".into(),
            })
            .await
            .unwrap();
        assert_eq!(r.render_id, "stub-render-id");
        assert_eq!(r.video_url, "stub://assembly/2.mp4");
        assert!(r.local_path.is_none());
    }

    #[tokio::test]
    async fn rejects_empty_clips() {
        let a = StubAssembler::new();
        let err = a
            .assemble(AssemblyInput {
                clips: vec![],
                soundtrack: None,
                format: "mp4".into(),
                resolution: "hd".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AssemblyError::InvalidInput(_)));
    }
}
