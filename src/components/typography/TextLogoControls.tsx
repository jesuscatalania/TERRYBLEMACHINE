import { GOOGLE_FONTS, type GoogleFont, injectGoogleFont } from "@/lib/googleFonts";

/**
 * Shape of the text style patch the typography page owns.
 *
 * `kerning` is per-letter spacing in px, applied to Fabric's Textbox via
 * `charSpacing` (1/1000 em).
 *
 * `font` is narrowed to `GoogleFont` (FU #175) so only families that are
 * actually in the curated Google Fonts list can flow into
 * `injectGoogleFont` — keeps an invalid family from silently 404'ing the
 * stylesheet at the SvgEditor boundary.
 */
export interface TextStyle {
  font: GoogleFont;
  color: string;
  size: number;
  kerning: number;
}

export interface TextLogoControlsProps {
  value: TextStyle;
  onChange: (next: TextStyle) => void;
}

export function TextLogoControls({ value, onChange }: TextLogoControlsProps) {
  return (
    <div className="flex flex-col gap-2">
      <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        Text
      </span>
      <select
        aria-label="Font"
        value={value.font}
        onChange={(e) => {
          const next = e.target.value as GoogleFont;
          // Fire-and-forget font load; onChange propagates immediately so
          // the parent updates its state without waiting on the network.
          void injectGoogleFont(next);
          onChange({ ...value, font: next });
        }}
        className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-2xs text-neutral-dark-100"
      >
        {GOOGLE_FONTS.map((f) => (
          <option key={f} value={f}>
            {f}
          </option>
        ))}
      </select>
      <label className="flex items-center gap-2 text-2xs text-neutral-dark-300">
        Color
        <input
          aria-label="Color"
          type="color"
          value={value.color}
          onChange={(e) => onChange({ ...value, color: e.target.value })}
          className="h-6 w-10 cursor-pointer"
        />
      </label>
      <label className="flex flex-col gap-1 text-2xs text-neutral-dark-300">
        Size: {value.size}px
        <input
          aria-label="Size"
          type="range"
          min={12}
          max={240}
          step={1}
          value={value.size}
          onChange={(e) => onChange({ ...value, size: Number(e.target.value) })}
          className="accent-accent-500"
        />
      </label>
      <label className="flex flex-col gap-1 text-2xs text-neutral-dark-300">
        Kerning: {value.kerning.toFixed(1)}
        <input
          aria-label="Kerning"
          type="range"
          min={-5}
          max={30}
          step={0.5}
          value={value.kerning}
          onChange={(e) => onChange({ ...value, kerning: Number(e.target.value) })}
          className="accent-accent-500"
        />
      </label>
    </div>
  );
}
