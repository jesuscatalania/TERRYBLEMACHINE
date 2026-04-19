import { describe, expect, it } from "vitest";
import { serializableMockOnly } from "./invoke-mock";

describe("serializableMockOnly", () => {
  it("passes through static values", () => {
    const result = serializableMockOnly({
      foo: 1,
      bar: { nested: true },
      baz: [1, 2, 3],
    });
    expect(result).toEqual({ foo: 1, bar: { nested: true }, baz: [1, 2, 3] });
  });

  it("throws when a value is a function", () => {
    expect(() => serializableMockOnly({ cmd: () => "dynamic" })).toThrow(
      /command "cmd" is a function/,
    );
  });

  it("throws message names the offending command", () => {
    expect(() => serializableMockOnly({ a: 1, b: () => 2, c: 3 })).toThrow(/command "b"/);
  });
});
