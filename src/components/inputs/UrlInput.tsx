import { Globe, Link2 } from "lucide-react";
import { useEffect, useId, useState } from "react";

const FAVICON_SERVICE = "https://www.google.com/s2/favicons";

/**
 * Normalize user-typed URL input.
 *
 * - Trims whitespace.
 * - Auto-prepends `https://` if no scheme is present.
 * - Returns the canonical `URL#toString()` form on success.
 * - Returns `null` if the result is not a valid http(s) URL.
 */
export function normalizeUrl(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  const withScheme = /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  let parsed: URL;
  try {
    parsed = new URL(withScheme);
  } catch {
    return null;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return null;
  if (!parsed.hostname?.includes(".")) return null;
  return parsed.toString();
}

/** Build a favicon URL via Google's service. Returns `null` for invalid input. */
export function faviconUrl(raw: string, size = 32): string | null {
  const canonical = normalizeUrl(raw);
  if (!canonical) return null;
  const host = new URL(canonical).hostname;
  return `${FAVICON_SERVICE}?sz=${size}&domain=${encodeURIComponent(host)}`;
}

export interface UrlInputProps {
  /** Controlled value */
  value?: string;
  /** Controlled setter (receives raw typed string). */
  onChangeRaw?: (value: string) => void;
  /** Fires with the normalized URL when valid, or `null` when invalid/empty. */
  onValidChange?: (value: string | null) => void;
  placeholder?: string;
  label?: string;
  className?: string;
}

export function UrlInput({
  value,
  onChangeRaw,
  onValidChange,
  placeholder = "https://…",
  label,
  className = "",
}: UrlInputProps) {
  const [local, setLocal] = useState("");
  const raw = value ?? local;
  const setRaw = (next: string) => {
    if (onChangeRaw) onChangeRaw(next);
    else setLocal(next);
  };

  const id = useId();
  const canonical = normalizeUrl(raw);
  const favicon = canonical ? faviconUrl(canonical) : null;
  const showError = raw.trim().length > 0 && canonical === null;

  useEffect(() => {
    onValidChange?.(canonical);
  }, [canonical, onValidChange]);

  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      {label ? (
        <label
          htmlFor={id}
          className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
        >
          {label}
        </label>
      ) : null}

      <div className="flex items-center gap-2 rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 px-3 py-2 focus-within:border-accent-500 focus-within:ring-1 focus-within:ring-accent-400">
        {favicon ? (
          <img src={favicon} alt="Favicon" className="h-4 w-4 shrink-0 rounded-[2px]" />
        ) : canonical ? (
          <Globe
            className="h-4 w-4 shrink-0 text-neutral-dark-400"
            strokeWidth={1.5}
            aria-hidden="true"
          />
        ) : (
          <Link2
            className="h-4 w-4 shrink-0 text-neutral-dark-500"
            strokeWidth={1.5}
            aria-hidden="true"
          />
        )}
        <input
          id={id}
          type="url"
          value={raw}
          onChange={(e) => setRaw(e.currentTarget.value)}
          placeholder={placeholder}
          className="w-full bg-transparent text-neutral-dark-100 placeholder:text-neutral-dark-400 focus:outline-none"
        />
      </div>

      {showError ? (
        <div role="alert" className="font-mono text-2xs text-rose-400 tracking-label">
          Invalid URL
        </div>
      ) : canonical ? (
        <div className="truncate font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
          → {canonical}
        </div>
      ) : null}
    </div>
  );
}
