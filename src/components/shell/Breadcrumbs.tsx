export interface BreadcrumbsProps {
  parts: readonly string[];
  className?: string;
}

export function Breadcrumbs({ parts, className = "" }: BreadcrumbsProps) {
  const lastIdx = parts.length - 1;
  return (
    <nav
      aria-label="Breadcrumb"
      className={`flex items-center gap-2 font-mono text-[11px] text-neutral-dark-400 uppercase tracking-label ${className}`}
    >
      {parts.map((part, idx) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: breadcrumb parts are positional by design
        <span key={idx} className="flex items-center gap-2">
          <span
            aria-current={idx === lastIdx ? "page" : undefined}
            className={idx === lastIdx ? "text-neutral-dark-100" : undefined}
          >
            {part}
          </span>
          {idx < lastIdx ? (
            <span aria-hidden="true" className="text-neutral-dark-500">
              /
            </span>
          ) : null}
        </span>
      ))}
    </nav>
  );
}
