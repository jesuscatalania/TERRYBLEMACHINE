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

  it("calls onDone exactly once when both done-event and invoke-reject occur", async () => {
    // Reset the default beforeEach listen wiring so we can control the done listener.
    listen.mockReset();
    let doneCb: ((payload: { message_id: string; error?: string }) => void) | null = null;
    listen
      .mockResolvedValueOnce(unlistenChunk)
      .mockImplementationOnce(
        (_name: string, cb: (e: { payload: { message_id: string; error?: string } }) => void) => {
          doneCb = (payload) => cb({ payload });
          return Promise.resolve(unlistenDone);
        },
      );
    // Make invoke reject AFTER we fire the done-event below.
    let rejectInvoke: ((err: Error) => void) | null = null;
    invoke.mockImplementationOnce(
      () =>
        new Promise((_resolve, reject) => {
          rejectInvoke = reject;
        }),
    );
    const onDone = vi.fn();
    const promise = sendChatMessage([], "asst-1", () => {}, onDone);
    // Give the listener awaits a tick to resolve.
    await Promise.resolve();
    await Promise.resolve();
    // Simulate done-event arriving first.
    doneCb?.({ message_id: "asst-1", error: "from-event" });
    // Then invoke rejects.
    rejectInvoke?.(new Error("late"));
    await promise;
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(onDone).toHaveBeenCalledWith("from-event");
  });
});
