import { AssetsPageShell } from "@/components/assets/AssetsPageShell";
import { getAssetCatalogWorkstation } from "@/lib/admin/admin.service";
import { assetListFiltersSchema } from "@/lib/assets/assets.schemas";

type AssetsPageProps = {
  searchParams?: Promise<{
    q?: string;
    status?: string;
    assetType?: string;
  }>;
};

function normalizeSearchParam(value?: string) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

export default async function AssetsPage({ searchParams }: AssetsPageProps) {
  const resolvedSearchParams = searchParams ? await searchParams : undefined;
  const filters = assetListFiltersSchema.parse({
    q: normalizeSearchParam(resolvedSearchParams?.q),
    status: normalizeSearchParam(resolvedSearchParams?.status),
    assetType: normalizeSearchParam(resolvedSearchParams?.assetType),
    take: 100,
  });
  const assets = await getAssetCatalogWorkstation(filters);

  return (
    <AssetsPageShell
      assets={assets}
      filters={{
        q: filters.q ?? "",
        status: filters.status ?? "",
        assetType: filters.assetType ?? "",
      }}
    />
  );
}
