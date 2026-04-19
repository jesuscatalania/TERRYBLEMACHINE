import { beforeEach, describe, expect, it, vi } from "vitest";

const unlistenChunk = vi.fn();
const unlistenDone = vi.fn();
const listen = vi.fn();
const invoke = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listen(...args),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import { sendChatMessage } from "@/lib/chatCommands";

beforeEach(() => {
  vi.clearAllMocks();
  listen.mockResolvedValueOnce(unlistenChunk).mockResolvedValueOnce(unlistenDone);
});

describe("sendChatMessage", () => {
  it("subscribes to chunk + done and invokes chat_send_message", async () => {
    invoke.mockResolvedValueOnce(undefined);
    await sendChatMessage(
      [{ id: "1", role: "user", content: "hi", createdAt: "t" }],
      "asst-1",
      () => {},
      () => {},
    );
    expect(listen).toHaveBeenCalledTimes(2);
    expect(listen.mock.calls[0]?.[0]).toBe("chat:stream-chunk");
    expect(listen.mock.calls[1]?.[0]).toBe("chat:stream-done");
    expect(invoke).toHaveBeenCalledWith("chat_send_message", {
      input: { messages: [{ role: "user", content: "hi" }] },
      messageId: "asst-1",
    });
  });

  it("on invoke rejection, calls onDone with error and unlistens both", async () => {
    invoke.mockRejectedValueOnce(new Error("nope"));
    const onDone = vi.fn();
    await sendChatMessage([], "asst-1", () => {}, onDone);
    expect(onDone).toHaveBeenCalledWith("Error: nope");
    expect(unlistenChunk).toHaveBeenCalled();
    expect(unlistenDone).toHaveBeenCalled();
  });
});
