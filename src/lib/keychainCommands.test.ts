import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  deleteApiKey,
  getApiKey,
  isKeyStoreIpcError,
  listApiKeys,
  storeApiKey,
} from "@/lib/keychainCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("keychainCommands", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("storeApiKey forwards service + key to the IPC layer", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await storeApiKey("claude", "sk-xxx");
    expect(invokeMock).toHaveBeenCalledWith("store_api_key", {
      service: "claude",
      key: "sk-xxx",
    });
  });

  it("getApiKey returns the stored key", async () => {
    invokeMock.mockResolvedValueOnce("sk-xxx");
    const key = await getApiKey("claude");
    expect(key).toBe("sk-xxx");
    expect(invokeMock).toHaveBeenCalledWith("get_api_key", { service: "claude" });
  });

  it("deleteApiKey forwards service to the IPC layer", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await deleteApiKey("fal");
    expect(invokeMock).toHaveBeenCalledWith("delete_api_key", { service: "fal" });
  });

  it("listApiKeys returns the list of service ids with stored keys", async () => {
    invokeMock.mockResolvedValueOnce(["claude", "fal"]);
    const list = await listApiKeys();
    expect(list).toEqual(["claude", "fal"]);
    expect(invokeMock).toHaveBeenCalledWith("list_api_keys");
  });

  describe("isKeyStoreIpcError", () => {
    it("returns true for a well-formed IPC error", () => {
      expect(isKeyStoreIpcError({ kind: "NotFound", detail: "no key stored" })).toBe(true);
    });

    it("returns false for a plain Error", () => {
      expect(isKeyStoreIpcError(new Error("boom"))).toBe(false);
    });

    it("returns false for null / undefined / primitives", () => {
      expect(isKeyStoreIpcError(null)).toBe(false);
      expect(isKeyStoreIpcError(undefined)).toBe(false);
      expect(isKeyStoreIpcError("NotFound")).toBe(false);
    });

    it("returns false when the shape is missing fields", () => {
      expect(isKeyStoreIpcError({ kind: "NotFound" })).toBe(false);
      expect(isKeyStoreIpcError({ detail: "x" })).toBe(false);
      expect(isKeyStoreIpcError({ kind: 1, detail: "x" })).toBe(false);
    });
  });
});
