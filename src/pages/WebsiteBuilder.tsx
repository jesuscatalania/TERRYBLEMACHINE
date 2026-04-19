import { ExternalLink, Laptop, Smartphone, Tablet } from "lucide-react";
import { useState } from "react";
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
        {analysis ? (
          <div className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-3 text-2xs">
            <div className="font-mono text-accent-500 uppercase tracking-label">
              Analyzed · {analysis.title || analysis.url}
            </div>
            <div className="mt-1 text-neutral-dark-300">
              Colors: {analysis.colors.slice(0, 6).join(", ") || "—"}
              {" — "}
              Fonts: {analysis.fonts.slice(0, 3).join(", ") || "—"}
              {analysis.assets && analysis.assets.length > 0
                ? ` — Assets: ${analysis.assets.length}`
                : ""}
            </div>
          </div>
        ) : null}
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
