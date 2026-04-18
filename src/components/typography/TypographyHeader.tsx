import { Sparkles } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import type { LogoStyle } from "@/lib/logoCommands";

/**
 * Top-of-page controls for the Typography module: MOD—05 tag plus the
 * prompt / style / palette / Generate row. Pure controlled component —
 * it receives every value + every `onChange` via props and does no
 * store/IPC work of its own (all state + async flows live in
 * `TypographyPage`, which owns the `handleGenerate` IPC call).
 */
export interface TypographyHeaderProps {
  prompt: string;
  onPromptChange: (next: string) => void;
  style: LogoStyle;
  onStyleChange: (next: LogoStyle) => void;
  palette: string;
  onPaletteChange: (next: string) => void;
  busy: boolean;
  onGenerate: () => void;
}

export function TypographyHeader({
  prompt,
  onPromptChange,
  style,
  onStyleChange,
  palette,
  onPaletteChange,
  busy,
  onGenerate,
}: TypographyHeaderProps) {
  return (
    <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
      <div className="flex items-center gap-2">
        <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
          MOD—05 · TYPE & LOGO
        </span>
      </div>
      <div className="flex items-end gap-2">
        <div className="flex-1">
          <Input
            label="Describe the logo"
            id="logo-prompt"
            placeholder={'"TERRYBLEMACHINE" — AI design tool, bold mark'}
            value={prompt}
            onValueChange={onPromptChange}
          />
        </div>
        <label className="flex flex-col gap-1">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Style
          </span>
          <select
            aria-label="Logo style"
            value={style}
            onChange={(e) => onStyleChange(e.target.value as LogoStyle)}
            className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-neutral-dark-100 text-xs"
          >
            <option value="minimalist">Minimalist</option>
            <option value="wordmark">Wordmark</option>
            <option value="emblem">Emblem</option>
            <option value="mascot">Mascot</option>
          </select>
        </label>
        <div className="w-48">
          <Input
            label="Palette"
            id="logo-palette"
            placeholder="monochrome / warm / sunset"
            value={palette}
            onValueChange={onPaletteChange}
          />
        </div>
        <Button variant="primary" onClick={onGenerate} disabled={!prompt.trim() || busy}>
          <Sparkles className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          {busy ? "Generating…" : "Generate 6 variants"}
        </Button>
      </div>
    </div>
  );
}
