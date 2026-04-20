import { ChevronDown, ChevronRight, ExternalLink, Laptop, Smartphone, Tablet } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input, Textarea } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";
import { Tabs } from "@/components/ui/Tabs";
import { ToolDropdown } from "@/components/ui/ToolDropdown";
import { CodeEditor } from "@/components/website/CodeEditor";
import { DevicePreview, type DeviceSize } from "@/components/website/DevicePreview";
import {
  WebsiteExportDialog,
  type WebsiteExportSettings,
} from "@/components/website/WebsiteExportDialog";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";
import { formatError } from "@/lib/formatError";
import { projectsRoot } from "@/lib/projectCommands";
import { parseOverride, resolveOverrideToModel } from "@/lib/promptOverride";
import {
  type AnalysisResult,
  analyzeUrl,
  exportWebsite,
  type GeneratedFile,
  type GeneratedProject,
  generateWebsite,
  modifyCodeSelection,
  openInBrowser,
  refineWebsite,
  type Template,
} from "@/lib/websiteCommands";
import { useAppStore } from "@/stores/appStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

const TEMPLATES: { id: Template; label: string }[] = [
  { id: "landing-page", label: "Landing" },
  { id: "portfolio", label: "Portfolio" },
  { id: "blog", label: "Blog" },
  { id: "dashboard", label: "Dashboard" },
  { id: "ecommerce", label: "E-Commerce" },
  { id: "custom", label: "Custom" },
];

