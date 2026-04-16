import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { StatusBar } from "@/components/shell/StatusBar";
import { useAiStore } from "@/stores/aiStore";

describe("StatusBar", () => {
  beforeEach(() => {
    useAiStore.setState({
      budget: { usedCents: 0, limitCents: 5000, periodStartedAt: "2026-04-16T00:00:00Z" },
      cache: { hits: 0, misses: 0, size: 0 },
      activeRequests: [],
    });
  });

  it("renders AI · IDLE when no active requests", () => {
    render(<StatusBar />);
    expect(screen.getByText(/AI · IDLE/)).toBeInTheDocument();
  });

  it("renders AI · ACTIVE when there are active requests", () => {
    useAiStore.setState({
      budget: { usedCents: 0, limitCents: 5000, periodStartedAt: "2026-04-16T00:00:00Z" },
      cache: { hits: 0, misses: 0, size: 0 },
      activeRequests: [{ id: "r1", provider: "claude", task: "test" }],
    });
    render(<StatusBar />);
    expect(screen.getByText(/AI · ACTIVE/)).toBeInTheDocument();
  });

  it("formats budget in dollars with two decimals", () => {
    useAiStore.setState({
      budget: { usedCents: 1234, limitCents: 5000, periodStartedAt: "2026-04-16T00:00:00Z" },
      cache: { hits: 0, misses: 0, size: 0 },
      activeRequests: [],
    });
    render(<StatusBar />);
    expect(screen.getByText(/\$12\.34 \/ \$50\.00/)).toBeInTheDocument();
  });

  it("shows cache size / capacity", () => {
    useAiStore.setState({
      budget: { usedCents: 0, limitCents: 5000, periodStartedAt: "2026-04-16T00:00:00Z" },
      cache: { hits: 0, misses: 0, size: 412 },
      activeRequests: [],
    });
    render(<StatusBar cacheCapacity={500} />);
    expect(screen.getByText(/412 \/ 500/)).toBeInTheDocument();
  });

  it("renders queue count matching active requests", () => {
    useAiStore.setState({
      budget: { usedCents: 0, limitCents: 5000, periodStartedAt: "2026-04-16T00:00:00Z" },
      cache: { hits: 0, misses: 0, size: 0 },
      activeRequests: [
        { id: "1", provider: "claude", task: "a" },
        { id: "2", provider: "fal", task: "b" },
      ],
    });
    const { getByRole } = render(<StatusBar />);
    expect(getByRole("contentinfo")).toHaveTextContent(/QUEUE\s*2/);
  });

  it("renders progress percentage when renderProgress is given", () => {
    render(<StatusBar renderProgress={0.34} />);
    expect(screen.getByText("34%")).toBeInTheDocument();
  });

  it("hides progress when renderProgress is undefined", () => {
    render(<StatusBar />);
    expect(screen.queryByText(/RENDER/)).toBeNull();
  });
});
