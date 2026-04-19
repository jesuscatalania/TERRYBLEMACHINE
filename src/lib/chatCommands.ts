import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ChatMessage } from "@/stores/chatStore";

export interface ChunkEvent {
  message_id: string;
  chunk: string;
}

export interface DoneEvent {
  message_id: string;
  error?: string;
}

/**
 * Send a chat message through the `chat_send_message` Tauri command and route
 * streamed chunks back to the caller. Listeners are set up BEFORE invoke so
 * no event is missed, and are cleaned up when the done event fires or when
 * invoke itself rejects. `onDone` is guarded by a `doneFired` flag so it is
 * invoked exactly once even if both the done-event and an invoke rejection
 * occur (e.g. Tauri plumbing failure after the backend already emitted done).
 */
export async function sendChatMessage(
  messages: ChatMessage[],
  assistantMessageId: string,
  onChunk: (chunk: string) => void,
  onDone: (error?: string) => void,
): Promise<void> {
  let doneFired = false;
  const fireDone = (error?: string) => {
    if (doneFired) return;
    doneFired = true;
    onDone(error);
  };

  const unlistenChunk = await listen<ChunkEvent>("chat:stream-chunk", (event) => {
    if (event.payload.message_id === assistantMessageId) {
      onChunk(event.payload.chunk);
    }
  });
  const unlistenDone = await listen<DoneEvent>("chat:stream-done", (event) => {
    if (event.payload.message_id === assistantMessageId) {
      fireDone(event.payload.error);
      unlistenChunk();
      unlistenDone();
    }
  });
  await invoke("chat_send_message", {
    input: { messages: messages.map((m) => ({ role: m.role, content: m.content })) },
    messageId: assistantMessageId,
  }).catch((err) => {
    fireDone(String(err));
    unlistenChunk();
    unlistenDone();
  });
}
