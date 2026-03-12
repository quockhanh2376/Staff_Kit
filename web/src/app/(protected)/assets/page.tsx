import { getAssetCatalogPreview } from "@/lib/admin/admin.service";

const statusTone: Record<string, string> = {
  IN_STOCK: "text-accent",
  ASSIGNED: "text-foreground",
  RETIRED: "text-amber-500",
  DISPOSED: "text-rose-500",
};

export default async function AssetsPage() {
  const assets = await getAssetCatalogPreview();

  return (
    <main className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
      <div className="space-y-3">
        <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
          Assets
        </p>
        <h1 className="text-3xl font-semibold tracking-tight">Asset catalog preview</h1>
        <p className="max-w-3xl text-sm leading-6 text-muted">
          Seeded assets are already split across in-stock, assigned, retired, and disposed states.
          This page is the right base for the future CRUD, preload, and asset detail flows.
        </p>
      </div>

      <div className="mt-6 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {assets.map((asset) => (
          <article key={asset.id} className="rounded-[24px] border border-border bg-surface-strong px-5 py-5">
            <div className="flex items-start justify-between gap-4">
              <div>
                <p className="font-mono text-sm text-muted">{asset.assetCode}</p>
                <h2 className="mt-2 text-lg font-semibold text-foreground">{asset.name}</h2>
              </div>
              <span className={`text-sm font-semibold ${statusTone[asset.status] ?? "text-foreground"}`}>
                {asset.status}
              </span>
            </div>

            <div className="mt-4 space-y-2 text-sm text-muted">
              <p>Type: {asset.assetType}</p>
              <p>Managing unit: {asset.managingUnit ?? "-"}</p>
              <p>
                Holder:{" "}
                {asset.assignments[0]
                  ? `${asset.assignments[0].employee.employeeId} - ${asset.assignments[0].employee.fullName}`
                  : "Not assigned"}
              </p>
            </div>
          </article>
        ))}
      </div>
    </main>
  );
}