export function WebsiteBuilderPage() {
  const [prompt, setPrompt] = useState("");
  const [template, setTemplate] = useState<Template>("landing-page");
  const [project, setProject] = useState<GeneratedProject | null>(null);
  const [device, setDevice] = useState<DeviceSize>("desktop");
  const [busy, setBusy] = useState(false);
  const [refUrl, setRefUrl] = useState("");
  const [analyzing, setAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [exportOpen, setExportOpen] = useState(false);
  const [exporting, setExporting] = useState(false);
  // "auto" = let the router strategy pick. Otherwise a PascalCase Model
  // enum string from the ToolDropdown (or resolved from a `/tool` slug).
  const [model, setModel] = useState<string>("auto");
  const [refineInput, setRefineInput] = useState("");
  const [refineBusy, setRefineBusy] = useState(false);
  const optimize = useOptimizePrompt({
    taskKind: "TextGeneration",
    value: prompt,
    setValue: setPrompt,
  });
  const notify = useUiStore((s) => s.notify);
  const currentProject = useProjectStore((s) => s.currentProject);

  async function handleAnalyze() {
    const trimmed = refUrl.trim();
    if (!trimmed) return;
    setAnalyzing(true);
    try {
      const result = await analyzeUrl(trimmed, { projectPath: currentProject?.path });
      setAnalysis(result);
      notify({
        kind: "success",
        message: "URL analyzed",
        detail: result.title || result.url,
      });
    } catch (err) {
      notify({
        kind: "error",
        message: "URL analysis failed",
        detail: formatError(err),
      });
    } finally {
      setAnalyzing(false);
    }
  }

  async function submit() {
    if (!prompt.trim()) return;
    setBusy(true);
    try {
      // Design-spec order (see phase-9 claude-bridge spec):
      //   1) parseOverride on the RAW prompt → strip slug
      //   2) optimize the cleanPrompt (never the slug — Claude must
      //      not see `/claude` or it may mangle the override)
      //   3) re-attach the slug to the textarea for display so the
      //      user's override survives visually
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

      // Slug always beats ToolDropdown; ToolDropdown beats auto-routing.
      const overrideModel = parsed.override ? resolveOverrideToModel(parsed.override) : undefined;
      const finalModel = overrideModel ?? (model === "auto" ? undefined : model);

      const result = await generateWebsite({
        prompt: textForDispatch,
        template,
        module: "website",
        reference: analysis ?? null,
        model_override: finalModel,
      });
      setProject(result);
      notify({ kind: "success", message: "Generated", detail: result.summary });
    } catch (err) {
      notify({
        kind: "error",
        message: "Generation failed",
        detail: formatError(err),
      });
    } finally {
      setBusy(false);
    }
  }

  // Register this page's submit() as the global "Generate" handler so
  // the header's Generate button fires it. Without this, the header
  // button is a no-op (we saw users wait 10min expecting a response).
  const setActiveGenerate = useAppStore((s) => s.setActiveGenerate);
  useEffect(() => {
    setActiveGenerate(() => {
      void submit();
    });
    return () => setActiveGenerate(null);
  }, [setActiveGenerate, prompt, template, analysis, model, optimize.enabled]);

  function updateFiles(next: GeneratedFile[]) {
    if (!project) return;
    setProject({ ...project, files: next });
  }

  async function handleAssist(input: {
    filePath: string;
    selection: string;
    instruction: string;
    files: GeneratedFile[];
  }): Promise<string> {
    // `input.files` is intentionally ignored — the backend no longer
    // consumes project-wide context in the modify-selection IPC payload
    // (debug-review Important #3). Re-introduce when the prompt actually
    // uses project context.
    const { replacement } = await modifyCodeSelection({
      file_path: input.filePath,
      selection: input.selection,
      instruction: input.instruction,
    });
    if (!replacement.trim()) {
      throw new Error("Assist returned empty replacement");
    }
    return replacement;
  }

  async function handleExport(settings: WebsiteExportSettings) {
    if (!project) return;
    // Close the dialog straight away so the user sees the pending toast land.
    setExportOpen(false);
    setExporting(true);
    try {
      // Prefer the current project's folder so all exports stay near the
      // source. If no project is open (the user is in a scratch session),
      // fall back to the app's projects root.
      const destination = currentProject ? `${currentProject.path}/export` : await projectsRoot();
      const written = await exportWebsite({
        project,
        format: settings.format,
        destination,
        deploy: settings.deploy,
      });
      notify({ kind: "success", message: "Exported", detail: written });
    } catch (err) {
      notify({
        kind: "error",
        message: "Export failed",
        detail: formatError(err),
      });
    } finally {
      setExporting(false);
    }
  }

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      {/* Brief row */}
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            MOD—01 · WEBSITE BUILDER
          </span>
        </div>
        <Tabs
          activeId={template}
          onChange={(id) => setTemplate(id as Template)}
          items={TEMPLATES.map((t) => ({ id: t.id, label: t.label }))}
        />
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Reference URL (optional)"
              id="website-ref-url"
              placeholder="https://stripe.com"
              value={refUrl}
              onValueChange={setRefUrl}
            />
          </div>
          <Button
            variant="secondary"
            onClick={handleAnalyze}
            disabled={!refUrl.trim() || analyzing}
          >
            {analyzing ? "Analyzing…" : "Analyze"}
          </Button>
        </div>
        {analysis ? <AnalysisPanel analysis={analysis} /> : null}
        <Textarea
          id="website-brief"
          label="Describe the site"
          placeholder="A landing page for a specialty coffee roaster. Warm earthy palette, hero with espresso shot, 3-column feature strip."
          value={prompt}
          onValueChange={setPrompt}
          rows={3}
        />
        <div className="flex items-center gap-2">
          <ToolDropdown taskKind="TextGeneration" value={model} onChange={setModel} />
          <OptimizeToggle
            enabled={optimize.enabled}
            onToggle={optimize.setEnabled}
            busy={optimize.busy}
            canUndo={optimize.canUndo}
            onUndo={optimize.undo}
          />
        </div>
        <div className="flex items-center justify-between">
          <span className="font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
            Generated files are editable live in the preview below.
          </span>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={async () => {
                if (!project) return;
                try {
                  await openInBrowser(project);
                  notify({ kind: "success", message: "Im Browser geöffnet" });
                } catch (err) {
                  notify({
                    kind: "error",
                    message: "Konnte Browser nicht öffnen",
                    detail: formatError(err),
                  });
                }
              }}
              disabled={!project}
            >
              <ExternalLink className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
              Im Browser öffnen
            </Button>
            <LoadingButton
              variant="secondary"
              onClick={() => setExportOpen(true)}
              disabled={!project}
              loading={exporting}
            >
              Export
            </LoadingButton>
            <LoadingButton
              variant="primary"
              onClick={submit}
              disabled={!prompt.trim()}
              loading={busy}
            >
              Generate
            </LoadingButton>
          </div>
        </div>
      </div>

      {/* Split: editor / preview */}
      <div className="grid min-h-0 grid-cols-2">
        <div className="flex h-full min-h-0 flex-col border-neutral-dark-700 border-r">
          {project ? (
            <CodeEditor
              files={project.files}
              onChange={updateFiles}
              onRequestAssist={handleAssist}
              onNotify={(message) => notify({ kind: "info", message })}
            />
          ) : (
            <div className="flex h-full items-center justify-center p-10 text-center text-neutral-dark-400">
              Nothing generated yet.
            </div>
          )}
        </div>
        <div className="flex h-full min-h-0 flex-col">
          <div className="flex items-center justify-end gap-2 border-neutral-dark-700 border-b px-4 py-2">
            <DeviceButton
              active={device === "desktop"}
              onClick={() => setDevice("desktop")}
              icon={<Laptop className="h-3 w-3" strokeWidth={1.5} />}
              label="Desktop"
            />
            <DeviceButton
              active={device === "tablet"}
              onClick={() => setDevice("tablet")}
              icon={<Tablet className="h-3 w-3" strokeWidth={1.5} />}
              label="Tablet"
            />
            <DeviceButton
              active={device === "mobile"}
              onClick={() => setDevice("mobile")}
              icon={<Smartphone className="h-3 w-3" strokeWidth={1.5} />}
              label="Mobile"
            />
          </div>
          <div className="min-h-0 flex-1">
            {project ? (
              <DevicePreview files={project.files} device={device} />
            ) : (
              <div className="flex h-full items-center justify-center p-10 text-center text-neutral-dark-400">
                Preview appears after generation.
              </div>
            )}
          </div>
          {project ? (
            <div
              className="flex flex-col gap-2 border-neutral-dark-700 border-t p-4"
              data-testid="refine-panel"
            >
              <span className="font-mono text-2xs text-neutral-dark-400 tracking-label uppercase">
                Refine — weitere Änderungen
              </span>
              <div className="flex items-end gap-2">
                <div className="flex-1">
                  <Textarea
                    id="website-refine-input"
                    aria-label="Refine instruction"
                    value={refineInput}
                    onValueChange={setRefineInput}
                    placeholder="Mach den Planeten rot, entferne den Header, füge Scroll-Animation hinzu…"
                    rows={2}
                  />
                </div>
                <LoadingButton
                  variant="primary"
                  onClick={async () => {
                    if (!project || !refineInput.trim()) return;
                    setRefineBusy(true);
                    try {
                      const result = await refineWebsite(project, refineInput.trim());
                      setProject(result.project);
                      setRefineInput("");
                      const count = result.changed_paths.length;
                      const headPaths = result.changed_paths.slice(0, 3).join(", ");
                      notify({
                        kind: "success",
                        message: "Projekt aktualisiert",
                        detail: `${count} file${count === 1 ? "" : "s"}${headPaths ? `: ${headPaths}` : ""}`,
                      });
                    } catch (err) {
                      notify({
                        kind: "error",
                        message: "Refine fehlgeschlagen",
                        detail: formatError(err),
                      });
                    } finally {
                      setRefineBusy(false);
                    }
                  }}
                  disabled={!refineInput.trim()}
                  loading={refineBusy}
                >
                  Refine
                </LoadingButton>
              </div>
            </div>
          ) : null}
        </div>
      </div>

      {exportOpen ? (
        <WebsiteExportDialog open onClose={() => setExportOpen(false)} onExport={handleExport} />
      ) : null}
    </div>
  );
}

