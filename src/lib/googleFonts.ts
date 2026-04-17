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
 * Idempotently inject a Google Fonts stylesheet <link> into <head>. Safe to
 * call repeatedly — the `id` check prevents duplicate insertions.
 */
export function injectGoogleFont(name: GoogleFont): void {
  if (typeof document === "undefined") return;
  const id = `gfont-${name.replace(/\s+/g, "-")}`;
  if (document.getElementById(id)) return;
  const link = document.createElement("link");
  link.id = id;
  link.rel = "stylesheet";
  link.href = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(name)}:wght@400;700&display=swap`;
  document.head.appendChild(link);
}
