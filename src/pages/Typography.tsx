import { useRef, useState } from "react";
import { BrandKitDialog, type BrandKitDialogInput } from "@/components/typography/BrandKitDialog";
import { LogoGallery } from "@/components/typography/LogoGallery";
import { SvgEditor, type SvgEditorHandle } from "@/components/typography/SvgEditor";
import { TextLogoControls, type TextStyle } from "@/components/typography/TextLogoControls";
import { TypographyHeader } from "@/components/typography/TypographyHeader";
import { Button } from "@/components/ui/Button";
import { type BrandKitInput, exportBrandKit } from "@/lib/brandKitCommands";
import { generateLogoVariants, type LogoStyle, type LogoVariant } from "@/lib/logoCommands";
import { vectorizeImage } from "@/lib/vectorizerCommands";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

const DEFAULT_TEXT_STYLE: TextStyle = {
  font: "Inter",
  color: "#F7F7F8",
  size: 72,
  kerning: 0,
};

export function TypographyPage() {
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState<LogoStyle>("minimalist");
  const [palette, setPalette] = useState("");
  const [busy, setBusy] = useState(false);
  const [variants, setVariants] = useState<LogoVariant[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const [textStyle, setTextStyle] = useState<TextStyle>(DEFAULT_TEXT_STYLE);
  // `logoText` drives the "Add text" button. We intentionally keep it
  // separate from `prompt` (the generation prompt) — a realistic prompt
  // like "a minimalist logo for Acme Corp" would otherwise ship as the
  // literal logo text (FU #176).
  const [logoText, setLogoText] = useState("");
  const [vectorizing, setVectorizing] = useState(false);
  // `vectorized` gates the Export button on a fresh vectorize so we never
  // ship a kit whose SVG doesn't match the selected PNG. It's reset inline
  // in `LogoGallery.onSelect` below. Teardown scenarios (SvgEditor
  // unmounts, HMR, test cleanup) can leave the flag stale against an empty
  // canvas — the empty-SVG check in `handleExport` is the backstop that
  // catches that edge case without needing an onDispose callback.
  const [vectorized, setVectorized] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const editorRef = useRef<SvgEditorHandle>(null);
  const notify = useUiStore((s) => s.notify);
  const currentProject = useProjectStore((s) => s.currentProject);

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
      setVectorized(true);
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

  async function handleExport(dialogInput: BrandKitDialogInput) {
    // Pre-validation — these cannot happen in normal flow because the Export
    // button is already disabled when either precondition is false. Surface
    // via notify and bail out so the dialog closes cleanly; the dialog's
    // role="alert" is reserved for actual backend rejections.
    if (!selectedVariant?.local_path) {
      notify({
        kind: "error",
        message: "Export failed",
        detail: "No source PNG — select a variant with a local copy first",
      });
      return;
    }
    const logoSvg = editorRef.current?.toSvgString() ?? "";
    if (!logoSvg) {
      notify({
        kind: "error",
        message: "Export failed",
        detail: "No SVG in editor — vectorize the variant first",
      });
      return;
    }
    // Backend errors propagate to the dialog, which displays them inline.
    const input: BrandKitInput = {
      logo_svg: logoSvg,
      source_png_path: selectedVariant.local_path,
      brand_name: dialogInput.brand_name,
      primary_color: dialogInput.primary_color,
      accent_color: dialogInput.accent_color,
      font: dialogInput.font,
    };
    const zipPath = await exportBrandKit(input, dialogInput.destination);
    notify({ kind: "success", message: "Brand kit exported", detail: zipPath });
  }

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      <TypographyHeader
        prompt={prompt}
        onPromptChange={setPrompt}
        style={style}
        onStyleChange={setStyle}
        palette={palette}
        onPaletteChange={setPalette}
        busy={busy}
        onGenerate={handleGenerate}
      />

      <div className="grid min-h-0 grid-cols-[1fr_18rem]">
        <LogoGallery
          variants={variants}
          selectedUrl={selectedUrl}
          onSelect={(url) => {
            // Only reset `vectorized` when the selection actually changes.
            // Re-clicking the already-selected variant shouldn't wipe a
            // successful vectorize.
            if (url !== selectedUrl) {
              setSelectedUrl(url);
              setVectorized(false);
            }
          }}
        />
        <div className="flex min-h-0 flex-col gap-3 border-neutral-dark-700 border-l p-3">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Editor
          </span>
          {selectedVariant ? (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <Button variant="primary" onClick={handleVectorize} disabled={!canVectorize}>
                  {vectorizing ? "Vectorizing…" : "Vectorize"}
                </Button>
                {/* No visible <label>: the "Logo text" placeholder is self-
                    descriptive and the 18rem panel is tight on vertical
                    space. The aria-label keeps the input accessible to
                    screen readers. */}
                <input
                  aria-label="Logo text"
                  value={logoText}
                  onChange={(e) => setLogoText(e.target.value)}
                  placeholder="Logo text"
                  className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-2xs text-neutral-dark-100"
                />
                <Button
                  variant="ghost"
                  onClick={() => {
                    // Fire-and-forget — addText returns a Promise so the
                    // font can finish loading before the Textbox renders,
                    // but the click handler doesn't need to block on it.
                    // `logoText` drives this, NOT `prompt` — a long
                    // generation prompt would otherwise ship as logo text.
                    // Error surfaces via notify to match `handleVectorize`'s
                    // error-channel pattern (not the dialog banner, which
                    // is reserved for backend-rejection paths).
                    const trimmed = logoText.trim();
                    if (trimmed) {
                      editorRef.current?.addText(trimmed, textStyle).catch((err) => {
                        notify({
                          kind: "error",
                          message: "Add text failed",
                          detail: err instanceof Error ? err.message : String(err),
                        });
                      });
                    }
                  }}
                  disabled={!logoText.trim()}
                >
                  Add text
                </Button>
                <Button
                  variant="ghost"
                  onClick={() => setExportOpen(true)}
                  disabled={!vectorized || !selectedVariant.local_path || vectorizing}
                >
                  Export brand kit
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
              <TextLogoControls
                value={textStyle}
                onChange={(next) => {
                  setTextStyle(next);
                  // Fire-and-forget — updateText no-ops if nothing is
                  // selected, so this is safe even before Add text is
                  // clicked.
                  void editorRef.current?.updateText(next);
                }}
              />
            </>
          ) : (
            <div className="mt-2 font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
              No selection
            </div>
          )}
        </div>
      </div>
      <BrandKitDialog
        open={exportOpen}
        onClose={() => setExportOpen(false)}
        onSubmit={handleExport}
        defaultDestination={currentProject ? `${currentProject.path}/exports` : ""}
      />
    </div>
  );
}
