import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// Stub Sandpack so the test for JSX routing doesn't try to boot the
// real in-browser bundler under jsdom.
vi.mock("@codesandbox/sandpack-react", () => ({
  Sandpack: () => <div data-testid="sandpack-stub" />,
}));

import { composeHtml, DevicePreview } from "@/components/website/DevicePreview";

const html = (content: string) => ({ path: "index.html", content });
const css = (content: string) => ({ path: "styles.css", content });

describe("composeHtml", () => {
  it("returns a fallback when no html file is present", () => {
    const out = composeHtml([{ path: "main.js", content: "" }]);
    expect(out).toContain("No index.html");
  });

  it("returns the single html file verbatim when no css is present", () => {
    const out = composeHtml([html("<html><body>Hi</body></html>")]);
    expect(out).toBe("<html><body>Hi</body></html>");
  });

  it("inlines css before </head>", () => {
    const out = composeHtml([
      html("<html><head><title>t</title></head><body></body></html>"),
      css("body { color: red; }"),
    ]);
    expect(out).toContain("<style");
    expect(out).toContain("body { color: red; }");
    expect(out).toMatch(/<style[^>]*>.*<\/style>\s*<\/head>/s);
  });

  it("prepends styles when no head exists", () => {
    const out = composeHtml([html("<body>Hi</body>"), css("a { }")]);
    expect(out.startsWith("<style")).toBe(true);
    expect(out).toContain("<body>Hi</body>");
  });
});

describe("DevicePreview", () => {
  it("renders an iframe and the device label", () => {
    render(<DevicePreview files={[html("<html><body>x</body></html>")]} device="mobile" />);
    expect(screen.getByTestId("device-preview-iframe")).toBeInTheDocument();
    expect(screen.getByText(/mobile/i)).toBeInTheDocument();
    expect(screen.getByText(/375px/)).toBeInTheDocument();
  });

  it("routes .jsx projects to Sandpack instead of the iframe", () => {
    render(
      <DevicePreview
        files={[{ path: "src/App.jsx", content: "export default () => null;" }]}
        device="desktop"
      />,
    );
    expect(screen.getByTestId("device-preview-sandpack")).toBeInTheDocument();
    expect(screen.queryByTestId("device-preview-iframe")).not.toBeInTheDocument();
  });
});
