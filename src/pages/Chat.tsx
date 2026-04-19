import { Send, Sparkles, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";
import { ToolDropdown } from "@/components/ui/ToolDropdown";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";
import { sendChatMessage } from "@/lib/chatCommands";
import { useAppStore } from "@/stores/appStore";
import { useChatStore } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";

export function ChatPage() {
  const messages = useChatStore((s) => s.messages);
  const addMessage = useChatStore((s) => s.addMessage);
  const appendChunk = useChatStore((s) => s.appendChunk);
  const newChat = useChatStore((s) => s.newChat);
  const notify = useUiStore((s) => s.notify);

  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [model, setModel] = useState<string>("auto");
  const optimize = useOptimizePrompt({
    taskKind: "TextGeneration",
    value: input,
    setValue: setInput,
  });

  async function handleSend() {
    if (!input.trim() || busy) return;
    setBusy(true);
    try {
      const userText = input.trim();
      addMessage({ role: "user", content: userText });
      setInput("");
      const assistantId = addMessage({ role: "assistant", content: "" });
      // Snapshot the messages *without* the empty assistant row and substitute
      // the current user text (makes the transcript self-consistent for the
      // backend even though the store's tail message is the in-progress one).
      const priorMessages = useChatStore.getState().messages.slice(0, -1);
      await sendChatMessage(
        priorMessages,
        assistantId,
        (chunk) => appendChunk(assistantId, chunk),
        (error) => {
          if (error) {
            notify({ kind: "error", message: "Chat failed", detail: error });
          }
        },
      );
    } finally {
      setBusy(false);
    }
  }

  // Header Generate button → send the current chat message.
  const setActiveGenerate = useAppStore((s) => s.setActiveGenerate);
  useEffect(() => {
    setActiveGenerate(() => {
      void handleSend();
    });
    return () => setActiveGenerate(null);
  }, [setActiveGenerate, input, busy]);

  return (
    <div className="grid h-full grid-rows-[auto_1fr_auto]">
      <div className="flex items-center justify-between border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            CHAT · CLAUDE
          </span>
        </div>
        <button
          type="button"
          onClick={newChat}
          className="flex items-center gap-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label hover:text-neutral-dark-200"
        >
          <Trash2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          New chat
        </button>
      </div>

      <div className="overflow-y-auto p-6" role="log" aria-live="polite">
        <div className="flex flex-col gap-4">
          {messages.length === 0 ? (
            <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              <Sparkles className="mr-2 inline h-3 w-3" /> Start a conversation
            </div>
          ) : (
            messages.map((m) => (
              <div
                key={m.id}
                className={`flex max-w-[80%] flex-col gap-1 rounded-xs border p-3 text-2xs ${m.role === "user" ? "self-end border-accent-500/40 bg-accent-500/10 text-neutral-dark-100" : "self-start border-neutral-dark-700 bg-neutral-dark-900 text-neutral-dark-200"}`}
              >
                <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                  {m.role}
                </span>
                <p className="whitespace-pre-wrap leading-relaxed">{m.content}</p>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="flex flex-col gap-2 border-neutral-dark-700 border-t p-4">
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
        <div className="flex items-end gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                if (optimize.enabled) {
                  void optimize.optimize().then(handleSend);
                } else {
                  void handleSend();
                }
              }
            }}
            placeholder="Type a message — Cmd+Enter to send"
            className="min-h-[60px] flex-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-2 font-mono text-2xs text-neutral-dark-100"
          />
          <LoadingButton
            variant="primary"
            onClick={async () => {
              if (optimize.enabled) {
                await optimize.optimize();
              }
              await handleSend();
            }}
            disabled={!input.trim() || busy}
            loading={busy}
          >
            <Send className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            Send
          </LoadingButton>
        </div>
      </div>
    </div>
  );
}
