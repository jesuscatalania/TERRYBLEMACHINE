export type IsoPresetName = "none" | "room" | "city" | "product";

export interface IsoCamera {
  position: [number, number, number];
  fov: number;
}

/** Returns the camera position + fov for a given preset, or null for "none"
 * (caller keeps current camera). */
export function cameraForIso(preset: IsoPresetName): IsoCamera | null {
  switch (preset) {
    case "room":
      return { position: [6, 5, 6], fov: 35 };
    case "city":
      return { position: [12, 10, 12], fov: 30 };
    case "product":
      return { position: [3, 2.5, 3], fov: 40 };
    default:
      return null;
  }
}
