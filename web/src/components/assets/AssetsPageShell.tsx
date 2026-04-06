"use client";

import { useState } from "react";

import {
  AssetFormModal,
  type AssetFormAsset,
} from "@/components/assets/AssetFormModal";
import { AssetPreloadModal } from "@/components/assets/AssetPreloadModal";
import { AssetsFilterBar } from "@/components/assets/AssetsFilterBar";
import { AssetsTable, type AssetTableRow } from "@/components/assets/AssetsTable";

type AssetsPageShellProps = {
  assets: AssetTableRow[];
  filters: {
    q: string;
    status: string;
    assetType: string;
  };
};

export function AssetsPageShell({ assets, filters }: AssetsPageShellProps) {
  const [formMode, setFormMode] = useState<"create" | "edit">("create");
  const [selectedAsset, setSelectedAsset] = useState<AssetFormAsset | null>(null);
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [isPreloadOpen, setIsPreloadOpen] = useState(false);

  function handleCreateRequested() {
    setFormMode("create");
    setSelectedAsset(null);
    setIsFormOpen(true);
  }

  function handleEditRequested(asset: AssetTableRow) {
    setFormMode("edit");
    setSelectedAsset(asset);
    setIsFormOpen(true);
  }

  function handleCloseForm() {
    setIsFormOpen(false);
  }

  function handleOpenPreload() {
    setIsPreloadOpen(true);
  }

  function handleClosePreload() {
    setIsPreloadOpen(false);
  }

  return (
    <>
      <main className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
        <div className="space-y-3">
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
            Assets
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">Asset workstation</h1>
          <p className="max-w-3xl text-sm leading-6 text-muted">
            Search the asset catalog, filter by status or type, and prepare the first CRUD and preload
            workflows from one workstation.
          </p>
        </div>

        <form action="/assets" className="mt-6 space-y-4" method="get">
          <AssetsFilterBar
            filters={filters}
            onCreateRequested={handleCreateRequested}
            onPreloadRequested={handleOpenPreload}
          />
        </form>

        <div className="mt-6">
          <AssetsTable assets={assets} onEdit={handleEditRequested} />
        </div>
      </main>

      {isFormOpen ? (
        <AssetFormModal
          key={`${formMode}-${selectedAsset?.assetCode ?? "new"}`}
          asset={selectedAsset}
          isOpen={isFormOpen}
          mode={formMode}
          onClose={handleCloseForm}
          onSubmitted={handleCloseForm}
        />
      ) : null}
      {isPreloadOpen ? (
        <AssetPreloadModal
          key="asset-preload"
          isOpen={isPreloadOpen}
          onClose={handleClosePreload}
          onSubmitted={handleClosePreload}
        />
      ) : null}
    </>
  );
}
