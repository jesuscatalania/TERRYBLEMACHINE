import { useRef, useState } from "react";
import { BrandKitDialog, type BrandKitDialogInput } from "@/components/typography/BrandKitDialog";
import { LogoGallery } from "@/components/typography/LogoGallery";
import { SvgEditor, type SvgEditorHandle } from "@/components/typography/SvgEditor";
import { TextLogoControls, type TextStyle } from "@/components/typography/TextLogoControls";
import { TypographyHeader } from "@/components/typography/TypographyHeader";
import { Button } from "@/components/ui/Button";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";
import { ToolDropdown } from "@/components/ui/ToolDropdown";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";
import { type BrandKitInput, exportBrandKit } from "@/lib/brandKitCommands";
import { generateLogoVariants, type LogoStyle, type LogoVariant } from "@/lib/logoCommands";
import { parseOverride, resolveOverrideToModel } from "@/lib/promptOverride";
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
  // "auto" = let the router strategy pick. Otherwise a PascalCase Model
  // enum string from the ToolDropdown (or resolved from a `/tool` slug).
  const [model, setModel] = useState<string>("auto");
  const optimize = useOptimizePrompt({
    taskKind: "Logo",
    value: prompt,
    setValue: setPrompt,
  });
  const editorRef = useRef<SvgEditorHandle>(null);
  // Request-id token for vectorize. User clicks variant A → Vectorize →
  // clicks variant B mid-flight → second Vectorize; whichever returns last
  // wins. Without this guard, a late reply from A paints the editor even
  // though `selectedVariant === B`, shipping inconsistent brand kit.
  const vectorizeRequestRef = useRef(0);
  const notify = useUiStore((s) => s.notify);
  const currentProject = useProjectStore((s) => s.currentProject);

  const selectedVariant = selectedUrl
    ? (variants.find((v) => v.url === selectedUrl) ?? null)
    : null;

  async function handleGenerate() {
    if (!prompt.trim()) return;
    setBusy(true);
    try {
      // Design-spec order (see phase-9 claude-bridge spec):
      //   1) parseOverride on the RAW prompt → strip slug
      //   2) optimize the cleanPrompt (never the slug — Claude must
      //      not see `/ideogram` or it may mangle the override)
      //   3) re-attach the slug to the input for display
      //   4) dispatch with the optimized clean text + resolved model
      const parsed = parseOverride(prompt);
      let textForDispatch = parsed.cleanPrompt || prompt;

      if (optimize.enabled && parsed.cleanPrompt) {
        const optimized = await optimize.optimize(parsed.cleanPrompt);
        if (optimized !== undefined) {
          textForDispatch = optimized;
          if (parsed.override && parsed.slugLocation) {
            const reattached =
              parsed.slugLocation === "start"
                ? `/${parsed.override} ${optimized}`
                : `${optimized} /${parsed.override}`;
            setPrompt(reattached);
          }
        }
      }

      const overrideModel = parsed.override ? resolveOverrideToModel(parsed.override) : undefined;
      const finalModel = overrideModel ?? (model === "auto" ? undefined : model);

      const results = await generateLogoVariants({
        prompt: textForDispatch,
        style,
        count: 6,
        palette: palette.trim() || undefined,
        module: "typography",
        model_override: finalModel,
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
    const myRequest = ++vectorizeRequestRef.current;
    try {
      const result = await vectorizeImage({
        image_path: selectedVariant.local_path,
      });
      // Stale-result guard: if another vectorize started after us (or the
      // selection changed, which also bumps the counter), drop this result.
      if (myRequest !== vectorizeRequestRef.current) return;
      await editorRef.current?.loadSvg(result.svg, result.width, result.height);
      setVectorized(true);
      notify({ kind: "success", message: "Vectorized logo" });
    } catch (err) {
      if (myRequest !== vectorizeRequestRef.current) return;
      notify({
        kind: "error",
        message: "Vectorize failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      // Only the winning request flips the vectorizing flag off — a stale
      // call returning before the live one must not clear the spinner.
      if (myRequest === vectorizeRequestRef.current) {
        setVectorizing(false);
      }
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
    // Show an in-flight progress toast around the export. Real backend
    // progress events don't exist yet — the 0%-bar is a "this is happening"
    // cue. `dismissNotification` on both success and error ensures the
    // in-flight toast never lingers past the terminal notification.
    const progressId = notify({
      kind: "info",
      message: "Building brand kit",
      progress: { current: 0, total: 12 },
    });
    try {
      const zipPath = await exportBrandKit(input, dialogInput.destination);
      useUiStore.getState().dismissNotification(progressId);
      notify({ kind: "success", message: "Brand kit exported", detail: zipPath });
    } catch (err) {
      useUiStore.getState().dismissNotification(progressId);
      throw err;
    }
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
        toolsSlot={
          <>
            <ToolDropdown taskKind="Logo" value={model} onChange={setModel} />
            <OptimizeToggle
              enabled={optimize.enabled}
              onToggle={optimize.setEnabled}
              busy={optimize.busy}
              canUndo={optimize.canUndo}
              onUndo={optimize.undo}
            />
          </>
        }
      />

      <div className="grid min-h-0 grid-cols-[1fr_18rem]">
        <LogoGallery
          variants={variants}
          selectedUrl={selectedUrl}
          busy={busy}
          onSelect={(url) => {
            // Only reset `vectorized` when the selection actually changes.
            // Re-clicking the already-selected variant shouldn't wipe a
            // successful vectorize.
            if (url !== selectedUrl) {
              setSelectedUrl(url);
              setVectorized(false);
              // Invalidate any in-flight vectorize for the OLD variant —
              // its late reply must not paint the editor now that
              // `selectedVariant` points elsewhere.
              vectorizeRequestRef.current++;
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
                <LoadingButton
                  variant="primary"
                  onClick={handleVectorize}
                  disabled={!selectedVariant?.local_path}
                  loading={vectorizing}
                >
                  Vectorize
                </LoadingButton>
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
