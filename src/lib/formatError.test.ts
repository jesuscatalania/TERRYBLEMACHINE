import { describe, expect, it } from "vitest";
import { formatError } from "@/lib/formatError";

describe("formatError", () => {
  it("extracts .message from Error instances", () => {
    expect(formatError(new Error("boom"))).toBe("boom");
  });

  it("returns strings unchanged", () => {
    expect(formatError("not found")).toBe("not found");
  });

  it("reads Tauri IPC error shape { kind, detail }", () => {
    expect(formatError({ kind: "router", detail: "connection timeout" })).toBe(
      "router: connection timeout",
    );
  });

  it("handles unit-variant IPC errors (kind only)", () => {
    expect(formatError({ kind: "no-output" })).toBe("no-output");
  });

  it("falls back to .message on plain objects with a message field", () => {
    expect(formatError({ message: "custom error" })).toBe("custom error");
  });

  it("never returns [object Object] for unknown objects", () => {
    const result = formatError({ foo: "bar" });
    expect(result).not.toBe("[object Object]");
  });

  it("stringifies primitives sensibly", () => {
    expect(formatError(42)).toBe("42");
    expect(formatError(null)).toBe("null");
    expect(formatError(undefined)).toBe("undefined");
  });
});
