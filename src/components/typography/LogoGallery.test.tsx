import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LogoGallery } from "@/components/typography/LogoGallery";
import type { LogoVariant } from "@/lib/logoCommands";
import { useLogoStore } from "@/stores/logoStore";

function sample(): LogoVariant[] {
  return [
    { url: "https://example.com/a.png", local_path: null, seed: 1, model: "ideogram-v3" },
    { url: "https://example.com/b.png", local_path: null, seed: 2, model: "ideogram-v3" },
  ];
}

describe("LogoGallery", () => {
  beforeEach(() => useLogoStore.getState().clearFavorites());

  it("renders empty state when no variants", () => {
    render(<LogoGallery variants={[]} selectedUrl={null} onSelect={() => {}} />);
    expect(screen.getByText(/No logos yet/i)).toBeInTheDocument();
  });

  it("renders each variant", () => {
    render(<LogoGallery variants={sample()} selectedUrl={null} onSelect={() => {}} />);
    expect(screen.getByTestId("logo-variant-https://example.com/a.png")).toBeInTheDocument();
    expect(screen.getByTestId("logo-variant-https://example.com/b.png")).toBeInTheDocument();
  });

  it("calls onSelect when variant image is clicked", () => {
    const onSelect = vi.fn();
    render(<LogoGallery variants={sample()} selectedUrl={null} onSelect={onSelect} />);
    fireEvent.click(screen.getAllByLabelText(/select logo variant/i)[0] as HTMLElement);
    expect(onSelect).toHaveBeenCalledWith("https://example.com/a.png");
  });

  it("toggles favorite in store when heart is clicked", () => {
    render(<LogoGallery variants={sample()} selectedUrl={null} onSelect={() => {}} />);
    expect(useLogoStore.getState().isFavorite("https://example.com/a.png")).toBe(false);
    fireEvent.click(screen.getAllByLabelText(/^favorite$/i)[0] as HTMLElement);
    expect(useLogoStore.getState().isFavorite("https://example.com/a.png")).toBe(true);
  });

  it('"Show favorites only" filter hides non-favorites and restores them on toggle off', () => {
    render(<LogoGallery variants={sample()} selectedUrl={null} onSelect={() => {}} />);
    // Favorite only the first variant.
    fireEvent.click(screen.getAllByLabelText(/^favorite$/i)[0] as HTMLElement);
    expect(useLogoStore.getState().isFavorite("https://example.com/a.png")).toBe(true);

    // Enable the filter — only the favorited variant should remain.
    fireEvent.click(screen.getByRole("button", { name: /show favorites only/i }));
    expect(screen.getByTestId("logo-variant-https://example.com/a.png")).toBeInTheDocument();
    expect(screen.queryByTestId("logo-variant-https://example.com/b.png")).toBeNull();

    // Toggle back off — both variants render again.
    fireEvent.click(screen.getByRole("button", { name: /show all/i }));
    expect(screen.getByTestId("logo-variant-https://example.com/a.png")).toBeInTheDocument();
    expect(screen.getByTestId("logo-variant-https://example.com/b.png")).toBeInTheDocument();
  });

  it("favorites-only with no favorites shows the empty-state hint", () => {
    render(<LogoGallery variants={sample()} selectedUrl={null} onSelect={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /show favorites only/i }));
    expect(screen.getByText(/no favorites yet/i)).toBeInTheDocument();
  });

  it("renders 6 skeleton tiles when busy and no variants", () => {
    const { container } = render(
      <LogoGallery variants={[]} selectedUrl={null} onSelect={() => {}} busy={true} />,
    );
    const skeletons = container.querySelectorAll("[data-skeleton='true']");
    expect(skeletons).toHaveLength(6);
  });

  it("renders 'No logos yet' when not busy and no variants", () => {
    render(<LogoGallery variants={[]} selectedUrl={null} onSelect={() => {}} busy={false} />);
    expect(screen.getByText(/No logos yet/i)).toBeInTheDocument();
  });
});
