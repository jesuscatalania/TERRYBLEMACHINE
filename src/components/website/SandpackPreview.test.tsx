import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// Stub the Sandpack component so jsdom doesn't try to spin up the
// in-browser bundler (which relies on service workers and iframes
// jsdom refuses to drive). We only care that the wrapper resolves
// template + files correctly and hands them down.
vi.mock("@codesandbox/sandpack-react", () => ({
  Sandpack: ({ template, files }: { template: string; files: Record<string, string> }) => (
    <div
      data-testid="sandpack-stub"
      data-template={template}
      data-file-count={Object.keys(files).length}
    />
  ),
}));

import {
  guessTemplate,
  inferDependencies,
  SandpackPreview,
  shouldUseSandpack,
} from "@/components/website/SandpackPreview";

describe("guessTemplate", () => {
  it("picks vite-react-ts when any .tsx file is present", () => {
    expect(
      guessTemplate([
        { path: "package.json", content: "{}" },
        { path: "src/App.tsx", content: "export default () => null;" },
      ]),
    ).toBe("vite-react-ts");
  });

  it("picks vite-react when .jsx is present and no .tsx", () => {
    expect(
      guessTemplate([
        { path: "src/App.jsx", content: "export default () => null;" },
        { path: "src/main.js", content: "" },
      ]),
    ).toBe("vite-react");
  });

  it("picks vite when only a vite.config.* is present", () => {
    expect(
      guessTemplate([
        { path: "vite.config.ts", content: "export default {};" },
        { path: "src/main.js", content: "" },
      ]),
    ).toBe("vite");
  });

  it("falls back to static for plain html", () => {
    expect(
      guessTemplate([
        { path: "index.html", content: "<html></html>" },
        { path: "styles.css", content: "body {}" },
      ]),
    ).toBe("static");
  });
});

describe("inferDependencies", () => {
  it("extracts dependencies from a package.json", () => {
    const deps = inferDependencies([
      {
        path: "package.json",
        content: JSON.stringify({
          dependencies: { react: "^19.0.0", three: "^0.184.0" },
        }),
      },
    ]);
    expect(deps).toEqual({ react: "^19.0.0", three: "^0.184.0" });
  });

  it("returns empty object when no package.json is present", () => {
    expect(inferDependencies([{ path: "index.html", content: "" }])).toEqual({});
  });

  it("returns empty object when package.json is malformed", () => {
    expect(inferDependencies([{ path: "package.json", content: "{not valid" }])).toEqual({});
  });
});

describe("shouldUseSandpack", () => {
  it("routes TSX files to Sandpack", () => {
    expect(shouldUseSandpack([{ path: "src/App.tsx", content: "" }])).toBe(true);
  });

  it("routes JSX files to Sandpack", () => {
    expect(shouldUseSandpack([{ path: "src/App.jsx", content: "" }])).toBe(true);
  });

  it("routes a package.json with react to Sandpack", () => {
    expect(
      shouldUseSandpack([
        {
          path: "package.json",
          content: JSON.stringify({ dependencies: { react: "^19.0.0" } }),
        },
      ]),
    ).toBe(true);
  });

  it("keeps plain HTML+CSS on the static iframe path", () => {
    expect(
      shouldUseSandpack([
        { path: "index.html", content: "<html></html>" },
        { path: "styles.css", content: "" },
      ]),
    ).toBe(false);
  });
});

describe("SandpackPreview", () => {
  it("renders Sandpack with the vite-react-ts template for TSX projects", () => {
    const { getByTestId } = render(
      <SandpackPreview
        files={[
          {
            path: "package.json",
            content: JSON.stringify({ dependencies: { react: "^19.0.0" } }),
          },
          { path: "src/App.tsx", content: "export default () => null;" },
        ]}
        device="desktop"
      />,
    );
    const stub = getByTestId("sandpack-stub");
    expect(stub.getAttribute("data-template")).toBe("vite-react-ts");
    // package.json + App.tsx = 2 files
    expect(stub.getAttribute("data-file-count")).toBe("2");
  });

  it("renders Sandpack with the vite-react template for JSX-only projects", () => {
    const { getByTestId } = render(
      <SandpackPreview
        files={[{ path: "src/App.jsx", content: "export default () => null;" }]}
        device="mobile"
      />,
    );
    expect(getByTestId("sandpack-stub").getAttribute("data-template")).toBe("vite-react");
  });

  it("renders Sandpack with the static template for plain HTML projects", () => {
    const { getByTestId } = render(
      <SandpackPreview files={[{ path: "index.html", content: "<h1>Hi</h1>" }]} device="desktop" />,
    );
    expect(getByTestId("sandpack-stub").getAttribute("data-template")).toBe("static");
  });
});
