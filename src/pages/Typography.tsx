import { Sparkles } from "lucide-react";
import { useRef, useState } from "react";
import { LogoGallery } from "@/components/typography/LogoGallery";
import { SvgEditor, type SvgEditorHandle } from "@/components/typography/SvgEditor";
import { TextLogoControls, type TextStyle } from "@/components/typography/TextLogoControls";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { generateLogoVariants, type LogoStyle, type LogoVariant } from "@/lib/logoCommands";
import { vectorizeImage } from "@/lib/vectorizerCommands";
import { useUiStore } from "@/stores/uiStore";

const DEFAULT_TEXT_STYLE: TextStyle = {
  font: "Inter",
  color: "#F7F7F8",
  size: 72,
  kerning: 0,
  tracking: 0,
};

export function TypographyPage() {
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState<LogoStyle>("minimalist");
  const [palette, setPalette] = useState("");
  const [busy, setBusy] = useState(false);
  const [variants, setVariants] = useState<LogoVariant[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const [textStyle, setTextStyle] = useState<TextStyle>(DEFAULT_TEXT_STYLE);
  const [vectorizing, setVectorizing] = useState(false);
  const editorRef = useRef<SvgEditorHandle>(null);
  const notify = useUiStore((s) => s.notify);

  const selectedVariant = selectedUrl
    ? (variants.find((v) => v.url === selectedUrl) ?? null)
    : null;
  const canVectorize = Boolean(selectedVariant?.local_path) && !vectorizing;

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

  async function handleVectorize() {
    if (!selectedVariant?.local_path) return;
    setVectorizing(true);
    try {
      const result = await vectorizeImage({
        image_path: selectedVariant.local_path,
      });
      await editorRef.current?.loadSvg(result.svg, result.width, result.height);
      notify({ kind: "success", message: "Vectorized logo" });
    } catch (err) {
      notify({
        kind: "error",
        message: "Vectorize failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setVectorizing(false);
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
        <div className="flex min-h-0 flex-col gap-3 border-neutral-dark-700 border-l p-3">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Editor
          </span>
          {selectedVariant ? (
            <>
              <div className="flex items-center gap-2">
                <Button variant="primary" onClick={handleVectorize} disabled={!canVectorize}>
                  {vectorizing ? "Vectorizing…" : "Vectorize"}
                </Button>
                {!selectedVariant.local_path && (
                  <span className="font-mono text-2xs text-neutral-dark-500">
                    No local copy yet
                  </span>
                )}
              </div>
              <div className="min-h-0 flex-1">
                <SvgEditor ref={editorRef} />
              </div>
              <TextLogoControls value={textStyle} onChange={setTextStyle} />
            </>
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
