import { Heart } from "lucide-react";
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

  if (variants.length === 0) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        No logos yet — generate variants above
      </div>
    );
  }

  return (
    <div className="grid grid-cols-3 gap-2 p-3">
      {variants.map((v) => {
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
  );
}
