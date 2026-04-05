const assetStatuses = ["IN_STOCK", "ASSIGNED", "RETIRED", "DISPOSED"] as const;

type AssetsFilterBarProps = {
  filters: {
    q: string;
    status: string;
    assetType: string;
  };
  onCreateRequested?: () => void;
  onPreloadRequested?: () => void;
};

export function AssetsFilterBar({
  filters,
  onCreateRequested,
  onPreloadRequested,
}: AssetsFilterBarProps) {
  return (
    <div className="flex flex-col gap-4 rounded-[24px] border border-border bg-surface-strong px-5 py-5">
      <div className="grid gap-3 lg:grid-cols-[minmax(0,1.5fr)_220px_220px]">
        <label className="space-y-2 text-sm text-muted">
          <span className="block font-medium text-foreground">Search assets</span>
          <input
            className="w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
            defaultValue={filters.q}
            name="q"
            placeholder="Search asset code or asset name..."
            type="search"
          />
        </label>

        <label className="space-y-2 text-sm text-muted">
          <span className="block font-medium text-foreground">Status</span>
          <select
            className="w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
            defaultValue={filters.status}
            name="status"
          >
            <option value="">All statuses</option>
            {assetStatuses.map((status) => (
              <option key={status} value={status}>
                {status}
              </option>
            ))}
          </select>
        </label>

        <label className="space-y-2 text-sm text-muted">
          <span className="block font-medium text-foreground">Asset type</span>
          <input
            className="w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
            defaultValue={filters.assetType}
            name="assetType"
            placeholder="Laptop, Dock, Monitor..."
            type="text"
          />
        </label>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <button
            className="rounded-full bg-accent px-5 py-2.5 text-sm font-semibold text-accent-foreground transition hover:brightness-110"
            type="submit"
          >
            Apply Filters
          </button>
          <a
            className="rounded-full border border-border bg-surface px-5 py-2.5 text-sm font-medium text-muted transition hover:border-accent hover:text-accent"
            href="/assets"
          >
            Reset
          </a>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <button
            className="rounded-full border border-border bg-surface px-5 py-2.5 text-sm font-semibold text-foreground transition hover:border-accent hover:text-accent"
            onClick={onCreateRequested}
            type="button"
          >
            New Asset
          </button>
          <button
            className="rounded-full border border-border bg-surface px-5 py-2.5 text-sm font-semibold text-foreground transition hover:border-accent hover:text-accent"
            onClick={onPreloadRequested}
            type="button"
          >
            Preload Assets
          </button>
        </div>
      </div>
    </div>
  );
}
