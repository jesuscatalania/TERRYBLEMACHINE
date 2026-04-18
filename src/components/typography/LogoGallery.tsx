import { Heart } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import type { LogoVariant } from "@/lib/logoCommands";
import { useLogoStore } from "@/stores/logoStore";

export interface LogoGalleryProps {
  variants: LogoVariant[];
  selectedUrl: string | null;
  onSelect: (url: string) => void;
}

export function LogoGallery({ variants, selectedUrl, onSelect }: LogoGalleryProps) {
  const isFavorite = useLogoStore((s) => s.isFavorite);
  const toggleFavorite = useLogoStore((s) => s.toggleFavorite);
  // `favOnly` is gallery-local: the parent page doesn't need to know or
  // care whether the user is filtering, so the state stays internal and
  // the component's prop surface is unchanged.
  const [favOnly, setFavOnly] = useState(false);

  // Reset the filter on every new generation cycle so the user isn't
  // silently staring at an empty grid after re-running Generate with
  // "Show favorites only" still toggled from a prior batch. The
  // `variants` array identity is the trigger — biome wants only values
  // the effect body references, but here the dep IS the trigger, not a
  // value we read. Same escape hatch used in FabricCanvas.tsx / Input.tsx.
  // biome-ignore lint/correctness/useExhaustiveDependencies: variants is the trigger, not a read
  useEffect(() => {
    setFavOnly(false);
  }, [variants]);

  if (variants.length === 0) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        No logos yet — generate variants above
      </div>
    );
  }

  const shown = favOnly ? variants.filter((v) => isFavorite(v.url)) : variants;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center justify-end px-3 pt-3">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setFavOnly((prev) => !prev)}
          aria-pressed={favOnly}
        >
          {favOnly ? "Show all" : "Show favorites only"}
        </Button>
      </div>
      {shown.length === 0 ? (
        <div className="flex flex-1 items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
          No favorites yet — tap the heart on any logo
        </div>
      ) : (
        <div className="grid grid-cols-3 gap-2 p-3">
          {shown.map((v) => {
            const fav = isFavorite(v.url);
            const selected = selectedUrl === v.url;
            return (
              <div
                key={v.url}
                className={`relative rounded-xs border ${selected ? "border-accent-500" : "border-neutral-dark-700"} bg-neutral-dark-900`}
                data-testid={`logo-variant-${v.url}`}
              >
                <button
                  type="button"
                  onClick={() => onSelect(v.url)}
                  className="block aspect-square w-full overflow-hidden"
                  aria-label="Select logo variant"
                >
                  <img src={v.url} alt="" className="h-full w-full object-contain" />
                </button>
                <button
                  type="button"
                  onClick={() => toggleFavorite(v.url)}
                  aria-label={fav ? "Unfavorite" : "Favorite"}
                  className="absolute top-1 right-1 rounded-full bg-neutral-dark-950/80 p-1"
                >
                  <Heart
                    className={`h-3 w-3 ${fav ? "fill-accent-500 text-accent-500" : "text-neutral-dark-400"}`}
                  />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
