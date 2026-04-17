import { useState } from "react";
import { Dropdown } from "@/components/ui/Dropdown";
import { Input } from "@/components/ui/Input";
import { GOOGLE_FONTS, type GoogleFont, injectGoogleFont } from "@/lib/googleFonts";

export interface TextControlsProps {
  initialFont?: string;
  initialColor?: string;
  initialSize?: number;
  onChange: (patch: { font?: string; color?: string; size?: number }) => void;
}

export function TextControls({
  initialFont = "Inter",
  initialColor = "#F7F7F8",
  initialSize = 48,
  onChange,
}: TextControlsProps) {
  // Runtime guard against an off-list initialFont — if the Textbox currently
  // carries a family we don't know how to inject (e.g. "Comic Sans MS"),
  // fall back to Inter rather than silently advertising a bogus selection.
  // See FU #105 (c).
  const [font, setFont] = useState<GoogleFont>(() =>
    (GOOGLE_FONTS as readonly string[]).includes(initialFont)
      ? (initialFont as GoogleFont)
      : "Inter",
  );
  const [color, setColor] = useState(initialColor);
  const [size, setSize] = useState(initialSize);

  // Dropdown.onChange signature is (value: string) => void — we still want
  // to await font-load before pushing state + notifying the parent, so we
  // wrap the async work inside and fire-and-await it ourselves.
  const handleFontChange = (v: string) => {
    const next = v as GoogleFont;
    // Intentionally not awaited by Dropdown — we use an IIFE so we can await
    // injectGoogleFont before committing state and propagating the change.
    void (async () => {
      await injectGoogleFont(next);
      setFont(next);
      onChange({ font: next });
    })();
  };

  return (
    <div className="flex flex-col gap-2">
      <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        Text
      </span>

      <div className="flex flex-col gap-1.5">
        <label
          htmlFor="text-font"
          className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
        >
          Font
        </label>
        <Dropdown
          id="text-font"
          value={font}
          searchable
          onChange={handleFontChange}
          options={GOOGLE_FONTS.map((f) => ({ value: f, label: f }))}
        />
      </div>

      <div className="flex items-center gap-2">
        <label
          htmlFor="text-color"
          className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
        >
          Color
        </label>
        <input
          id="text-color"
          type="color"
          value={color}
          onChange={(e) => {
            setColor(e.currentTarget.value);
            onChange({ color: e.currentTarget.value });
          }}
          className="h-6 w-12 cursor-pointer rounded-xs border border-neutral-dark-700 bg-transparent"
        />
      </div>

      <Input
        type="number"
        label="Size"
        id="text-size"
        value={String(size)}
        onValueChange={(v) => {
          const n = Number(v);
          if (Number.isFinite(n) && n > 0) {
            setSize(n);
            onChange({ size: n });
          }
        }}
      />
    </div>
  );
}
