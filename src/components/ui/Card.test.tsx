import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Card, CardBody, CardFooter, CardHeader } from "@/components/ui/Card";

describe("Card", () => {
  it("renders header/body/footer slots", () => {
    render(
      <Card>
        <CardHeader>Head</CardHeader>
        <CardBody>Body</CardBody>
        <CardFooter>Foot</CardFooter>
      </Card>,
    );
    expect(screen.getByText("Head")).toBeInTheDocument();
    expect(screen.getByText("Body")).toBeInTheDocument();
    expect(screen.getByText("Foot")).toBeInTheDocument();
  });

  it("renders schematic variant with corner brackets", () => {
    const { container } = render(
      <Card variant="schematic">
        <CardBody>x</CardBody>
      </Card>,
    );
    expect(container.querySelectorAll("[data-bracket]")).toHaveLength(4);
  });
});
