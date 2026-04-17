import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { faviconUrl, normalizeUrl, UrlInput } from "@/components/inputs/UrlInput";

describe("normalizeUrl", () => {
  it("auto-prepends https:// when no scheme", () => {
    expect(normalizeUrl("stripe.com")).toBe("https://stripe.com/");
  });

  it("keeps http:// as-is", () => {
    expect(normalizeUrl("http://example.com/path")).toBe("http://example.com/path");
  });

  it("preserves explicit https://", () => {
    expect(normalizeUrl("https://foo.dev")).toBe("https://foo.dev/");
  });

  it("returns null for obviously invalid input", () => {
    expect(normalizeUrl("")).toBeNull();
    expect(normalizeUrl("   ")).toBeNull();
    expect(normalizeUrl("not a url")).toBeNull();
    expect(normalizeUrl("javascript:alert(1)")).toBeNull();
  });
});

describe("faviconUrl", () => {
  it("returns Google's favicon service URL for a valid host", () => {
    const url = faviconUrl("https://stripe.com/path");
    expect(url).toContain("stripe.com");
    expect(url).toContain("google.com/s2/favicons");
  });

  it("returns null for invalid input", () => {
    expect(faviconUrl("nope")).toBeNull();
  });
});

describe("UrlInput", () => {
  it("renders a URL input and accepts typing", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<UrlInput onValidChange={onChange} />);
    await user.type(screen.getByRole("textbox"), "stripe.com");
    expect(onChange).toHaveBeenLastCalledWith("https://stripe.com/");
  });

  it("shows an inline error for obviously invalid input", async () => {
    const user = userEvent.setup();
    render(<UrlInput onValidChange={vi.fn()} />);
    await user.type(screen.getByRole("textbox"), "not a url");
    expect(screen.getByRole("alert")).toHaveTextContent(/invalid/i);
  });

  it("renders a favicon img element for a valid URL", async () => {
    const user = userEvent.setup();
    render(<UrlInput onValidChange={vi.fn()} />);
    await user.type(screen.getByRole("textbox"), "example.com");
    const img = await screen.findByAltText(/favicon/i);
    expect(img).toHaveAttribute("src");
    expect(img.getAttribute("src")).toContain("example.com");
  });
});
