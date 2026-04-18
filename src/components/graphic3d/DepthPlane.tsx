import { useLoader } from "@react-three/fiber";
import { TextureLoader } from "three";

export interface DepthPlaneProps {
  /** URL of the color (albedo) image. Mapped onto the plane as `map`. */
  imageUrl: string;
  /** URL of the single-channel depth PNG. Mapped as `displacementMap`. */
  depthUrl: string;
  /** Vertical displacement scale (world units). Defaults to `0.5`. */
  displacementScale?: number;
  /** Plane width in world units. Defaults to `4`. */
  width?: number;
  /** Plane height in world units. Defaults to `3`. */
  height?: number;
  /** Subdivisions per axis — more segments = smoother relief. Defaults to `128`. */
  segments?: number;
}

/**
 * A Three.js plane textured with a color image and displaced by a depth map.
 *
 * The mesh is rotated `-π/2` around X so it sits as a ground plane under the
 * default camera (`[4, 3, 4]` looking at origin), matching the visual
 * framing used elsewhere in Graphic3D.
 *
 * Textures are loaded asynchronously via R3F's `useLoader` — callers should
 * wrap this component in `<Suspense>` if they want to show a fallback; the
 * parent `<Canvas>` already provides an implicit Suspense boundary.
 */
export function DepthPlane({
  imageUrl,
  depthUrl,
  displacementScale = 0.5,
  width = 4,
  height = 3,
  segments = 128,
}: DepthPlaneProps) {
  const [colorMap, depthMap] = useLoader(TextureLoader, [imageUrl, depthUrl]);
  return (
    <mesh rotation={[-Math.PI / 2, 0, 0]}>
      <planeGeometry args={[width, height, segments, segments]} />
      <meshStandardMaterial
        map={colorMap}
        displacementMap={depthMap}
        displacementScale={displacementScale}
      />
    </mesh>
  );
}
