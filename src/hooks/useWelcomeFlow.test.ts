import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useWelcomeFlow, WELCOME_LOCALSTORAGE_KEY } from "@/hooks/useWelcomeFlow";

describe("useWelcomeFlow", () => {
  afterEach(() => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
  });

  it("opens by default when no flag is set", () => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    const { result } = renderHook(() => useWelcomeFlow());
    expect(result.current.open).toBe(true);
  });

  it("stays closed when flag is set", () => {
    window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
    const { result } = renderHook(() => useWelcomeFlow());
    expect(result.current.open).toBe(false);
  });

  it("dismiss() sets flag and closes", () => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    const { result } = renderHook(() => useWelcomeFlow());
    act(() => result.current.dismiss());
    expect(result.current.open).toBe(false);
    expect(window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY)).toBe("true");
  });
});
