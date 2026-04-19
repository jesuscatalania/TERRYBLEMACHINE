//! Parse `/tool` overrides from the start or end of a prompt.
//!
//! Rules:
//! - Slug must be at start (matches `^/<slug>\s+`) OR at end (matches `\s+/<slug>$`)
//! - Mid-string `/slug` is ignored (user typed a slash for unrelated reason)
//! - First-position wins if both ends carry a slug
//! - Slug is case-insensitive; whitelist-checked against OVERRIDE_ALIASES
//! - Unknown slugs are NOT consumed — left in cleanPrompt so user sees them

/** Map of override slug → backend Model enum string (PascalCase). */
export const OVERRIDE_ALIASES: Record<string, string> = {
  // Claude
  claude: "ClaudeSonnet",
  "claude-opus": "ClaudeOpus",
  "claude-sonnet": "ClaudeSonnet",
  "claude-haiku": "ClaudeHaiku",
  // fal images
  sdxl: "FalSdxl",
  flux: "FalFluxPro",
  "flux-pro": "FalFluxPro",
  "flux-fill": "FalFluxFill",
  upscale: "FalRealEsrgan",
  // Replicate
  "flux-dev": "ReplicateFluxDev",
  triposr: "ReplicateTripoSR",
  depth: "ReplicateDepthAnythingV2",
  // Logo
  ideogram: "IdeogramV3",
  // Video (fal-aggregated Kling)
  kling: "FalKlingV2Master",
  "kling-v2": "FalKlingV2Master",
  "kling-v15": "FalKlingV15",
  // Direct video providers
  runway: "RunwayGen3",
  higgsfield: "HiggsfieldMulti",
  // Video montage
  shotstack: "ShotstackMontage",
  remotion: "RemotionLocal",
  // 3D
  meshy: "MeshyText3D",
  "meshy-text": "MeshyText3D",
  "meshy-image": "MeshyImage3D",
};

export interface ParsedOverride {
  override?: string;
  cleanPrompt: string;
  slugLocation?: "start" | "end";
}

const SLUG_RE = /^[a-z0-9][a-z0-9-]*$/;

function isKnownSlug(slug: string): boolean {
  // Object.hasOwn (ES2022) would be cleaner, but the project targets ES2020;
  // hasOwnProperty.call is the safe equivalent and excludes prototype keys
  // (e.g. "constructor", "toString") which would otherwise pass `in`.
  // biome-ignore lint/suspicious/noPrototypeBuiltins: ES2020 target, no Object.hasOwn
  return Object.prototype.hasOwnProperty.call(OVERRIDE_ALIASES, slug.toLowerCase());
}

export function parseOverride(input: string): ParsedOverride {
  const trimmed = input.trim();
  // Try start: /<slug> [rest...]
  const startMatch = trimmed.match(/^\/([a-z0-9][a-z0-9-]*)(?:\s+(.*))?$/i);
  if (startMatch) {
    const slug = (startMatch[1] ?? "").toLowerCase();
    if (isKnownSlug(slug) && SLUG_RE.test(slug)) {
      return {
        override: slug,
        cleanPrompt: (startMatch[2] ?? "").trim(),
        slugLocation: "start",
      };
    }
  }
  // Try end: [rest...] /<slug>
  const endMatch = trimmed.match(/^(.*?)\s+\/([a-z0-9][a-z0-9-]*)$/i);
  if (endMatch) {
    const slug = (endMatch[2] ?? "").toLowerCase();
    if (isKnownSlug(slug) && SLUG_RE.test(slug)) {
      return {
        override: slug,
        cleanPrompt: (endMatch[1] ?? "").trim(),
        slugLocation: "end",
      };
    }
  }
  return { cleanPrompt: trimmed };
}

export function resolveOverrideToModel(slug: string): string | undefined {
  return OVERRIDE_ALIASES[slug.toLowerCase()];
}
