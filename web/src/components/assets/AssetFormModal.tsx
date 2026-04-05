"use client";

import { useState, useTransition } from "react";

import type { AssetActionResult } from "@/app/(protected)/assets/actions";
import { createAssetAction, updateAssetAction } from "@/app/(protected)/assets/actions";

export type AssetFormAsset = {
  assetCode: string;
  name: string;
  assetType: string;
  status: string;
  recordedAt?: Date | null;
  owningUnit?: string | null;
  managingUnit?: string | null;
  serialNumber?: string | null;
  brand?: string | null;
  modelName?: string | null;
  notes?: string | null;
  retiredAt?: Date | null;
  disposedAt?: Date | null;
};

type AssetFormModalProps = {
  isOpen: boolean;
  mode: "create" | "edit";
  asset?: AssetFormAsset | null;
  onClose: () => void;
  onSubmitted: () => void;
};

type AssetFormValues = {
  assetCode: string;
  name: string;
  assetType: string;
  status: string;
  recordedAt: string;
  owningUnit: string;
  managingUnit: string;
  serialNumber: string;
  brand: string;
  modelName: string;
  notes: string;
  retiredAt: string;
  disposedAt: string;
};

function formatDateInput(value?: Date | null) {
  return value ? value.toISOString().slice(0, 10) : "";
}

function buildInitialValues(asset?: AssetFormAsset | null): AssetFormValues {
  return {
    assetCode: asset?.assetCode ?? "",
    name: asset?.name ?? "",
    assetType: asset?.assetType ?? "",
    status: asset?.status ?? "IN_STOCK",
    recordedAt: formatDateInput(asset?.recordedAt),
    owningUnit: asset?.owningUnit ?? "",
    managingUnit: asset?.managingUnit ?? "",
    serialNumber: asset?.serialNumber ?? "",
    brand: asset?.brand ?? "",
    modelName: asset?.modelName ?? "",
    notes: asset?.notes ?? "",
    retiredAt: formatDateInput(asset?.retiredAt),
    disposedAt: formatDateInput(asset?.disposedAt),
  };
}

function getSubmitLabel(mode: "create" | "edit", isPending: boolean) {
  if (isPending) {
    return mode === "create" ? "Creating..." : "Saving...";
  }

  return mode === "create" ? "Create Asset" : "Save Changes";
}

function FieldLabel({
  htmlFor,
  label,
  children,
}: Readonly<{
  htmlFor: string;
  label: string;
  children: React.ReactNode;
}>) {
  return (
    <label className="space-y-2 text-sm text-muted" htmlFor={htmlFor}>
      <span className="block font-medium text-foreground">{label}</span>
      {children}
    </label>
  );
}

