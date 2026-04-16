import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ModuleIcon } from "@/components/shell/ModuleIcon";

describe("ModuleIcon", () => {
  it("renders an svg for each module id", () => {
    for (const id of ["website", "graphic2d", "graphic3d", "video", "typography"] as const) {
      const { container } = render(<ModuleIcon moduleId={id} />);
      expect(container.querySelector("svg")).not.toBeNull();
    }
  });
});
