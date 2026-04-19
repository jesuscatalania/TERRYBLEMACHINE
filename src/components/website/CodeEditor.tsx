import Editor, { type Monaco, type OnMount } from "@monaco-editor/react";
import { useCallback, useRef, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Tabs } from "@/components/ui/Tabs";
import { formatError } from "@/lib/formatError";
import type { GeneratedFile } from "@/lib/websiteCommands";
import { AssistPopover } from "./AssistPopover";

export interface CodeEditorAssistInput {
  /** File being edited (same as the currently active tab). */
  filePath: string;
  /** Text highlighted in the Monaco editor. Never empty. */
  selection: string;
  /** User instruction (already trimmed, guaranteed non-empty). */
  instruction: string;
  /** Snapshot of every file, so the parent can forward richer context. */
  files: GeneratedFile[];
}

export interface CodeEditorProps {
  files: readonly GeneratedFile[];
  /** Called when the user edits a file. */
  onChange: (files: GeneratedFile[]) => void;
  /**
   * Optional Cmd+K handler. When provided, a "Modify…" button is shown and
   * Cmd/Ctrl+K opens the AssistPopover. The parent is responsible for
   * invoking the backend and returning the replacement string.
   */
  onRequestAssist?: (input: CodeEditorAssistInput) => Promise<string>;
  /** Optional notifier for non-fatal messages ("Select code first" etc.). */
  onNotify?: (message: string) => void;
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

type MonacoEditor = Parameters<OnMount>[0];
type MonacoRange = ReturnType<MonacoEditor["getSelection"]>;

export function CodeEditor({ files, onChange, onRequestAssist, onNotify }: CodeEditorProps) {
  const [activeId, setActiveId] = useState(() => files[0]?.path ?? "");
  const active = files.find((f) => f.path === activeId) ?? files[0] ?? null;

  const editorRef = useRef<MonacoEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);

  const [assistSelection, setAssistSelection] = useState<string | null>(null);
  const [assistBusy, setAssistBusy] = useState(false);
  // We stash the exact Monaco range at popover-open time so the replacement
  // lands in the originally selected region even if focus/selection changes
  // while the user types the instruction.
  const pendingRangeRef = useRef<MonacoRange | null>(null);

  const handleChange = useCallback(
    (value: string | undefined) => {
      if (!active) return;
      const next = files.map((f) => (f.path === active.path ? { ...f, content: value ?? "" } : f));
      onChange(next);
    },
    [active, files, onChange],
  );

  const openAssist = useCallback(() => {
    if (!onRequestAssist) return;
    const editor = editorRef.current;
    if (!editor) return;
    const selection = editor.getSelection();
    const model = editor.getModel();
    if (!selection || !model) {
      onNotify?.("Select code first");
      return;
    }
    const text = model.getValueInRange(selection);
    if (!text.trim()) {
      onNotify?.("Select code first");
      return;
    }
    pendingRangeRef.current = selection;
    setAssistSelection(text);
  }, [onRequestAssist, onNotify]);

  const onMount: OnMount = (editor, monaco) => {
    monaco.editor.setTheme("vs-dark");
    editorRef.current = editor;
    monacoRef.current = monaco;

    if (onRequestAssist) {
      // Cmd+K on macOS, Ctrl+K elsewhere.
      editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK, () => {
        openAssist();
      });
    }
  };

  const closeAssist = useCallback(() => {
    setAssistSelection(null);
    pendingRangeRef.current = null;
    setAssistBusy(false);
  }, []);

  const applyAssist = useCallback(
    async (instruction: string) => {
      if (!onRequestAssist || !active || assistSelection === null) return;
      const editor = editorRef.current;
      const range = pendingRangeRef.current;
      if (!editor || !range) return;

      setAssistBusy(true);
      try {
        const replacement = await onRequestAssist({
          filePath: active.path,
          selection: assistSelection,
          instruction,
          files: files.map((f) => ({ path: f.path, content: f.content })),
        });
        editor.executeEdits("assist", [
          {
            range,
            text: replacement,
            forceMoveMarkers: true,
          },
        ]);
        closeAssist();
      } catch (err) {
        onNotify?.(formatError(err));
        setAssistBusy(false);
      }
    },
    [onRequestAssist, active, assistSelection, files, closeAssist, onNotify],
  );

  if (!active) {
    return (
      <div className="flex h-full items-center justify-center text-neutral-dark-500">
        No files to edit.
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between gap-2 pr-3">
        <div className="min-w-0 flex-1">
          <Tabs
            activeId={activeId}
            onChange={setActiveId}
            items={files.map((f) => ({ id: f.path, label: f.path }))}
          />
        </div>
        {onRequestAssist ? (
          <Button variant="secondary" size="sm" onClick={openAssist} title="Modify selection (⌘K)">
            Modify…
          </Button>
        ) : null}
      </div>
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
      {assistSelection !== null ? (
        <AssistPopover
          selection={assistSelection}
          onSubmit={applyAssist}
          onClose={closeAssist}
          busy={assistBusy}
        />
      ) : null}
    </div>
  );
}
