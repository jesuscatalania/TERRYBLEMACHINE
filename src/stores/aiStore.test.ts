import { beforeEach, describe, expect, it } from "vitest";
import { useAiStore } from "@/stores/aiStore";

describe("aiStore", () => {
  beforeEach(() => {
    useAiStore.setState({
      budget: { usedCents: 0, limitCents: 5000, periodStartedAt: "2026-04-16T00:00:00Z" },
      cache: { hits: 0, misses: 0, size: 0 },
      activeRequests: [],
    });
  });

  it("starts with zero spend", () => {
    expect(useAiStore.getState().budget.usedCents).toBe(0);
  });

  it("records spend and accumulates", () => {
    useAiStore.getState().recordSpend(150);
    useAiStore.getState().recordSpend(25);
    expect(useAiStore.getState().budget.usedCents).toBe(175);
  });

  it("reports budget status as 'ok' under 80 percent", () => {
    useAiStore.getState().recordSpend(3000);
    expect(useAiStore.getState().getBudgetStatus()).toBe("ok");
  });

  it("reports 'warn' at or above 80 percent", () => {
    useAiStore.getState().recordSpend(4000);
    expect(useAiStore.getState().getBudgetStatus()).toBe("warn");
  });

  it("reports 'block' at or above 100 percent", () => {
    useAiStore.getState().recordSpend(5000);
    expect(useAiStore.getState().getBudgetStatus()).toBe("block");
  });

  it("records cache hits and misses", () => {
    useAiStore.getState().recordCacheHit();
    useAiStore.getState().recordCacheHit();
    useAiStore.getState().recordCacheMiss();
    const cache = useAiStore.getState().cache;
    expect(cache.hits).toBe(2);
    expect(cache.misses).toBe(1);
  });

  it("tracks active requests by id", () => {
    useAiStore.getState().startRequest({ id: "r1", provider: "claude", task: "generate" });
    useAiStore.getState().startRequest({ id: "r2", provider: "fal", task: "image" });
    expect(useAiStore.getState().activeRequests).toHaveLength(2);
    useAiStore.getState().finishRequest("r1");
    const active = useAiStore.getState().activeRequests;
    expect(active).toHaveLength(1);
    expect(active[0]?.id).toBe("r2");
  });
});
