import { type InputHTMLAttributes, type TextareaHTMLAttributes, useEffect, useRef } from "react";

const INPUT_BASE =
  "w-full rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 px-3 py-2 text-sm text-neutral-dark-100 placeholder:text-neutral-dark-400 focus:border-accent-500 focus:outline-none focus-visible:ring-1 focus-visible:ring-accent-400 disabled:cursor-not-allowed disabled:opacity-50";

// ─── Shared label/error wrapper ────────────────────────────────────────

interface FieldWrapperProps {
  id?: string;
  label?: string;
  error?: string;
  children: React.ReactNode;
}

function FieldWrapper({ id, label, error, children }: FieldWrapperProps) {
  if (!label && !error) return <>{children}</>;
  return (
    <div className="flex flex-col gap-1.5">
      {label ? (
        <label
          htmlFor={id}
          className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
        >
          {label}
        </label>
      ) : null}
      {children}
      {error ? (
        <span role="alert" className="font-mono text-2xs text-rose-400">
          {error}
        </span>
      ) : null}
    </div>
  );
}

// ─── Text Input ────────────────────────────────────────────────────────

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  /** Optional label rendered above the input. */
  label?: string;
  /** Error text rendered below the input. */
  error?: string;
  /** Convenience callback with just the string value. */
  onValueChange?: (value: string) => void;
  /** Raw React onChange, also supported. */
  onChange?: InputHTMLAttributes<HTMLInputElement>["onChange"];
}

export function Input({
  label,
  error,
  onValueChange,
  onChange,
  className = "",
  type = "text",
  id,
  ...rest
}: InputProps) {
  return (
    <FieldWrapper id={id} label={label} error={error}>
      <input
        id={id}
        type={type}
        className={`${INPUT_BASE} ${className}`}
        onChange={(e) => {
          onChange?.(e);
          onValueChange?.(e.currentTarget.value);
        }}
        {...rest}
      />
    </FieldWrapper>
  );
}

// ─── Number Input ──────────────────────────────────────────────────────

export interface NumberInputProps extends InputProps {
  min?: number;
  max?: number;
  step?: number;
}

export function NumberInput({ inputMode = "numeric", ...rest }: NumberInputProps) {
  return <Input type="number" inputMode={inputMode} {...rest} />;
}

// ─── Textarea (auto-resize) ────────────────────────────────────────────

export interface TextareaProps
  extends Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "onChange"> {
  label?: string;
  error?: string;
  /** Max height in pixels before scrolling kicks in. Defaults to 320. */
  maxHeight?: number;
  onValueChange?: (value: string) => void;
  onChange?: TextareaHTMLAttributes<HTMLTextAreaElement>["onChange"];
}

export function Textarea({
  label,
  error,
  maxHeight = 320,
  onValueChange,
  onChange,
  className = "",
  id,
  ...rest
}: TextareaProps) {
  const ref = useRef<HTMLTextAreaElement>(null);

  const resize = () => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    const next = Math.min(el.scrollHeight, maxHeight);
    el.style.height = `${next}px`;
    el.style.overflowY = el.scrollHeight > maxHeight ? "auto" : "hidden";
  };

  // Resize on mount + when value prop changes (controlled case)
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentionally re-runs when value/defaultValue change
  useEffect(() => {
    resize();
  }, [rest.value, rest.defaultValue]);

  return (
    <FieldWrapper id={id} label={label} error={error}>
      <textarea
        id={id}
        ref={ref}
        rows={rest.rows ?? 2}
        className={`${INPUT_BASE} resize-none ${className}`}
        onChange={(e) => {
          onChange?.(e);
          onValueChange?.(e.currentTarget.value);
          resize();
        }}
        onInput={resize}
        {...rest}
      />
    </FieldWrapper>
  );
}
