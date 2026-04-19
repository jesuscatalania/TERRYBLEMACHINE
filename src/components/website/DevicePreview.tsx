import { useEffect, useMemo, useRef } from "react";
import type { GeneratedFile } from "@/lib/websiteCommands";
import { SandpackPreview, shouldUseSandpack } from "./SandpackPreview";

export type DeviceSize = "desktop" | "tablet" | "mobile";

const DEVICE_WIDTHS: Record<DeviceSize, number> = {
  desktop: 1920,
  tablet: 768,
  mobile: 375,
};

export interface DevicePreviewProps {
  files: readonly GeneratedFile[];
  device: DeviceSize;
  /** Debounce the re-render of the iframe to keep hot-reload under ~500ms. */
  debounceMs?: number;
}

/**
 * Renders the generated project's `index.html` inside an iframe scoped to a
 * device-specific width. Uses `srcdoc` so changes land immediately without a
 * filesystem round-trip.
 *
 * When the generated project looks like a JS/TS bundle (JSX/TSX files, a
 * Vite config, or `package.json` with React), execution is handed off to
 * {@link SandpackPreview} which spins up a real in-browser bundler. Plain
 * static HTML projects keep using the cheap iframe path below.
 */
export function DevicePreview({ files, device, debounceMs = 150 }: DevicePreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  const useSandpack = useMemo(() => shouldUseSandpack(files), [files]);
  const html = useMemo(() => (useSandpack ? "" : composeHtml(files)), [files, useSandpack]);

  useEffect(() => {
    if (useSandpack) return;
    const id = window.setTimeout(() => {
      if (iframeRef.current) {
        iframeRef.current.srcdoc = html;
      }
    }, debounceMs);
    return () => window.clearTimeout(id);
  }, [html, debounceMs, useSandpack]);

  if (useSandpack) {
    return (
      <div
        className="relative flex h-full w-full flex-col bg-neutral-dark-950 p-4"
        data-testid="device-preview-sandpack"
      >
        <div className="mb-1 flex items-center gap-2 font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
          <span>{device}</span>
          <span>·</span>
          <span>Sandpack</span>
        </div>
        <div className="min-h-0 flex-1">
          <SandpackPreview files={files} device={device} />
        </div>
      </div>
    );
  }

  return (
    <div className="relative flex h-full w-full items-center justify-center bg-neutral-dark-950 p-4">
      <div
        className="flex h-full flex-col items-center"
        style={{ width: DEVICE_WIDTHS[device], maxWidth: "100%" }}
      >
        <div className="mb-1 flex items-center gap-2 font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
          <span>{device}</span>
          <span>·</span>
          <span className="tabular-nums">{DEVICE_WIDTHS[device]}px</span>
        </div>
        <iframe
          ref={iframeRef}
          title="Generated website preview"
          data-testid="device-preview-iframe"
          className="h-full w-full rounded-xs border border-neutral-dark-600 bg-white"
          sandbox="allow-same-origin"
        />
      </div>
    </div>
  );
}

/**
 * Collapses multi-file output into a single HTML document by:
 * - Finding the first `index.html` (or first `.html` file) and using its
 *   body as the base.
 * - Inlining `.css` files into a `<style>` tag appended to `<head>`.
 * - Leaving other asset references as-is (the preview is read-only).
 */
export function composeHtml(files: readonly GeneratedFile[]): string {
  const index =
    files.find((f) => f.path.toLowerCase().endsWith("index.html")) ??
    files.find((f) => f.path.toLowerCase().endsWith(".html")) ??
    null;
  if (!index) {
    return "<!doctype html><html><body><p>No index.html in generated project.</p></body></html>";
  }

  const cssFiles = files.filter((f) => f.path.toLowerCase().endsWith(".css"));
  if (cssFiles.length === 0) return index.content;

  const inlineStyles = cssFiles
    .map((f) => `<style data-from="${escapeHtml(f.path)}">${f.content}</style>`)
    .join("\n");

  if (index.content.includes("</head>")) {
    return index.content.replace("</head>", `${inlineStyles}\n</head>`);
  }
  // No head — prepend styles before the body.
  return `${inlineStyles}\n${index.content}`;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
