import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { detectClaudeCli, getClaudeTransport, setClaudeTransport } from "@/lib/claudeTransport";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("claudeTransport", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("detectClaudeCli returns the resolved binary path", async () => {
    invokeMock.mockResolvedValueOnce("/opt/homebrew/bin/claude");
    const path = await detectClaudeCli();
    expect(path).toBe("/opt/homebrew/bin/claude");
    expect(invokeMock).toHaveBeenCalledWith("detect_claude_cli");
  });

  it("getClaudeTransport returns the stored transport string", async () => {
    invokeMock.mockResolvedValueOnce("cli");
    const t = await getClaudeTransport();
    expect(t).toBe("cli");
    expect(invokeMock).toHaveBeenCalledWith("get_claude_transport");
  });

  it("setClaudeTransport passes the transport as a named argument", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await setClaudeTransport("api");
    expect(invokeMock).toHaveBeenCalledWith("set_claude_transport", {
      transport: "api",
    });
  });
});
