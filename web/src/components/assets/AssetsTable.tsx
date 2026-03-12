export type AssetTableRow = {
  id: number;
  assetCode: string;
  name: string;
  assetType: string;
  status: string;
  brand?: string | null | undefined;
  modelName?: string | null | undefined;
  notes?: string | null | undefined;
  owningUnit: string | null | undefined;
  managingUnit: string | null | undefined;
  serialNumber: string | null | undefined;
  recordedAt: Date;
  retiredAt?: Date | null | undefined;
  disposedAt?: Date | null | undefined;
  assignments: Array<{
    employee: {
      employeeId: string;
      fullName: string;
    };
  }>;
};

type AssetsTableProps = {
  assets: AssetTableRow[];
  onEdit?: (asset: AssetTableRow) => void;
};

const statusTone: Record<string, string> = {
  IN_STOCK: "text-accent",
  ASSIGNED: "text-foreground",
  RETIRED: "text-amber-500",
  DISPOSED: "text-rose-500",
};

function formatRecordedAt(value: Date) {
  return value.toISOString().slice(0, 10);
}

function getCurrentHolder(asset: AssetTableRow) {
  const holder = asset.assignments[0]?.employee;

  if (!holder) {
    return "Not assigned";
  }

  return `${holder.employeeId} - ${holder.fullName}`;
}

export function AssetsTable({ assets, onEdit }: AssetsTableProps) {
  return (
    <div className="overflow-hidden rounded-[24px] border border-border bg-surface-strong">
      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-border text-sm">
          <thead className="bg-surface">
            <tr className="text-left text-xs uppercase tracking-[0.18em] text-muted">
              <th className="px-4 py-4 font-medium">Asset Code</th>
              <th className="px-4 py-4 font-medium">Name</th>
              <th className="px-4 py-4 font-medium">Asset Type</th>
              <th className="px-4 py-4 font-medium">Status</th>
              <th className="px-4 py-4 font-medium">Owning Unit</th>
              <th className="px-4 py-4 font-medium">Managing Unit</th>
              <th className="px-4 py-4 font-medium">Serial Number</th>
              <th className="px-4 py-4 font-medium">Current Holder</th>
              <th className="px-4 py-4 font-medium">Recorded At</th>
              <th className="px-4 py-4 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {assets.length === 0 ? (
              <tr>
                <td className="px-4 py-6 text-sm text-muted" colSpan={10}>
                  No assets match the current filters.
                </td>
              </tr>
            ) : (
              assets.map((asset) => (
                <tr key={asset.id} className="align-top">
                  <td className="px-4 py-4 font-mono text-xs text-muted">{asset.assetCode}</td>
                  <td className="px-4 py-4 font-medium text-foreground">{asset.name}</td>
                  <td className="px-4 py-4 text-muted">{asset.assetType}</td>
                  <td className={`px-4 py-4 font-semibold ${statusTone[asset.status] ?? "text-foreground"}`}>
                    {asset.status}
                  </td>
                  <td className="px-4 py-4 text-muted">{asset.owningUnit ?? "-"}</td>
                  <td className="px-4 py-4 text-muted">{asset.managingUnit ?? "-"}</td>
                  <td className="px-4 py-4 text-muted">{asset.serialNumber ?? "-"}</td>
                  <td className="px-4 py-4 text-muted">{getCurrentHolder(asset)}</td>
                  <td className="px-4 py-4 text-muted">{formatRecordedAt(asset.recordedAt)}</td>
                  <td className="px-4 py-4">
                    <button
                      className="rounded-full border border-border bg-surface px-4 py-2 text-xs font-semibold text-foreground transition hover:border-accent hover:text-accent"
                      onClick={() => onEdit?.(asset)}
                      type="button"
                    >
                      Edit
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