export function AssetFormModal({
  isOpen,
  mode,
  asset,
  onClose,
  onSubmitted,
}: AssetFormModalProps) {
  const [isPending, startTransition] = useTransition();
  const [result, setResult] = useState<AssetActionResult | null>(null);
  const [values, setValues] = useState<AssetFormValues>(() => buildInitialValues(asset));

  if (!isOpen) {
    return null;
  }

  function updateValue(field: keyof AssetFormValues, value: string) {
    setValues((current) => {
      if (field === "status") {
        return {
          ...current,
          status: value,
          retiredAt: value === "RETIRED" ? current.retiredAt : "",
          disposedAt: value === "DISPOSED" ? current.disposedAt : "",
        };
      }

      return {
        ...current,
        [field]: value,
      };
    });
  }

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const formData = new FormData(event.currentTarget);

    startTransition(async () => {
      const action = mode === "create" ? createAssetAction : updateAssetAction;
      const nextResult = await action(formData);

      setResult(nextResult);

      if (nextResult.ok) {
        onSubmitted();
        onClose();

        if (typeof window !== "undefined") {
          window.location.reload();
        }
      }
    });
  }

  const isRetired = values.status === "RETIRED";
  const isDisposed = values.status === "DISPOSED";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(4,10,8,0.72)] px-4 py-6">
      <div className="max-h-[90vh] w-full max-w-5xl overflow-y-auto rounded-[28px] border border-border bg-surface p-6 shadow-[0_30px_90px_rgba(2,8,6,0.38)]">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
              {mode === "create" ? "New Asset" : "Edit Asset"}
            </p>
            <h2 className="text-2xl font-semibold tracking-tight text-foreground">
              {mode === "create" ? "Create asset record" : `Update ${asset?.assetCode ?? "asset"}`}
            </h2>
            <p className="max-w-2xl text-sm leading-6 text-muted">
              Maintain asset master data here. Assignment and stock movement stay in receive, return,
              and approval workflows.
            </p>
          </div>

          <button
            className="rounded-full border border-border bg-surface-strong px-4 py-2 text-sm font-medium text-muted transition hover:border-accent hover:text-accent"
            onClick={onClose}
            type="button"
          >
            Close
          </button>
        </div>

        <form className="mt-6 space-y-5" onSubmit={handleSubmit}>
          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            <FieldLabel htmlFor="assetCode" label="Asset Code">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="assetCode"
                name="assetCode"
                onChange={(event) => updateValue("assetCode", event.target.value)}
                readOnly={mode === "edit"}
                value={values.assetCode}
              />
            </FieldLabel>

            <FieldLabel htmlFor="name" label="Name">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="name"
                name="name"
                onChange={(event) => updateValue("name", event.target.value)}
                value={values.name}
              />
            </FieldLabel>

            <FieldLabel htmlFor="assetType" label="Asset Type">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="assetType"
                name="assetType"
                onChange={(event) => updateValue("assetType", event.target.value)}
                value={values.assetType}
              />
            </FieldLabel>

            <FieldLabel htmlFor="status" label="Status">
              <select
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="status"
                name="status"
                onChange={(event) => updateValue("status", event.target.value)}
                value={values.status}
              >
                <option value="IN_STOCK">IN_STOCK</option>
                <option value="ASSIGNED">ASSIGNED</option>
                <option value="RETIRED">RETIRED</option>
                <option value="DISPOSED">DISPOSED</option>
              </select>
            </FieldLabel>

            <FieldLabel htmlFor="recordedAt" label="Recorded At">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="recordedAt"
                name="recordedAt"
                onChange={(event) => updateValue("recordedAt", event.target.value)}
                type="date"
                value={values.recordedAt}
              />
            </FieldLabel>

            <FieldLabel htmlFor="owningUnit" label="Owning Unit">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="owningUnit"
                name="owningUnit"
                onChange={(event) => updateValue("owningUnit", event.target.value)}
                value={values.owningUnit}
              />
            </FieldLabel>

            <FieldLabel htmlFor="managingUnit" label="Managing Unit">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="managingUnit"
                name="managingUnit"
                onChange={(event) => updateValue("managingUnit", event.target.value)}
                value={values.managingUnit}
              />
            </FieldLabel>

            <FieldLabel htmlFor="serialNumber" label="Serial Number">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="serialNumber"
                name="serialNumber"
                onChange={(event) => updateValue("serialNumber", event.target.value)}
                value={values.serialNumber}
              />
            </FieldLabel>

            <FieldLabel htmlFor="brand" label="Brand">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="brand"
                name="brand"
                onChange={(event) => updateValue("brand", event.target.value)}
                value={values.brand}
              />
            </FieldLabel>

            <FieldLabel htmlFor="modelName" label="Model Name">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
                id="modelName"
                name="modelName"
                onChange={(event) => updateValue("modelName", event.target.value)}
                value={values.modelName}
              />
            </FieldLabel>

            <FieldLabel htmlFor="retiredAt" label="Retired At">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition disabled:cursor-not-allowed disabled:opacity-50"
                disabled={!isRetired}
                id="retiredAt"
                name="retiredAt"
                onChange={(event) => updateValue("retiredAt", event.target.value)}
                type="date"
                value={values.retiredAt}
              />
            </FieldLabel>

            <FieldLabel htmlFor="disposedAt" label="Disposed At">
              <input
                className="w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition disabled:cursor-not-allowed disabled:opacity-50"
                disabled={!isDisposed}
                id="disposedAt"
                name="disposedAt"
                onChange={(event) => updateValue("disposedAt", event.target.value)}
                type="date"
                value={values.disposedAt}
              />
            </FieldLabel>
          </div>

          <FieldLabel htmlFor="notes" label="Notes">
            <textarea
              className="min-h-28 w-full rounded-2xl border border-border bg-surface-strong px-4 py-3 text-sm text-foreground outline-none transition focus:border-accent"
              id="notes"
              name="notes"
              onChange={(event) => updateValue("notes", event.target.value)}
              value={values.notes}
            />
          </FieldLabel>

          {result ? (
            <div
              className={`rounded-2xl border px-4 py-3 text-sm ${
                result.ok
                  ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-200"
                  : "border-rose-500/40 bg-rose-500/10 text-rose-200"
              }`}
            >
              {result.message}
            </div>
          ) : null}

          <div className="flex flex-wrap items-center justify-end gap-3">
            <button
              className="rounded-full border border-border bg-surface-strong px-5 py-2.5 text-sm font-medium text-muted transition hover:border-accent hover:text-accent"
              onClick={onClose}
              type="button"
            >
              Cancel
            </button>
            <button
              className="rounded-full bg-accent px-5 py-2.5 text-sm font-semibold text-accent-foreground transition hover:brightness-110 disabled:opacity-60"
              disabled={isPending}
              type="submit"
            >
              {getSubmitLabel(mode, isPending)}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
