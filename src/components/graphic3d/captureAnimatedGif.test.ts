import { beforeEach, describe, expect, it, vi } from "vitest";

const addFrameSpy = vi.fn();
const renderSpy = vi.fn();

interface GifInstance {
  addFrame: typeof addFrameSpy;
  render: typeof renderSpy;
  handlers: Map<string, (...args: unknown[]) => void>;
  on: (event: string, cb: (...args: unknown[]) => void) => void;
}

const gifInstances: GifInstance[] = [];

vi.mock("gif.js", () => ({
  default: class {
    addFrame = addFrameSpy;
    render = renderSpy;
    handlers = new Map<string, (...args: unknown[]) => void>();
    constructor() {
      gifInstances.push(this as unknown as GifInstance);
    }
    on(event: string, cb: (...args: unknown[]) => void) {
      this.handlers.set(event, cb);
    }
  },
}));

// FileReader.readAsDataURL uses a real worker path in jsdom which hangs for
// empty blobs. Stub with a synchronous fake that fires onload on the
// microtask queue so the test promise can resolve.
vi.stubGlobal(
  "FileReader",
  class {
    result: string | null = null;
    onload: (() => void) | null = null;
    onerror: (() => void) | null = null;
    readAsDataURL(_blob: Blob) {
      this.result = "data:image/gif;base64,";
      setTimeout(() => this.onload?.(), 0);
    }
  },
);

import type { Camera, Scene, WebGLRenderer } from "three";
import { captureAnimatedGif } from "@/components/graphic3d/captureAnimatedGif";

interface FakeScene {
  [k: string]: unknown;
}

interface FakeCameraPosition {
  x: number;
  y: number;
  z: number;
  set: (x: number, y: number, z: number) => void;
}

interface FakeCamera {
  position: FakeCameraPosition;
  lookAt: (x: number, y: number, z: number) => void;
}

interface FakeGl {
  domElement: HTMLCanvasElement;
  render: (scene: Scene, camera: Camera) => void;
}

function makeFakeScene(): { gl: FakeGl; scene: FakeScene; camera: FakeCamera } {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 48;
  const setSpy = vi.fn(function (this: FakeCameraPosition, x: number, y: number, z: number) {
    this.x = x;
    this.y = y;
    this.z = z;
  });
  const camera: FakeCamera = {
    position: { x: 4, y: 3, z: 4, set: setSpy },
    lookAt: vi.fn(),
  };
  return {
    gl: { domElement: canvas, render: vi.fn() },
    scene: {},
    camera,
  };
}

function finishGifWith(blob: Blob) {
  const inst = gifInstances[gifInstances.length - 1];
  inst.handlers.get("finished")?.(blob);
}

describe("captureAnimatedGif", () => {
  beforeEach(() => {
    addFrameSpy.mockClear();
    renderSpy.mockClear();
    gifInstances.length = 0;
  });

  it("adds `frames` frames to the gif", async () => {
    const { gl, scene, camera } = makeFakeScene();
    const promise = captureAnimatedGif(
      gl as unknown as WebGLRenderer,
      scene as unknown as Scene,
      camera as unknown as Camera,
      { frames: 5, delayMs: 40 },
    );
    finishGifWith(new Blob([], { type: "image/gif" }));
    const result = await promise;
    expect(addFrameSpy).toHaveBeenCalledTimes(5);
    expect(renderSpy).toHaveBeenCalledOnce();
    expect(result).toBe("data:image/gif;base64,");
  });

  it("uses default 30 frames when `frames` is not set", async () => {
    const { gl, scene, camera } = makeFakeScene();
    const promise = captureAnimatedGif(
      gl as unknown as WebGLRenderer,
      scene as unknown as Scene,
      camera as unknown as Camera,
    );
    finishGifWith(new Blob([], { type: "image/gif" }));
    await promise;
    expect(addFrameSpy).toHaveBeenCalledTimes(30);
  });

  it("clamps frames to >=1 and delayMs to >=10", async () => {
    const { gl, scene, camera } = makeFakeScene();
    const promise = captureAnimatedGif(
      gl as unknown as WebGLRenderer,
      scene as unknown as Scene,
      camera as unknown as Camera,
      { frames: 0, delayMs: 1 },
    );
    finishGifWith(new Blob([], { type: "image/gif" }));
    await promise;
    expect(addFrameSpy).toHaveBeenCalledTimes(1);
    expect(addFrameSpy).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ delay: 10 }),
    );
  });

  it("restores original camera position after render", async () => {
    const { gl, scene, camera } = makeFakeScene();
    camera.position.x = 10;
    camera.position.y = 2;
    camera.position.z = -4;
    const promise = captureAnimatedGif(
      gl as unknown as WebGLRenderer,
      scene as unknown as Scene,
      camera as unknown as Camera,
      { frames: 3 },
    );
    finishGifWith(new Blob([], { type: "image/gif" }));
    await promise;
    // Last call to set() should be the restore-call with the original xyz.
    expect(camera.position.set).toHaveBeenLastCalledWith(10, 2, -4);
  });

  it("calls gl.render for every frame plus the restore render", async () => {
    const { gl, scene, camera } = makeFakeScene();
    const promise = captureAnimatedGif(
      gl as unknown as WebGLRenderer,
      scene as unknown as Scene,
      camera as unknown as Camera,
      { frames: 4 },
    );
    finishGifWith(new Blob([], { type: "image/gif" }));
    await promise;
    // 4 orbit frames + 1 restore
    expect(gl.render).toHaveBeenCalledTimes(5);
  });

  it("resolves empty string on gif abort and still restores camera", async () => {
    const { gl, scene, camera } = makeFakeScene();
    camera.position.x = 7;
    camera.position.y = 1;
    camera.position.z = 2;
    const promise = captureAnimatedGif(
      gl as unknown as WebGLRenderer,
      scene as unknown as Scene,
      camera as unknown as Camera,
      { frames: 2 },
    );
    const inst = gifInstances[gifInstances.length - 1];
    inst.handlers.get("abort")?.();
    const result = await promise;
    expect(result).toBe("");
    expect(camera.position.set).toHaveBeenLastCalledWith(7, 1, 2);
  });
});
