//! Routing strategy + retry policy.

use std::time::Duration;

use super::models::{AiRequest, Complexity, Model, TaskKind};

/// What the router will try, in order: primary first, then each fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    pub primary: Model,
    pub fallbacks: Vec<Model>,
}

impl RouteDecision {
    fn new(primary: Model) -> Self {
        Self {
            primary,
            fallbacks: Vec::new(),
        }
    }
    fn with_fallbacks(primary: Model, fallbacks: Vec<Model>) -> Self {
        Self { primary, fallbacks }
    }
}

/// Strategy interface so tests (or future config-driven routing) can swap
/// in alternative logic without changing the router.
pub trait RoutingStrategy: Send + Sync {
    fn select(&self, request: &AiRequest) -> RouteDecision;
}

/// Default routing table derived from `docs/LLM-STRATEGIE.md`.
pub struct DefaultRoutingStrategy;

impl RoutingStrategy for DefaultRoutingStrategy {
    fn select(&self, request: &AiRequest) -> RouteDecision {
        use Complexity::*;
        use Model::*;
        use TaskKind::*;

        match (request.task, request.complexity) {
            // Text / code — cost-tier mapping.
            (TextGeneration, Complex) => {
                RouteDecision::with_fallbacks(ClaudeOpus, vec![ClaudeSonnet])
            }
            (TextGeneration, Medium) => {
                RouteDecision::with_fallbacks(ClaudeSonnet, vec![ClaudeHaiku])
            }
            (TextGeneration, Simple) => RouteDecision::new(ClaudeHaiku),

            // Images — fal.ai is cheaper & faster; complex prompts get Flux Pro.
            //
            // Naming note: `FalFluxPro` is fal.ai's current top-tier image
            // endpoint (`fal-ai/flux-pro`, also marketed as Flux 1.1 Pro). The
            // plan document calls it "Flux 2 Pro" — that name does not
            // currently exist in the fal.ai catalog, so we track the actual
            // production endpoint. If fal.ai publishes a genuine
            // `flux-2-pro` later, add `Model::FalFlux2Pro` and swap this
            // mapping.
            (ImageGeneration, Simple) => {
                RouteDecision::with_fallbacks(FalSdxl, vec![FalFluxPro, ReplicateFluxDev])
            }
            (ImageGeneration, Medium) => {
                RouteDecision::with_fallbacks(FalFluxPro, vec![ReplicateFluxDev])
            }
            (ImageGeneration, Complex) => {
                RouteDecision::with_fallbacks(FalFluxPro, vec![ReplicateFluxDev])
            }

            (ImageEdit, _) => RouteDecision::with_fallbacks(FalFluxPro, vec![ReplicateFluxDev]),
            (Inpaint, _) => RouteDecision::new(FalFluxFill),
            (Upscale, _) => RouteDecision::new(FalRealEsrgan),

            // Logos — only Ideogram reliably renders text inside images.
            (Logo, _) => RouteDecision::new(IdeogramV3),

            // Video — Kling V2 master (via fal.ai aggregator) is the new
            // default. V1.5 (also via fal) is the cheaper fallback; Runway
            // and Higgsfield sit at the tail of the chain as last-resort
            // options if fal.ai is down. The direct Kling client
            // (`Model::Kling20`) is intentionally NOT in this chain — it
            // stays registered so advanced users can opt in by configuring
            // a direct Kling key, but the default flow only needs the
            // single `fal` keychain entry.
            (TextToVideo, _) | (ImageToVideo, _) => RouteDecision::with_fallbacks(
                FalKlingV2Master,
                vec![FalKlingV15, RunwayGen3, HiggsfieldMulti],
            ),
            (VideoMontage, _) => RouteDecision::new(ShotstackMontage),

            // 3D — Meshy is the Pro-grade default. TripoSR (on Replicate)
            // is the quick-preview tier: cheaper + faster but lower-fidelity.
            // Complexity::Simple opts into TripoSR; Medium/Complex stays
            // Meshy-primary with TripoSR as the fallback.
            (Text3D, _) => RouteDecision::new(MeshyText3D),
            (Image3D, Complexity::Simple) => {
                RouteDecision::with_fallbacks(ReplicateTripoSR, vec![MeshyImage3D])
            }
            (Image3D, _) => RouteDecision::with_fallbacks(MeshyImage3D, vec![ReplicateTripoSR]),

            // Vision analysis — Claude Sonnet leads; Haiku is the cheaper
            // fallback for simple reference-image extraction.
            (ImageAnalysis, _) => RouteDecision::with_fallbacks(ClaudeSonnet, vec![ClaudeHaiku]),

            // Depth maps — only Depth-Anything v2 on Replicate for now.
            (DepthMap, _) => RouteDecision::new(ReplicateDepthAnythingV2),
        }
    }
}

