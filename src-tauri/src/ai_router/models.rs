//! Type definitions shared across the AI router.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::errors::ProviderError;

/// The external service behind a model. Nine providers match `docs/LLM-STRATEGIE.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Claude,
    Kling,
    Runway,
    Higgsfield,
    Shotstack,
    Ideogram,
    Meshy,
    Fal,
    Replicate,
}

/// Concrete models the router can dispatch to. The enum stays flat so the
/// routing strategy is a simple pattern match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Model {
    // Claude
    ClaudeOpus,
    ClaudeSonnet,
    ClaudeHaiku,
    // Kling
    Kling20,
    // Runway
    RunwayGen3,
    // Higgsfield
    HiggsfieldMulti,
    // Shotstack
    ShotstackMontage,
    // Ideogram
    IdeogramV3,
    // Meshy
    MeshyText3D,
    MeshyImage3D,
    // fal.ai
    FalFluxPro,
    FalSdxl,
    FalRealEsrgan,
    FalFluxFill,
    // Replicate (catch-all; the slug specifies the underlying model)
    ReplicateFluxDev,
}

impl Model {
    /// Which provider hosts this model.
    pub fn provider(self) -> Provider {
        match self {
            Self::ClaudeOpus | Self::ClaudeSonnet | Self::ClaudeHaiku => Provider::Claude,
            Self::Kling20 => Provider::Kling,
            Self::RunwayGen3 => Provider::Runway,
            Self::HiggsfieldMulti => Provider::Higgsfield,
            Self::ShotstackMontage => Provider::Shotstack,
            Self::IdeogramV3 => Provider::Ideogram,
            Self::MeshyText3D | Self::MeshyImage3D => Provider::Meshy,
            Self::FalFluxPro | Self::FalSdxl | Self::FalRealEsrgan | Self::FalFluxFill => {
                Provider::Fal
            }
            Self::ReplicateFluxDev => Provider::Replicate,
        }
    }
}

/// What the user is trying to do. Drives [`crate::ai_router::RoutingStrategy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskKind {
    /// Code gen, copy, reasoning, classification.
    TextGeneration,
    /// Text-to-image.
    ImageGeneration,
    /// Image-to-image (refine, stylize).
    ImageEdit,
    /// Inpainting / region fill.
    Inpaint,
    /// Upscaling.
    Upscale,
    /// Logo / typo work (text-in-image).
    Logo,
    /// Text-to-video.
    TextToVideo,
    /// Image-to-video.
    ImageToVideo,
    /// Video montage / assembly.
    VideoMontage,
    /// Text-to-3D mesh.
    Text3D,
    /// Image-to-3D mesh.
    Image3D,
}

/// Priority within the router's queue. Higher variants are dequeued first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
}

/// Complexity hint — helps the default strategy pick cheap vs premium models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

/// A generative request heading into the router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub id: String,
    pub task: TaskKind,
    #[serde(default = "default_priority")]
    pub priority: Priority,
    #[serde(default = "default_complexity")]
    pub complexity: Complexity,
    pub prompt: String,
    /// Provider-specific additional inputs (image URLs, tuning params, etc).
    #[serde(default)]
    pub payload: serde_json::Value,
}

fn default_priority() -> Priority {
    Priority::Normal
}
fn default_complexity() -> Complexity {
    Complexity::Medium
}

/// A successful generative response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub request_id: String,
    pub model: Model,
    pub output: serde_json::Value,
    /// Cost in cents (integer) when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cents: Option<u32>,
    /// True when served from the router's cache (Schritt 2.3).
    #[serde(default)]
    pub cached: bool,
}

/// Provider-client abstraction. Individual clients land in `api_clients/` in
/// Schritt 2.2.
///
/// Method names use `execute` (internal router vocabulary). Public impls in
/// `api_clients/` may expose richer domain methods (e.g. `send_request`,
/// `stream_completion`) on top.
#[async_trait]
pub trait AiClient: Send + Sync {
    fn provider(&self) -> Provider;
    fn supports(&self, model: Model) -> bool;
    async fn execute(&self, model: Model, request: &AiRequest)
        -> Result<AiResponse, ProviderError>;
    async fn health_check(&self) -> bool;
    /// Optional: service-reported usage (credits, rate-limit headers, etc.).
    /// Implementations that don't publish usage data return an empty struct.
    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError>;
}

/// Provider-reported usage snapshot. All fields are optional because each
/// service exposes a different subset.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProviderUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_used: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_remaining: Option<u64>,
    /// RFC 3339 timestamp at which rate-limit window resets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_reset: Option<String>,
    /// Human-readable note (e.g. plan name, quota label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
