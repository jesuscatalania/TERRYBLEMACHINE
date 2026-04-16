import { beforeEach, describe, expect, it } from "vitest";
import { useUiStore } from "@/stores/uiStore";

describe("uiStore", () => {
  beforeEach(() => {
    useUiStore.setState({ modals: [], notifications: [], loadingJobs: 0 });
  });

  it("opens and closes modals by id", () => {
    useUiStore.getState().openModal({ id: "confirm", title: "Delete?" });
    expect(useUiStore.getState().modals).toHaveLength(1);
    useUiStore.getState().closeModal("confirm");
    expect(useUiStore.getState().modals).toHaveLength(0);
  });

  it("does not stack duplicate modals", () => {
    useUiStore.getState().openModal({ id: "confirm", title: "Delete?" });
    useUiStore.getState().openModal({ id: "confirm", title: "Delete?" });
    expect(useUiStore.getState().modals).toHaveLength(1);
  });

  it("adds a notification and exposes it", () => {
    useUiStore.getState().notify({ kind: "success", message: "Saved" });
    const { notifications } = useUiStore.getState();
    expect(notifications).toHaveLength(1);
    expect(notifications[0]?.kind).toBe("success");
    expect(notifications[0]?.id).toBeDefined();
  });

  it("dismisses a notification by id", () => {
    useUiStore.getState().notify({ kind: "info", message: "Hi" });
    const id = useUiStore.getState().notifications[0]?.id;
    if (!id) throw new Error("expected an id");
    useUiStore.getState().dismissNotification(id);
    expect(useUiStore.getState().notifications).toHaveLength(0);
  });

  it("tracks loading via counter so concurrent jobs don't cancel each other", () => {
    useUiStore.getState().startLoading();
    useUiStore.getState().startLoading();
    expect(useUiStore.getState().isLoading()).toBe(true);
    useUiStore.getState().finishLoading();
    expect(useUiStore.getState().isLoading()).toBe(true);
    useUiStore.getState().finishLoading();
    expect(useUiStore.getState().isLoading()).toBe(false);
  });
});