// ─── Retry policy ──────────────────────────────────────────────────────────

/// Exponential backoff with a ceiling. Default: 3 attempts, 200ms base, 2×
/// factor, 5s max (so delays are 200ms, 400ms, 800ms, … up to 5s).
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base: Duration,
    pub factor: u32,
    pub max: Duration,
}

impl RetryPolicy {
    pub fn default_policy() -> Self {
        Self {
            max_attempts: 3,
            base: Duration::from_millis(200),
            factor: 2,
            max: Duration::from_secs(5),
        }
    }

    /// Delay before the *next* attempt, given zero-indexed attempt number.
    pub fn backoff_for(&self, attempt: u32) -> Duration {
        let multiplier = self.factor.saturating_pow(attempt);
        let raw = self.base.saturating_mul(multiplier);
        if raw > self.max {
            self.max
        } else {
            raw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::models::{Priority, TaskKind};

    fn req(task: TaskKind, complexity: Complexity) -> AiRequest {
        AiRequest {
            id: "t".into(),
            task,
            priority: Priority::Normal,
            complexity,
            prompt: String::new(),
            payload: serde_json::Value::Null,
        }
    }

    // ── Routing ───────────────────────────────────────────────────────────

    #[test]
    fn text_complex_uses_opus_falls_back_to_sonnet() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::TextGeneration, Complexity::Complex));
        assert_eq!(d.primary, Model::ClaudeOpus);
        assert_eq!(d.fallbacks, vec![Model::ClaudeSonnet]);
    }

    #[test]
    fn text_medium_uses_sonnet() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::TextGeneration, Complexity::Medium));
        assert_eq!(d.primary, Model::ClaudeSonnet);
        assert_eq!(d.fallbacks, vec![Model::ClaudeHaiku]);
    }

    #[test]
    fn text_simple_uses_haiku_no_fallback() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::TextGeneration, Complexity::Simple));
        assert_eq!(d.primary, Model::ClaudeHaiku);
        assert!(d.fallbacks.is_empty());
    }

    #[test]
    fn image_simple_uses_sdxl() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageGeneration, Complexity::Simple));
        assert_eq!(d.primary, Model::FalSdxl);
        assert_eq!(
            d.fallbacks,
            vec![Model::FalFluxPro, Model::ReplicateFluxDev]
        );
    }

    #[test]
    fn image_generation_simple_falls_back_to_replicate() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageGeneration, Complexity::Simple));
        assert_eq!(d.primary, Model::FalSdxl);
        assert!(d.fallbacks.contains(&Model::ReplicateFluxDev));
    }

    #[test]
    fn image_medium_uses_flux_pro_with_replicate_fallback() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageGeneration, Complexity::Medium));
        assert_eq!(d.primary, Model::FalFluxPro);
        assert_eq!(d.fallbacks, vec![Model::ReplicateFluxDev]);
    }

    #[test]
    fn image_generation_complex_has_replicate_fallback() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageGeneration, Complexity::Complex));
        assert_eq!(d.primary, Model::FalFluxPro);
        assert!(d.fallbacks.contains(&Model::ReplicateFluxDev));
    }

    #[test]
    fn image_edit_falls_back_to_replicate() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageEdit, Complexity::Medium));
        assert_eq!(d.primary, Model::FalFluxPro);
        assert!(d.fallbacks.contains(&Model::ReplicateFluxDev));
    }

    #[test]
    fn logo_uses_ideogram_only() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Logo, Complexity::Medium));
        assert_eq!(d.primary, Model::IdeogramV3);
        assert!(d.fallbacks.is_empty());
    }

    #[test]
    fn video_chain_uses_fal_kling_then_runway_then_higgsfield() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::TextToVideo, Complexity::Medium));
        assert_eq!(d.primary, Model::FalKlingV2Master);
        assert_eq!(
            d.fallbacks,
            vec![
                Model::FalKlingV15,
                Model::RunwayGen3,
                Model::HiggsfieldMulti,
            ]
        );
    }

    #[test]
    fn image_to_video_shares_text_to_video_chain() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageToVideo, Complexity::Medium));
        assert_eq!(d.primary, Model::FalKlingV2Master);
        assert_eq!(
            d.fallbacks,
            vec![
                Model::FalKlingV15,
                Model::RunwayGen3,
                Model::HiggsfieldMulti,
            ]
        );
    }

    #[test]
    fn video_montage_uses_shotstack() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::VideoMontage, Complexity::Medium));
        assert_eq!(d.primary, Model::ShotstackMontage);
    }

    #[test]
    fn upscale_uses_real_esrgan() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Upscale, Complexity::Simple));
        assert_eq!(d.primary, Model::FalRealEsrgan);
    }

    #[test]
    fn text_3d_uses_meshy() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Text3D, Complexity::Medium));
        assert_eq!(d.primary, Model::MeshyText3D);
    }

    #[test]
    fn image_3d_uses_meshy_image_endpoint() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Image3D, Complexity::Medium));
        assert_eq!(d.primary, Model::MeshyImage3D);
    }

    #[test]
    fn image_3d_simple_routes_to_triposr_first() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Image3D, Complexity::Simple));
        assert_eq!(d.primary, Model::ReplicateTripoSR);
        assert_eq!(d.fallbacks, vec![Model::MeshyImage3D]);
    }

    #[test]
    fn image_3d_medium_routes_to_meshy_with_triposr_fallback() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Image3D, Complexity::Medium));
        assert_eq!(d.primary, Model::MeshyImage3D);
        assert!(d.fallbacks.contains(&Model::ReplicateTripoSR));
    }

    #[test]
    fn image_3d_complex_routes_to_meshy_with_triposr_fallback() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::Image3D, Complexity::Complex));
        assert_eq!(d.primary, Model::MeshyImage3D);
        assert!(d.fallbacks.contains(&Model::ReplicateTripoSR));
    }

    #[test]
    fn image_analysis_uses_sonnet_with_haiku_fallback() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::ImageAnalysis, Complexity::Medium));
        assert_eq!(d.primary, Model::ClaudeSonnet);
        assert_eq!(d.fallbacks, vec![Model::ClaudeHaiku]);
    }

    #[test]
    fn depth_map_routes_to_depth_anything_v2() {
        let d = DefaultRoutingStrategy.select(&req(TaskKind::DepthMap, Complexity::Medium));
        assert_eq!(d.primary, Model::ReplicateDepthAnythingV2);
        assert!(d.fallbacks.is_empty());
    }

    // ── Retry policy ──────────────────────────────────────────────────────

    #[test]
    fn retry_default_policy_grows_exponentially() {
        let p = RetryPolicy::default_policy();
        assert_eq!(p.backoff_for(0), Duration::from_millis(200));
        assert_eq!(p.backoff_for(1), Duration::from_millis(400));
        assert_eq!(p.backoff_for(2), Duration::from_millis(800));
    }

    #[test]
    fn retry_backoff_clamped_to_max() {
        let p = RetryPolicy::default_policy();
        assert_eq!(p.backoff_for(20), p.max);
    }

    #[test]
    fn retry_zero_policy_returns_zero() {
        let p = RetryPolicy {
            max_attempts: 1,
            base: Duration::from_millis(0),
            factor: 1,
            max: Duration::from_millis(0),
        };
        assert_eq!(p.backoff_for(0), Duration::ZERO);
        assert_eq!(p.backoff_for(5), Duration::ZERO);
    }
}
