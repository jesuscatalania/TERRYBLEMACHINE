import { Laptop, Smartphone, Tablet } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Textarea } from "@/components/ui/Input";
import { Tabs } from "@/components/ui/Tabs";
import { CodeEditor } from "@/components/website/CodeEditor";
import { DevicePreview, type DeviceSize } from "@/components/website/DevicePreview";
import {
  type GeneratedFile,
  type GeneratedProject,
  generateWebsite,
  type Template,
} from "@/lib/websiteCommands";
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
  const notify = useUiStore((s) => s.notify);

  async function submit() {
    if (!prompt.trim()) return;
    setBusy(true);
    try {
      const result = await generateWebsite({
        prompt: prompt.trim(),
        template,
        module: "website",
      });
      setProject(result);
      notify({ kind: "success", message: "Generated", detail: result.summary });
    } catch (err) {
      notify({
        kind: "error",
        message: "Generation failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setBusy(false);
    }
  }

  function updateFiles(next: GeneratedFile[]) {
    if (!project) return;
    setProject({ ...project, files: next });
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
        <Textarea
          id="website-brief"
          label="Describe the site"
          placeholder="A landing page for a specialty coffee roaster. Warm earthy palette, hero with espresso shot, 3-column feature strip."
          value={prompt}
          onValueChange={setPrompt}
          rows={3}
        />
        <div className="flex items-center justify-between">
          <span className="font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
            Generated files are editable live in the preview below.
          </span>
          <Button variant="primary" onClick={submit} disabled={!prompt.trim() || busy}>
            {busy ? "Generating…" : "Generate"}
          </Button>
        </div>
      </div>

      {/* Split: editor / preview */}
      <div className="grid min-h-0 grid-cols-2">
        <div className="flex h-full min-h-0 flex-col border-neutral-dark-700 border-r">
          {project ? (
            <CodeEditor files={project.files} onChange={updateFiles} />
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
        </div>
      </div>
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
