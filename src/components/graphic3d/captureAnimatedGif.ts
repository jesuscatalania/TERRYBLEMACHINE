import GIF from "gif.js";
import type { Camera, Scene, WebGLRenderer } from "three";

export interface AnimatedGifOptions {
  /** Number of frames in the orbit. Clamped to >=1. Default 30. */
  frames?: number;
  /** Delay between frames in ms. Clamped to >=10. Default 100. */
  delayMs?: number;
  /** Orbit radius. If omitted, derived from the camera's current XZ distance to the origin. */
  radius?: number;
}

interface CameraPositionLike {
  x: number;
  y: number;
  z: number;
  set: (x: number, y: number, z: number) => void;
}

interface CameraLike {
  position: CameraPositionLike;
  lookAt?: (x: number, y: number, z: number) => void;
}

/**
 * Orbit the camera around the origin over `frames` discrete positions, render
 * each frame via `gl.render(scene, camera)`, then compose the captured frames
 * into a GIF via gif.js. Returns a data-URL of the animated GIF, or an empty
 * string if composition failed (allowing callers to fall back gracefully
 * rather than deal with rejected promises — consistent with FabricCanvas'
 * `toGif` in Phase 4).
 *
 * The camera's original position is captured at the start and restored on
 * every exit path (happy, sync error, gif.js abort) so the user's view is
 * never stuck at frame N-1.
 */
export function captureAnimatedGif(
  gl: WebGLRenderer,
  scene: Scene,
  camera: Camera,
  options: AnimatedGifOptions = {},
): Promise<string> {
  const frames = Math.max(1, options.frames ?? 30);
  const delayMs = Math.max(10, options.delayMs ?? 100);
  const canvasEl = gl.domElement;
  const cam = camera as unknown as CameraLike;
  // Derive radius from current camera distance to origin on the XZ plane.
  // If the camera is sitting at the origin (unlikely but possible for a
  // fresh/orthographic scene), fall back to 6 so we still produce motion.
  const derivedRadius = Math.hypot(cam.position.x, cam.position.z);
  const radius = options.radius ?? (derivedRadius > 0 ? derivedRadius : 6);
  const original = {
    x: cam.position.x,
    y: cam.position.y,
    z: cam.position.z,
  };

  const restore = () => {
    cam.position.set(original.x, original.y, original.z);
    cam.lookAt?.(0, 0, 0);
    try {
      gl.render(scene, camera);
    } catch {
      // Best-effort restore — if the renderer is already gone, swallow.
    }
  };

  return new Promise<string>((resolve) => {
    const gif = new GIF({
      workers: 2,
      quality: 10,
      width: canvasEl.width,
      height: canvasEl.height,
      workerScript: "/gif.worker.js",
    });

    try {
      for (let i = 0; i < frames; i++) {
        const angle = (i / frames) * Math.PI * 2;
        cam.position.set(Math.cos(angle) * radius, original.y, Math.sin(angle) * radius);
        cam.lookAt?.(0, 0, 0);
        gl.render(scene, camera);
        // gif.js accepts HTMLCanvasElement — copy:true snapshots the bitmap
        // so the next `gl.render` won't overwrite what we just queued.
        gif.addFrame(canvasEl, { copy: true, delay: delayMs });
      }
    } catch (err) {
      console.error("captureAnimatedGif: frame capture failed", err);
      restore();
      resolve("");
      return;
    }

    gif.on("finished", (blob: Blob) => {
      restore();
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => resolve("");
      reader.readAsDataURL(blob);
    });
    // gif.js fires "abort" when a worker aborts or render() is cancelled;
    // without this handler the promise would hang forever.
    gif.on("abort", () => {
      restore();
      resolve("");
    });

    try {
      gif.render();
    } catch (err) {
      console.error("captureAnimatedGif: gif.render() threw synchronously", err);
      restore();
      resolve("");
    }
  });
}
