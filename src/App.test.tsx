import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "@/App";

describe("App", () => {
  it("renders the app shell with prompt heading", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: /describe what to build/i })).toBeInTheDocument();
  });

  it("renders the sidebar with wordmark", () => {
    render(<App />);
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
  });

  it("renders the status bar", () => {
    render(<App />);
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
  });
});
