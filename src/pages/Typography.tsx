import { Sparkles } from "lucide-react";
import { useState } from "react";
import { LogoGallery } from "@/components/typography/LogoGallery";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { generateLogoVariants, type LogoStyle, type LogoVariant } from "@/lib/logoCommands";
import { useUiStore } from "@/stores/uiStore";

export function TypographyPage() {
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState<LogoStyle>("minimalist");
  const [palette, setPalette] = useState("");
  const [busy, setBusy] = useState(false);
  const [variants, setVariants] = useState<LogoVariant[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const notify = useUiStore((s) => s.notify);

  async function handleGenerate() {
    if (!prompt.trim()) return;
    setBusy(true);
    try {
      const results = await generateLogoVariants({
        prompt: prompt.trim(),
        style,
        count: 6,
        palette: palette.trim() || undefined,
        module: "typography",
      });
      setVariants(results);
      notify({ kind: "success", message: `Generated ${results.length} variants` });
    } catch (err) {
      notify({
        kind: "error",
        message: "Logo generation failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
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
              onValueChange={setPrompt}
            />
          </div>
          <label className="flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Style
            </span>
            <select
              aria-label="Logo style"
              value={style}
              onChange={(e) => setStyle(e.target.value as LogoStyle)}
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
              onValueChange={setPalette}
            />
          </div>
          <Button variant="primary" onClick={handleGenerate} disabled={!prompt.trim() || busy}>
            <Sparkles className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            {busy ? "Generating…" : "Generate 6 variants"}
          </Button>
        </div>
      </div>

      <div className="grid min-h-0 grid-cols-[1fr_18rem]">
        <LogoGallery variants={variants} selectedUrl={selectedUrl} onSelect={setSelectedUrl} />
        <div className="flex flex-col border-neutral-dark-700 border-l p-3">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Editor
          </span>
          {selectedUrl ? (
            <div className="mt-2 break-all font-mono text-2xs text-neutral-dark-500">
              Selected · {selectedUrl.slice(-24)}
            </div>
          ) : (
            <div className="mt-2 font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
              No selection
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
