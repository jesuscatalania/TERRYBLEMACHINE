export const GOOGLE_FONTS = [
  "Inter",
  "Roboto",
  "Open Sans",
  "Poppins",
  "Space Mono",
  "Playfair Display",
  "Merriweather",
  "Oswald",
  "Raleway",
  "Montserrat",
  "Lato",
  "Source Code Pro",
  "Nunito",
  "Work Sans",
  "Fira Sans",
  "Crimson Text",
  "Libre Baskerville",
  "Abril Fatface",
  "IBM Plex Sans",
  "IBM Plex Mono",
  "DM Sans",
  "DM Mono",
  "Bebas Neue",
  "Anton",
  "Archivo",
  "Rubik",
  "Quicksand",
  "Karla",
  "Inconsolata",
] as const;

export type GoogleFont = (typeof GOOGLE_FONTS)[number];

/**
 * Idempotently inject a Google Fonts stylesheet <link> into <head> and wait
 * for the browser to actually load the font before resolving. Safe to call
 * repeatedly — the `id` check prevents duplicate <link> insertions.
 *
 * Awaits `document.fonts.load()` so the caller can synchronously render with
 * the correct font afterwards instead of flashing the system fallback. See
 * FU #105.
 */
export async function injectGoogleFont(name: GoogleFont): Promise<void> {
  if (typeof document === "undefined") return;
  const id = `gfont-${name.replace(/\s+/g, "-")}`;
  if (!document.getElementById(id)) {
    const link = document.createElement("link");
    link.id = id;
    link.rel = "stylesheet";
    link.href = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(name)}:wght@400;700&display=swap`;
    document.head.appendChild(link);
  }
  // `document.fonts.load` resolves once the font face is ready for rendering.
  // Supported in all modern browsers + the WKWebView Tauri uses on macOS. A
  // failure (network hiccup, unknown family) is non-fatal — we'd just fall
  // back to the system font, same as before.
  if (document.fonts?.load) {
    try {
      await document.fonts.load(`16px "${name}"`);
    } catch {
      // Ignore — caller renders with the system fallback.
    }
  }
}