interface DeviceButtonProps {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}

// ─── Analysis Panel ────────────────────────────────────────────────────
//
// Rich, collapsible view of everything the URL analyzer extracted.
// Collapsed state: a single-line summary badge. Expanded state: screenshot
// thumbnail, copy excerpts (hero/nav/sections/CTAs), color swatches,
// typography samples rendered in-style, detected-feature pills, and raw
// details (spacing, CSS custom properties, image URLs).
//
// Kept inline in WebsiteBuilder.tsx to avoid splitting state (analysis + the
// collapse toggle both live in this page) across feature modules.

interface AnalysisPanelProps {
  analysis: AnalysisResult;
}

function AnalysisPanel({ analysis }: AnalysisPanelProps) {
  const [expanded, setExpanded] = useState(false);

  const features = analysis.detected_features ?? {};
  const featureBadges: string[] = [];
  if (features.has_canvas) featureBadges.push("Canvas");
  if (features.has_webgl) featureBadges.push("WebGL");
  if (features.has_three_js) featureBadges.push("Three.js");
  if (features.has_video) featureBadges.push("Video");
  if (features.has_form) featureBadges.push("Form");
  if (features.has_iframe) featureBadges.push("Iframe");

  const summaryBits: string[] = [];
  if (analysis.colors.length) summaryBits.push(`${analysis.colors.length} colors`);
  if (analysis.section_headings?.length)
    summaryBits.push(`${analysis.section_headings.length} sections`);
  if (featureBadges.length) summaryBits.push(`detected: ${featureBadges.join(", ").toLowerCase()}`);

  return (
    <div
      className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 text-2xs"
      data-testid="analysis-panel"
    >
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="flex w-full items-center gap-2 p-3 text-left"
        aria-expanded={expanded}
      >
        {expanded ? (
          <ChevronDown className="h-3 w-3 text-neutral-dark-400" strokeWidth={1.5} />
        ) : (
          <ChevronRight className="h-3 w-3 text-neutral-dark-400" strokeWidth={1.5} />
        )}
        <span className="font-mono text-accent-500 uppercase tracking-label">
          Analyzed · {analysis.title || analysis.url}
        </span>
        {summaryBits.length ? (
          <span className="truncate text-neutral-dark-400">· {summaryBits.join(" · ")}</span>
        ) : null}
      </button>

      {expanded ? (
        <div className="flex flex-col gap-4 border-neutral-dark-700 border-t p-3">
          {/* Screenshot thumbnail */}
          {analysis.screenshotPath ? (
            <div className="flex flex-col gap-1">
              <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                Screenshot
              </span>
              {/* The Tauri asset protocol isn't available in plain browsers /
                  the test DOM, so the <img> only renders when Tauri has
                  rewritten `file://` access. We leave it as a file:// path
                  and let the asset loader fail silently in non-Tauri hosts. */}
              <img
                src={
                  analysis.screenshotPath.startsWith("file://")
                    ? analysis.screenshotPath
                    : `file://${analysis.screenshotPath}`
                }
                alt="Reference screenshot"
                style={{ width: 180 }}
                className="rounded-xs border border-neutral-dark-700"
              />
              <span className="truncate font-mono text-neutral-dark-500">
                {analysis.screenshotPath}
              </span>
            </div>
          ) : null}

          {/* Copy excerpts */}
          <div className="grid grid-cols-2 gap-3">
            {analysis.hero_text ? (
              <AnalysisSection label="Hero">
                <p className="text-neutral-dark-200">{analysis.hero_text}</p>
              </AnalysisSection>
            ) : null}
            {analysis.nav_items?.length ? (
              <AnalysisSection label="Nav">
                <ul className="list-disc pl-4 text-neutral-dark-300">
                  {analysis.nav_items.map((n) => (
                    <li key={n}>{n}</li>
                  ))}
                </ul>
              </AnalysisSection>
            ) : null}
            {analysis.section_headings?.length ? (
              <AnalysisSection label="Sections">
                <ul className="list-disc pl-4 text-neutral-dark-300">
                  {analysis.section_headings.map((s) => (
                    <li key={s}>{s}</li>
                  ))}
                </ul>
              </AnalysisSection>
            ) : null}
            {analysis.cta_labels?.length ? (
              <AnalysisSection label="CTAs">
                <ul className="list-disc pl-4 text-neutral-dark-300">
                  {analysis.cta_labels.map((c) => (
                    <li key={c}>{c}</li>
                  ))}
                </ul>
              </AnalysisSection>
            ) : null}
          </div>

          {/* Color swatches */}
          {analysis.colors.length || analysis.color_roles ? (
            <AnalysisSection label="Colors">
              <div className="flex flex-wrap gap-1.5">
                {analysis.color_roles?.bg ? (
                  <Swatch color={analysis.color_roles.bg} label="bg" />
                ) : null}
                {analysis.color_roles?.fg ? (
                  <Swatch color={analysis.color_roles.fg} label="fg" />
                ) : null}
                {analysis.color_roles?.accent ? (
                  <Swatch color={analysis.color_roles.accent} label="accent" />
                ) : null}
                {analysis.colors.map((c) => (
                  <Swatch key={c} color={c} />
                ))}
              </div>
            </AnalysisSection>
          ) : null}

          {/* Typography samples (rendered in-style) */}
          {analysis.typography?.length ? (
            <AnalysisSection label="Typography">
              <div className="flex flex-col gap-1 text-neutral-dark-200">
                {analysis.typography.map((t) => (
                  <div
                    key={`${t.size}-${t.weight}-${t.family}`}
                    style={{
                      fontSize: t.size,
                      fontWeight: t.weight,
                      fontFamily: t.family,
                      lineHeight: 1.1,
                    }}
                  >
                    {t.size} / {t.weight} · {t.family}
                  </div>
                ))}
              </div>
            </AnalysisSection>
          ) : null}

          {/* Feature pills */}
          {featureBadges.length ? (
            <AnalysisSection label="Detected">
              <div className="flex flex-wrap gap-1.5">
                {featureBadges.map((b) => (
                  <span
                    key={b}
                    className="rounded-xs border border-accent-500/40 bg-accent-500/10 px-1.5 py-0.5 font-mono text-2xs text-accent-500 uppercase tracking-label"
                    data-testid={`feature-badge-${b.toLowerCase()}`}
                  >
                    {b}
                  </span>
                ))}
              </div>
            </AnalysisSection>
          ) : null}

          {/* Raw details */}
          <RawDetails analysis={analysis} />
        </div>
      ) : null}
    </div>
  );
}

