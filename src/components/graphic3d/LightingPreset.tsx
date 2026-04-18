import { Environment } from "@react-three/drei";

export type LightingName = "studio" | "outdoor" | "dramatic";

/** Maps a lighting preset to its drei Environment preset string. Pure helper
 * exported for tests; the in-canvas component uses it internally. */
export function presetEnvFor(name: LightingName): "studio" | "sunset" | "night" {
  if (name === "studio") return "studio";
  if (name === "outdoor") return "sunset";
  return "night";
}

interface PresetProps {
  name: LightingName;
}

export function LightingPreset({ name }: PresetProps) {
  if (name === "studio") {
    return (
      <>
        <ambientLight intensity={0.4} />
        <directionalLight position={[3, 5, 3]} intensity={1.2} />
        <directionalLight position={[-3, 2, -3]} intensity={0.6} color="#ffe6cc" />
        <Environment preset={presetEnvFor(name)} background={false} />
      </>
    );
  }
  if (name === "outdoor") {
    return (
      <>
        <ambientLight intensity={0.6} />
        <directionalLight position={[5, 10, 5]} intensity={1.6} color="#fff4d6" />
        <Environment preset={presetEnvFor(name)} background={false} />
      </>
    );
  }
  // dramatic
  return (
    <>
      <ambientLight intensity={0.1} />
      {/* Note: shadow maps are not enabled on the Canvas. Re-add castShadow when shadows ship. */}
      <spotLight position={[5, 8, 3]} angle={0.4} intensity={3} color="#ffd599" />
      <Environment preset={presetEnvFor(name)} background={false} />
    </>
  );
}
