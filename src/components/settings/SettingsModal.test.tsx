import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PROVIDERS } from "@/components/settings/providers";
import { SettingsModal } from "@/components/settings/SettingsModal";
import { useModalStackStore } from "@/stores/modalStackStore";
import { useUiStore } from "@/stores/uiStore";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const { invoke } = await import("@tauri-apps/api/core");
const invokeMock = vi.mocked(invoke);

function resetState() {
  useUiStore.setState({ modals: [], notifications: [], loadingJobs: 0 });
  useModalStackStore.setState({ stack: [] });
}

describe("SettingsModal", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    resetState();
  });

  afterEach(() => {
    resetState();
  });

  it("does not render when open=false", () => {
    invokeMock.mockResolvedValue([]);
    render(<SettingsModal open={false} onClose={() => {}} />);
    expect(screen.queryByRole("dialog")).toBeNull();
    // Should not have called listApiKeys either.
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("renders all 9 providers when opened", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "list_api_keys") return Promise.resolve([]);
      if (cmd === "get_claude_transport") return Promise.resolve("auto");
      if (cmd === "detect_claude_cli") return Promise.resolve(null);
      return Promise.reject(new Error(`unexpected cmd: ${cmd}`));
    });

    render(<SettingsModal open={true} onClose={() => {}} />);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("list_api_keys");
    });
    expect(PROVIDERS).toHaveLength(9);
    for (const p of PROVIDERS) {
      expect(screen.getByText(p.label)).toBeInTheDocument();
    }
  });

  it("marks configured providers with the green dot", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "list_api_keys") return Promise.resolve(["claude", "fal"]);
      if (cmd === "get_claude_transport") return Promise.resolve("auto");
      if (cmd === "detect_claude_cli") return Promise.resolve(null);
      return Promise.reject(new Error(`unexpected cmd: ${cmd}`));
    });

    render(<SettingsModal open={true} onClose={() => {}} />);

    await waitFor(() => {
      expect(screen.getByTestId("provider-dot-claude").getAttribute("data-configured")).toBe(
        "true",
      );
    });
    expect(screen.getByTestId("provider-dot-fal").getAttribute("data-configured")).toBe("true");
    expect(screen.getByTestId("provider-dot-runway").getAttribute("data-configured")).toBe("false");
    expect(screen.getByTestId("provider-dot-replicate").getAttribute("data-configured")).toBe(
      "false",
    );
  });

  it("surfaces a load failure as an error toast", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "list_api_keys")
        return Promise.reject({ kind: "Keychain", detail: "keychain unavailable" });
      if (cmd === "get_claude_transport") return Promise.resolve("auto");
      if (cmd === "detect_claude_cli") return Promise.resolve(null);
      return Promise.reject(new Error(`unexpected cmd: ${cmd}`));
    });

    render(<SettingsModal open={true} onClose={() => {}} />);

    await waitFor(() => {
      const notes = useUiStore.getState().notifications;
      const keychainToast = notes.find((n) => n.detail === "keychain unavailable");
      expect(keychainToast).toBeDefined();
      expect(keychainToast?.kind).toBe("error");
    });
  });
});