function AnalysisSection({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
        {label}
      </span>
      {children}
    </div>
  );
}

function Swatch({ color, label }: { color: string; label?: string }) {
  return (
    <div className="flex items-center gap-1" title={label ? `${label} ${color}` : color}>
      <span
        className="inline-block h-4 w-4 rounded-xs border border-neutral-dark-700"
        style={{ backgroundColor: color }}
      />
      <span className="font-mono text-neutral-dark-400">
        {label ? `${label}: ` : ""}
        {color}
      </span>
    </div>
  );
}

function RawDetails({ analysis }: { analysis: AnalysisResult }) {
  const [showImages, setShowImages] = useState(false);
  const imgs = analysis.image_urls ?? [];
  const spacing = analysis.spacing ?? [];
  const customProps = Object.entries(analysis.customProperties ?? {});

  if (!imgs.length && !spacing.length && !customProps.length) return null;

  return (
    <details className="group">
      <summary className="cursor-pointer font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
        Raw details
      </summary>
      <div className="mt-2 flex flex-col gap-2 text-neutral-dark-300">
        {spacing.length ? (
          <div>
            <span className="font-mono text-neutral-dark-500">Spacing: </span>
            {spacing.join(", ")}
          </div>
        ) : null}
        {customProps.length ? (
          <div>
            <span className="font-mono text-neutral-dark-500">Custom props: </span>
            {customProps
              .slice(0, 8)
              .map(([k, v]) => `${k}=${v}`)
              .join("; ")}
            {customProps.length > 8 ? ` (+${customProps.length - 8} more)` : ""}
          </div>
        ) : null}
        {imgs.length ? (
          <div>
            <span className="font-mono text-neutral-dark-500">Images ({imgs.length}): </span>
            <ul className="mt-1 flex flex-col gap-0.5 break-all">
              {imgs.slice(0, showImages ? imgs.length : 3).map((u) => (
                <li key={u} className="truncate font-mono text-neutral-dark-400">
                  {u}
                </li>
              ))}
            </ul>
            {imgs.length > 3 ? (
              <button
                type="button"
                className="mt-1 font-mono text-accent-500 uppercase tracking-label"
                onClick={() => setShowImages((v) => !v)}
              >
                {showImages ? "show less" : `show ${imgs.length - 3} more`}
              </button>
            ) : null}
          </div>
        ) : null}
      </div>
    </details>
  );
}

function DeviceButton({ active, onClick, icon, label }: DeviceButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex items-center gap-1.5 rounded-xs border px-2 py-1 font-mono text-2xs uppercase tracking-label ${
        active
          ? "border-accent-500 text-accent-500"
          : "border-neutral-dark-700 text-neutral-dark-400 hover:border-neutral-dark-600 hover:text-neutral-dark-100"
      }`}
    >
      {icon}
      {label}
    </button>
  );
}
