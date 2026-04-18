import { Bloom, EffectComposer, SSAO } from "@react-three/postprocessing";

export interface PostProcessingProps {
  bloom?: boolean;
  ssao?: boolean;
}

/**
 * Wraps optional Bloom + SSAO effects in an EffectComposer. Returns null
 * (short-circuit) when both flags are off so the composer only mounts when
 * at least one effect is active. Must be rendered INSIDE the R3F <Canvas>
 * because EffectComposer consumes the fiber context.
 */
export function PostProcessing({ bloom, ssao }: PostProcessingProps) {
  if (!bloom && !ssao) return null;
  if (bloom && ssao) {
    return (
      <EffectComposer>
        <Bloom intensity={0.6} luminanceThreshold={0.8} />
        <SSAO samples={16} radius={0.3} intensity={20} />
      </EffectComposer>
    );
  }
  if (bloom) {
    return (
      <EffectComposer>
        <Bloom intensity={0.6} luminanceThreshold={0.8} />
      </EffectComposer>
    );
  }
  return (
    <EffectComposer>
      <SSAO samples={16} radius={0.3} intensity={20} />
    </EffectComposer>
  );
}
