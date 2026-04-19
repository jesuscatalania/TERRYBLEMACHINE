import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProviderKeyRow } from "@/components/settings/ProviderKeyRow";
import type { ProviderDef } from "@/components/settings/providers";
import { useUiStore } from "@/stores/uiStore";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const { invoke } = await import("@tauri-apps/api/core");
const invokeMock = vi.mocked(invoke);

const PROVIDER: ProviderDef = {
  id: "claude",
  label: "Anthropic Claude",
  plan: "Subscription (Pro/Max)",
  helpUrl: "https://example.test/keys",
};

function resetStore() {
  useUiStore.setState({ modals: [], notifications: [], loadingJobs: 0 });
}

describe("ProviderKeyRow", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    resetStore();
  });

  afterEach(() => {
    resetStore();
  });

  it("renders not-configured state with gray dot and no Remove button", () => {
    render(<ProviderKeyRow provider={PROVIDER} configured={false} onChange={() => {}} />);
    const dot = screen.getByTestId(`provider-dot-${PROVIDER.id}`);
    expect(dot).toHaveAttribute("data-configured", "false");
    expect(screen.queryByRole("button", { name: /remove .* key/i })).toBeNull();
    expect(screen.getByText(PROVIDER.label)).toBeInTheDocument();
  });

  it("renders configured state with green dot and a Remove button", () => {
    render(<ProviderKeyRow provider={PROVIDER} configured={true} onChange={() => {}} />);
    const dot = screen.getByTestId(`provider-dot-${PROVIDER.id}`);
    expect(dot).toHaveAttribute("data-configured", "true");
    expect(
      screen.getByRole("button", { name: /remove anthropic claude key/i }),
    ).toBeInTheDocument();
  });

  it("save is disabled while the input is empty", () => {
    render(<ProviderKeyRow provider={PROVIDER} configured={false} onChange={() => {}} />);
    expect(screen.getByRole("button", { name: /^save$/i })).toBeDisabled();
  });

  it("saving calls store_api_key with the trimmed value and fires onChange + success toast", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    invokeMock.mockResolvedValueOnce(undefined); // store_api_key
    render(<ProviderKeyRow provider={PROVIDER} configured={false} onChange={onChange} />);

    const input = screen.getByLabelText(/anthropic claude api key/i);
    await user.type(input, "  sk-live-xyz  ");
    await user.click(screen.getByRole("button", { name: /^save$/i }));

    expect(invokeMock).toHaveBeenCalledWith("store_api_key", {
      service: "claude",
      key: "sk-live-xyz",
    });
    expect(onChange).toHaveBeenCalledOnce();
    const notes = useUiStore.getState().notifications;
    const last = notes[notes.length - 1];
    expect(last?.kind).toBe("success");
    expect(last?.message).toMatch(/saved anthropic claude/i);
  });

  it("surfaces a typed KeyStoreIpcError as an error toast with detail", async () => {
    const user = userEvent.setup();
    invokeMock.mockRejectedValueOnce({ kind: "Keychain", detail: "simulated failure" });
    render(<ProviderKeyRow provider={PROVIDER} configured={false} onChange={() => {}} />);

    await user.type(screen.getByLabelText(/anthropic claude api key/i), "sk-x");
    await user.click(screen.getByRole("button", { name: /^save$/i }));

    const notes = useUiStore.getState().notifications;
    const last = notes[notes.length - 1];
    expect(last?.kind).toBe("error");
    expect(last?.message).toMatch(/saving anthropic claude key failed/i);
    expect(last?.detail).toBe("simulated failure");
  });

  it("deleting calls delete_api_key and fires onChange + success toast", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    invokeMock.mockResolvedValueOnce(undefined); // delete_api_key
    render(<ProviderKeyRow provider={PROVIDER} configured={true} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: /remove anthropic claude key/i }));

    expect(invokeMock).toHaveBeenCalledWith("delete_api_key", { service: "claude" });
    expect(onChange).toHaveBeenCalledOnce();
    const notes = useUiStore.getState().notifications;
    const last = notes[notes.length - 1];
    expect(last?.kind).toBe("success");
    expect(last?.message).toMatch(/removed anthropic claude/i);
  });

  it("Show/Hide toggle flips the input type between password and text", async () => {
    const user = userEvent.setup();
    render(<ProviderKeyRow provider={PROVIDER} configured={false} onChange={() => {}} />);
    const input = screen.getByLabelText(/anthropic claude api key/i) as HTMLInputElement;
    // Before typing, the Show button is disabled (no value to reveal).
    await user.type(input, "sk-abc");
    expect(input.type).toBe("password");
    const toggle = screen.getByRole("button", { name: /show anthropic claude key/i });
    await user.click(toggle);
    expect(input.type).toBe("text");
    await user.click(screen.getByRole("button", { name: /hide anthropic claude key/i }));
    expect(input.type).toBe("password");
  });
});
