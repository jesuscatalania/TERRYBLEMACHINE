import { getModule } from "@/components/shell/modules";
import { SchematicFrame } from "@/components/shell/SchematicFrame";
import { Button } from "@/components/ui/Button";
import { useAppStore } from "@/stores/appStore";

export function HomePage() {
  const activeModule = useAppStore((s) => s.activeModule);
  const mod = getModule(activeModule);

  return (
    <div className="flex h-full items-center justify-center p-10">
      <SchematicFrame figLabel="FIG 01 — READY" tag={mod.tag} className="w-full max-w-xl">
        <div className="flex flex-col items-center gap-6 text-center">
          <h1 className="font-display text-3xl font-bold text-neutral-dark-50 tracking-tight">
            Describe what to build.
          </h1>
          <p className="max-w-sm text-neutral-dark-300 leading-relaxed">
            Text, image, or URL. The taste engine shapes every output against your{" "}
            <span className="font-mono text-accent-500">meingeschmack/</span> rules.
          </p>
          <div className="flex gap-2.5">
            <Button>URL Scan</Button>
            <Button>Upload</Button>
            <Button variant="primary">New Prompt</Button>
          </div>
        </div>
      </SchematicFrame>
    </div>
  );
}
