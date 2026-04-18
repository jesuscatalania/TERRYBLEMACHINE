import { useGLTF } from "@react-three/drei";
import { convertFileSrc } from "@tauri-apps/api/core";

export interface GltfModelProps {
  /**
   * Local filesystem path to the cached GLB, as returned by the
   * `generate_mesh_from_*` Tauri commands. When present, this is preferred
   * over `remoteUrl` because it avoids CORS and first-paint latency.
   */
  localPath: string | null;
  /**
   * Remote provider URL. Used as a fallback when `localPath` is `null`
   * (e.g. the backend download failed).
   */
  remoteUrl: string;
}

/**
 * Loads a GLB/GLTF scene via drei's `useGLTF`. Local cache paths are piped
 * through Tauri's `convertFileSrc` so the webview can fetch via the
 * `asset://` protocol.
 */
export function GltfModel({ localPath, remoteUrl }: GltfModelProps) {
  const src = localPath ? convertFileSrc(localPath) : remoteUrl;
  const { scene } = useGLTF(src);
  return <primitive object={scene} />;
}
