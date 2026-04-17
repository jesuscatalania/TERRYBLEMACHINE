import Editor, { type OnMount } from "@monaco-editor/react";
import { useCallback, useState } from "react";
import { Tabs } from "@/components/ui/Tabs";
import type { GeneratedFile } from "@/lib/websiteCommands";

export interface CodeEditorProps {
  files: readonly GeneratedFile[];
  /** Called when the user edits a file. */
  onChange: (files: GeneratedFile[]) => void;
}

/** Derive Monaco's language id from a file path. */
function languageFor(path: string): string {
  const lower = path.toLowerCase();
  if (lower.endsWith(".html")) return "html";
  if (lower.endsWith(".css")) return "css";
  if (lower.endsWith(".tsx") || lower.endsWith(".jsx")) return "javascript";
  if (lower.endsWith(".ts") || lower.endsWith(".js")) return "javascript";
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".md")) return "markdown";
  return "plaintext";
}

export function CodeEditor({ files, onChange }: CodeEditorProps) {
  const [activeId, setActiveId] = useState(() => files[0]?.path ?? "");
  const active = files.find((f) => f.path === activeId) ?? files[0] ?? null;

  const handleChange = useCallback(
    (value: string | undefined) => {
      if (!active) return;
      const next = files.map((f) => (f.path === active.path ? { ...f, content: value ?? "" } : f));
      onChange(next);
    },
    [active, files, onChange],
  );

  const onMount: OnMount = (_editor, monaco) => {
    monaco.editor.setTheme("vs-dark");
  };

  if (!active) {
    return (
      <div className="flex h-full items-center justify-center text-neutral-dark-500">
        No files to edit.
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <Tabs
        activeId={activeId}
        onChange={setActiveId}
        items={files.map((f) => ({ id: f.path, label: f.path }))}
      />
      <div className="min-h-0 flex-1">
        <Editor
          height="100%"
          path={active.path}
          language={languageFor(active.path)}
          value={active.content}
          onChange={handleChange}
          onMount={onMount}
          options={{
            fontSize: 12,
            fontFamily: "IBM Plex Mono, SF Mono, monospace",
            minimap: { enabled: false },
            wordWrap: "on",
            automaticLayout: true,
            scrollBeyondLastLine: false,
          }}
        />
      </div>
    </div>
  );
}
