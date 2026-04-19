import { Sandpack, type SandpackPredefinedTemplate } from "@codesandbox/sandpack-react";
import type { GeneratedFile } from "@/lib/websiteCommands";

export interface SandpackPreviewProps {
  files: readonly GeneratedFile[];
  device: "desktop" | "tablet" | "mobile";
}

/**
 * Renders a generated Vite/React/JSX project inside Sandpack's in-browser
 * bundler. Unlike the sibling {@link DevicePreview}, this component can
 * execute JSX + imports + Tailwind + Three.js — the full runtime Claude
 * now generates when asked for a real multi-file project.
 *
 * The template is auto-detected from the file shape (`.tsx` → `vite-react-ts`,
 * `.jsx` → `vite-react`, a `vite.config.*` → `vite`, else `static`).
 * Dependencies are inferred from any `package.json` shipped in the project.
 */
export function SandpackPreview({ files, device }: SandpackPreviewProps) {
  const sandpackFiles: Record<string, string> = {};
  for (const f of files) {
    const key = f.path.startsWith("/") ? f.path : `/${f.path}`;
    sandpackFiles[key] = f.content;
  }
  const template = guessTemplate(files);
  const dependencies = inferDependencies(files);

  return (
    <div className="h-full w-full overflow-hidden bg-neutral-dark-950" style={{ minHeight: 400 }}>
      <Sandpack
        template={template}
        files={sandpackFiles}
        options={{
          showNavigator: false,
          showTabs: false,
          showLineNumbers: false,
          showInlineErrors: true,
          editorHeight: 0,
          showConsole: false,
          autoReload: true,
          recompileMode: "delayed",
          recompileDelay: 600,
        }}
        customSetup={Object.keys(dependencies).length ? { dependencies } : undefined}
        theme="dark"
        // Remount on template/device change so Sandpack rebuilds cleanly.
        key={`${template}-${device}`}
      />
    </div>
  );
}

export function guessTemplate(files: readonly GeneratedFile[]): SandpackPredefinedTemplate {
  const hasTsx = files.some((f) => f.path.endsWith(".tsx"));
  if (hasTsx) return "vite-react-ts";
  const hasJsx = files.some((f) => f.path.endsWith(".jsx"));
  if (hasJsx) return "vite-react";
  const hasVite = files.some(
    (f) => f.path.endsWith("vite.config.js") || f.path.endsWith("vite.config.ts"),
  );
  if (hasVite) return "vite";
  return "static";
}

export function inferDependencies(files: readonly GeneratedFile[]): Record<string, string> {
  const pkg = files.find((f) => f.path.endsWith("package.json") || f.path === "/package.json");
  if (!pkg) return {};
  try {
    const parsed = JSON.parse(pkg.content) as {
      dependencies?: Record<string, string>;
    };
    return parsed.dependencies ?? {};
  } catch {
    return {};
  }
}

/**
 * True when a generated project looks like a JS/TS app (JSX/TSX files, a
 * Vite config, or a package.json that declares React). DevicePreview uses
 * this to decide whether to hand the files off to Sandpack.
 */
export function shouldUseSandpack(files: readonly GeneratedFile[]): boolean {
  if (files.some((f) => f.path.endsWith(".jsx") || f.path.endsWith(".tsx"))) {
    return true;
  }
  if (files.some((f) => f.path.endsWith("vite.config.js") || f.path.endsWith("vite.config.ts"))) {
    return true;
  }
  const pkg = files.find((f) => f.path.endsWith("package.json") || f.path === "/package.json");
  if (pkg) {
    try {
      const parsed = JSON.parse(pkg.content) as {
        dependencies?: Record<string, string>;
      };
      if (parsed.dependencies?.react) return true;
    } catch {
      // Malformed package.json — fall back to static preview.
    }
  }
  return false;
}
