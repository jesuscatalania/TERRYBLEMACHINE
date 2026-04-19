import { type FormEvent, useId, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { Modal } from "@/components/ui/Modal";
import { GOOGLE_FONTS, type GoogleFont } from "@/lib/googleFonts";

/**
 * Shape the dialog hands back on submit. Intentionally omits
 * `logo_svg` + `source_png_path` — those come from the Typography page
 * (SvgEditor + selected variant) and are appended before calling the
 * backend. Keeping them out of the dialog means the dialog can be tested
 * + reused without knowing about Fabric canvases or variant metadata.
 */
export interface BrandKitDialogInput {
  brand_name: string;
  primary_color: string;
  accent_color: string;
  font: string;
  destination: string;
}

export interface BrandKitDialogProps {
  open: boolean;
  onClose: () => void;
  /**
   * Fires with the user-editable fields; Typography page appends
   * `logo_svg` + `source_png_path` before calling `exportBrandKit`.
   * Reject to show an error inline.
   */
  onSubmit: (input: BrandKitDialogInput) => Promise<void>;
  defaultBrandName?: string;
  defaultDestination?: string;
}

// TERRYBLEMACHINE accent-500 (design-system token).
const DEFAULT_PRIMARY = "#e85d2d";
// neutral-dark-950 — jsdom + browsers lowercase `<input type=color>`
// values, so the stored default stays lowercase to avoid snapshot drift.
const DEFAULT_ACCENT = "#0e0e11";

export function BrandKitDialog({
  open,
  onClose,
  onSubmit,
  defaultBrandName = "",
  defaultDestination = "",
}: BrandKitDialogProps) {
  const nameId = useId();
  const fontId = useId();
  const destId = useId();
  const primaryId = useId();
  const accentId = useId();

  const [brandName, setBrandName] = useState(defaultBrandName);
  const [primary, setPrimary] = useState(DEFAULT_PRIMARY);
  const [accent, setAccent] = useState(DEFAULT_ACCENT);
  const [font, setFont] = useState<GoogleFont>("Inter");
  const [destination, setDestination] = useState(defaultDestination);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setBrandName(defaultBrandName);
    setPrimary(DEFAULT_PRIMARY);
    setAccent(DEFAULT_ACCENT);
    setFont("Inter");
    setDestination(defaultDestination);
    setBusy(false);
    setError(null);
  };

  const canSubmit = brandName.trim().length > 0 && destination.trim().length > 0 && !busy;

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await onSubmit({
        brand_name: brandName.trim(),
        primary_color: primary,
        accent_color: accent,
        font,
        destination: destination.trim(),
      });
      reset();
      onClose();
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else if (typeof err === "object" && err !== null && "detail" in err) {
        setError(String((err as { detail: unknown }).detail));
      } else {
        setError("Unknown error exporting brand kit");
      }
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={() => {
        if (!busy) {
          reset();
          onClose();
        }
      }}
      title="Export brand kit"
      maxWidth={520}
      footer={
        <>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              reset();
              onClose();
            }}
            disabled={busy}
          >
            Cancel
          </Button>
          <LoadingButton
            variant="primary"
            size="sm"
            type="submit"
            form="brand-kit-form"
            disabled={brandName.trim().length === 0 || destination.trim().length === 0}
            loading={busy}
          >
            Export
          </LoadingButton>
        </>
      }
    >
      <form id="brand-kit-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Input
          id={nameId}
          label="Brand name"
          placeholder="Acme"
          value={brandName}
          onValueChange={setBrandName}
          disabled={busy}
          autoFocus
        />

        <div className="flex gap-3">
          <label className="flex flex-1 flex-col gap-1.5">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Primary
            </span>
            <input
              id={primaryId}
              aria-label="Primary color"
              type="color"
              value={primary}
              onChange={(e) => setPrimary(e.target.value)}
              disabled={busy}
              className="h-8 w-full cursor-pointer rounded-xs border border-neutral-dark-700 bg-neutral-dark-900"
            />
          </label>
          <label className="flex flex-1 flex-col gap-1.5">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Accent
            </span>
            <input
              id={accentId}
              aria-label="Accent color"
              type="color"
              value={accent}
              onChange={(e) => setAccent(e.target.value)}
              disabled={busy}
              className="h-8 w-full cursor-pointer rounded-xs border border-neutral-dark-700 bg-neutral-dark-900"
            />
          </label>
        </div>

        <div className="flex flex-col gap-1.5">
          <label
            htmlFor={fontId}
            className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
          >
            Font
          </label>
          <select
            id={fontId}
            aria-label="Font"
            value={font}
            onChange={(e) => setFont(e.target.value as GoogleFont)}
            disabled={busy}
            className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-xs text-neutral-dark-100"
          >
            {GOOGLE_FONTS.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
        </div>

        <Input
          id={destId}
          label="Destination directory"
          placeholder="/Users/me/Documents/BrandKit"
          value={destination}
          onValueChange={setDestination}
          disabled={busy}
        />

        {error ? (
          <div
            role="alert"
            className="rounded-xs border border-rose-500/40 bg-rose-500/10 px-3 py-2 font-mono text-2xs text-rose-300"
          >
            {error}
          </div>
        ) : null}
      </form>
    </Modal>
  );
}
