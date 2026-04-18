export type CameraMode = "perspective" | "orthographic";

interface Props {
  mode: CameraMode;
  onModeChange: (mode: CameraMode) => void;
}

export function CameraControls({ mode, onModeChange }: Props) {
  return (
    <div className="flex flex-col gap-1">
      <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        Camera
      </span>
      <select
        value={mode}
        onChange={(e) => onModeChange(e.target.value as CameraMode)}
        className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-xs text-neutral-dark-100"
      >
        <option value="perspective">Perspective</option>
        <option value="orthographic">Orthographic</option>
      </select>
    </div>
  );
}
