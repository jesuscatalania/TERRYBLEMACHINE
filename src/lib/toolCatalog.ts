//! Central capability matrix: TaskKind → ToolDef[] in priority tier order.
//!
//! Drives both the per-prompt Tool dropdown UI AND the override-slug
//! resolver. Must stay in sync with src-tauri/src/ai_router/router.rs's
//! DefaultRoutingStrategy::select.

export type TaskKind =
  | "TextGeneration"
  | "ImageGeneration"
  | "ImageEdit"
  | "Inpaint"
  | "Upscale"
  | "Logo"
  | "TextToVideo"
  | "ImageToVideo"
  | "VideoMontage"
  | "Text3D"
  | "Image3D"
  | "ImageAnalysis"
  | "DepthMap";

export type Tier = "primary" | "fallback" | "alternative";

export interface ToolDef {
  /** Stable id used as the override slug suffix and Model identifier. */
  id: string;
  /** Display name in the dropdown. */
  label: string;
  /** Provider/host. */
  provider:
    | "claude"
    | "fal"
    | "replicate"
    | "ideogram"
    | "kling"
    | "runway"
    | "higgsfield"
    | "shotstack"
    | "meshy"
    | "remotion";
  /** Backend Model enum variant (PascalCase, matches Rust). */
  model: string;
  tier: Tier;
}

export const CATALOG: Record<TaskKind, readonly ToolDef[]> = {
  TextGeneration: [
    {
      id: "claude-opus",
      label: "Claude Opus",
      provider: "claude",
      model: "ClaudeOpus",
      tier: "primary",
    },
    {
      id: "claude-sonnet",
      label: "Claude Sonnet",
      provider: "claude",
      model: "ClaudeSonnet",
      tier: "fallback",
    },
    {
      id: "claude-haiku",
      label: "Claude Haiku",
      provider: "claude",
      model: "ClaudeHaiku",
      tier: "fallback",
    },
  ],
  ImageGeneration: [
    {
      id: "fal-flux-pro",
      label: "Flux Pro 1.1",
      provider: "fal",
      model: "FalFluxPro",
      tier: "primary",
    },
    { id: "fal-sdxl", label: "SDXL Fast", provider: "fal", model: "FalSdxl", tier: "fallback" },
    {
      id: "replicate-flux-dev",
      label: "Flux Dev",
      provider: "replicate",
      model: "ReplicateFluxDev",
      tier: "fallback",
    },
  ],
  ImageEdit: [
    {
      id: "fal-flux-pro",
      label: "Flux Pro 1.1",
      provider: "fal",
      model: "FalFluxPro",
      tier: "primary",
    },
    {
      id: "replicate-flux-dev",
      label: "Flux Dev",
      provider: "replicate",
      model: "ReplicateFluxDev",
      tier: "fallback",
    },
  ],
  Inpaint: [
    {
      id: "fal-flux-fill",
      label: "Flux Fill (inpaint)",
      provider: "fal",
      model: "FalFluxFill",
      tier: "primary",
    },
  ],
  Upscale: [
    {
      id: "fal-real-esrgan",
      label: "Real-ESRGAN",
      provider: "fal",
      model: "FalRealEsrgan",
      tier: "primary",
    },
  ],
  Logo: [
    {
      id: "ideogram-v3",
      label: "Ideogram v3",
      provider: "ideogram",
      model: "IdeogramV3",
      tier: "primary",
    },
  ],
  TextToVideo: [
    {
      id: "fal-kling-v2-master",
      label: "Kling V2 Master (via fal)",
      provider: "fal",
      model: "FalKlingV2Master",
      tier: "primary",
    },
    {
      id: "fal-kling-v15",
      label: "Kling V1.5 (via fal)",
      provider: "fal",
      model: "FalKlingV15",
      tier: "fallback",
    },
    {
      id: "runway-gen3",
      label: "Runway Gen-3",
      provider: "runway",
      model: "RunwayGen3",
      tier: "fallback",
    },
    {
      id: "higgsfield",
      label: "Higgsfield",
      provider: "higgsfield",
      model: "HiggsfieldMulti",
      tier: "fallback",
    },
  ],
  ImageToVideo: [
    {
      id: "fal-kling-v2-master",
      label: "Kling V2 Master (via fal)",
      provider: "fal",
      model: "FalKlingV2Master",
      tier: "primary",
    },
    {
      id: "fal-kling-v15",
      label: "Kling V1.5 (via fal)",
      provider: "fal",
      model: "FalKlingV15",
      tier: "fallback",
    },
    {
      id: "runway-gen3",
      label: "Runway Gen-3",
      provider: "runway",
      model: "RunwayGen3",
      tier: "fallback",
    },
    {
      id: "higgsfield",
      label: "Higgsfield",
      provider: "higgsfield",
      model: "HiggsfieldMulti",
      tier: "fallback",
    },
  ],
  VideoMontage: [
    {
      id: "shotstack",
      label: "Shotstack (cloud)",
      provider: "shotstack",
      model: "ShotstackMontage",
      tier: "primary",
    },
    {
      id: "remotion",
      label: "Remotion (local)",
      provider: "remotion",
      model: "RemotionLocal",
      tier: "alternative",
    },
  ],
  Text3D: [
    {
      id: "meshy-text-3d",
      label: "Meshy Text-3D",
      provider: "meshy",
      model: "MeshyText3D",
      tier: "primary",
    },
    {
      id: "replicate-triposr",
      label: "TripoSR (preview)",
      provider: "replicate",
      model: "ReplicateTripoSR",
      tier: "alternative",
    },
  ],
  Image3D: [
    {
      id: "meshy-image-3d",
      label: "Meshy Image-3D",
      provider: "meshy",
      model: "MeshyImage3D",
      tier: "primary",
    },
    {
      id: "replicate-triposr",
      label: "TripoSR (preview)",
      provider: "replicate",
      model: "ReplicateTripoSR",
      tier: "fallback",
    },
  ],
  ImageAnalysis: [
    {
      id: "claude-sonnet",
      label: "Claude Sonnet",
      provider: "claude",
      model: "ClaudeSonnet",
      tier: "primary",
    },
    {
      id: "claude-haiku",
      label: "Claude Haiku",
      provider: "claude",
      model: "ClaudeHaiku",
      tier: "fallback",
    },
  ],
  DepthMap: [
    {
      id: "replicate-depth-anything-v2",
      label: "Depth-Anything v2",
      provider: "replicate",
      model: "ReplicateDepthAnythingV2",
      tier: "primary",
    },
  ],
};

export function getToolsFor(task: TaskKind): readonly ToolDef[] {
  return CATALOG[task] ?? [];
}
