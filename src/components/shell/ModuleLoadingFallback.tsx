import { Skeleton } from "@/components/ui/Skeleton";

/**
 * Fallback shown by `<Suspense>` while a lazy module chunk loads. Mirrors
 * the module shell layout (header tag + brief-row inputs + content area)
 * so the user sees a familiar shape rather than a flashing spinner.
 */
export function ModuleLoadingFallback() {
  return (
    <div
      className="grid h-full grid-rows-[auto_1fr]"
      role="status"
      aria-busy="true"
      aria-live="polite"
    >
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <Skeleton width={140} height={12} />
        <div className="flex items-end gap-2">
          <Skeleton className="flex-1" height={36} />
          <Skeleton width={120} height={36} />
          <Skeleton width={120} height={36} />
        </div>
      </div>
      <div className="p-6">
        <Skeleton className="h-full w-full" />
      </div>
    </div>
  );
}
