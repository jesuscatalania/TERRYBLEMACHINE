/**
 * Static catalog of API providers exposed by the Settings modal.
 *
 * The `id` field MUST match the `KEYCHAIN_SERVICE` constant declared in the
 * corresponding `src-tauri/src/api_clients/<provider>.rs` module so the
 * backend resolves the right key on read.
 */

export type ProviderPlan = "Subscription (Pro/Max)" | "Pay-per-Use";

export type ProviderTransport = "auto" | "api" | "cli";

export interface ProviderDef {
  id: string;
  label: string;
  plan: ProviderPlan;
  helpUrl: string;
  /** Short hint rendered next to the input. Optional. */
  hint?: string;
  /**
   * Optional set of transport channels the provider can be reached over.
   * When present, the Settings row renders a transport selector. Currently
   * only Claude supports this (auto | api | cli).
   */
  transports?: readonly ProviderTransport[];
}

export const PROVIDERS: readonly ProviderDef[] = [
  {
    id: "claude",
    label: "Anthropic Claude",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://console.anthropic.com/settings/keys",
    transports: ["auto", "api", "cli"] as const,
  },
  {
    id: "kling",
    label: "Kling AI Video (direct — optional, fal handles Kling by default)",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://app.klingai.com/global/",
  },
  {
    id: "runway",
    label: "Runway Gen-3",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://app.runwayml.com/account",
  },
  {
    id: "higgsfield",
    label: "Higgsfield Video",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://higgsfield.ai/",
  },
  {
    id: "shotstack",
    label: "Shotstack (timeline assembly)",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://shotstack.io/dashboard/",
  },
  {
    id: "ideogram",
    label: "Ideogram (logos / typography)",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://ideogram.ai/manage-api",
  },
  {
    id: "meshy",
    label: "Meshy 3D",
    plan: "Subscription (Pro/Max)",
    helpUrl: "https://www.meshy.ai/api",
  },
  {
    id: "fal",
    label: "fal.ai (images + Kling video)",
    plan: "Pay-per-Use",
    helpUrl: "https://fal.ai/dashboard/keys",
  },
  {
    id: "replicate",
    label: "Replicate (specialty models)",
    plan: "Pay-per-Use",
    helpUrl: "https://replicate.com/account/api-tokens",
  },
] as const;
