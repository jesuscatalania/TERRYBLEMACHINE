import { beforeEach, describe, expect, it } from "vitest";
import { useLogoStore } from "@/stores/logoStore";

describe("logoStore", () => {
  beforeEach(() => useLogoStore.getState().clearFavorites());

  it("toggleFavorite adds then removes", () => {
    useLogoStore.getState().toggleFavorite("u1");
    expect(useLogoStore.getState().isFavorite("u1")).toBe(true);
    useLogoStore.getState().toggleFavorite("u1");
    expect(useLogoStore.getState().isFavorite("u1")).toBe(false);
  });

  it("isFavorite returns false for unknown url", () => {
    expect(useLogoStore.getState().isFavorite("nope")).toBe(false);
  });

  it("clearFavorites empties the set", () => {
    useLogoStore.getState().toggleFavorite("u1");
    useLogoStore.getState().toggleFavorite("u2");
    useLogoStore.getState().clearFavorites();
    expect(useLogoStore.getState().isFavorite("u1")).toBe(false);
    expect(useLogoStore.getState().isFavorite("u2")).toBe(false);
  });
});
