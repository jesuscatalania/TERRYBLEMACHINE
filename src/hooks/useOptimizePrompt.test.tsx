import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/optimizeCommands", () => ({
  optimizePrompt: vi.fn(),
}));

import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";
import { optimizePrompt } from "@/lib/optimizeCommands";

describe("useOptimizePrompt", () => {
  it("starts with enabled=false, busy=false, canUndo=false", () => {
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value: "x", setValue: () => {} }),
    );
    expect(result.current.enabled).toBe(false);
    expect(result.current.busy).toBe(false);
    expect(result.current.canUndo).toBe(false);
  });

  it("optimize replaces value + arms undo + returns the optimized string", async () => {
    let value = "a sunset";
    const setValue = vi.fn((next: string) => {
      value = next;
    });
    vi.mocked(optimizePrompt).mockResolvedValueOnce("warm sunset over berlin, 35mm film grain");
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value, setValue }),
    );
    let returned: string | undefined;
    await act(async () => {
      returned = await result.current.optimize();
    });
    expect(setValue).toHaveBeenCalledWith("warm sunset over berlin, 35mm film grain");
    expect(returned).toBe("warm sunset over berlin, 35mm film grain");
    expect(result.current.canUndo).toBe(true);
  });

  it("undo restores the previous value + clears canUndo", async () => {
    const setValue = vi.fn();
    vi.mocked(optimizePrompt).mockResolvedValueOnce("optimized");
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value: "original", setValue }),
    );
    await act(async () => {
      await result.current.optimize();
    });
    act(() => {
      result.current.undo();
    });
    expect(setValue).toHaveBeenLastCalledWith("original");
    expect(result.current.canUndo).toBe(false);
  });

  it("optimize failure leaves value unchanged + no undo armed", async () => {
    const setValue = vi.fn();
    vi.mocked(optimizePrompt).mockRejectedValueOnce(new Error("boom"));
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value: "x", setValue }),
    );
    await act(async () => {
      await expect(result.current.optimize()).rejects.toThrow("boom");
    });
    expect(setValue).not.toHaveBeenCalled();
    expect(result.current.canUndo).toBe(false);
  });

  it("ignores optimize calls while already busy", async () => {
    vi.mocked(optimizePrompt).mockClear();
    const setValue = vi.fn();
    let resolveFirst: (v: string) => void = () => {};
    vi.mocked(optimizePrompt).mockImplementationOnce(
      () =>
        new Promise((r) => {
          resolveFirst = r;
        }),
    );
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value: "x", setValue }),
    );
    act(() => {
      void result.current.optimize();
    });
    // second call while first is still pending
    await act(async () => {
      await result.current.optimize();
    });
    expect(optimizePrompt).toHaveBeenCalledTimes(1);
    await act(async () => {
      resolveFirst("done");
    });
  });
});
