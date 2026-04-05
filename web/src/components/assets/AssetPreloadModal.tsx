"use client";

import { useState, useTransition } from "react";

import type { AssetActionResult } from "@/app/(protected)/assets/actions";
import { preloadAssetsAction } from "@/app/(protected)/assets/actions";
import {
  parseAssetPreloadFile,
  type AssetPreloadParseResult,
} from "@/lib/assets/assets-preload-parser";

type AssetPreloadModalProps = {
  isOpen: boolean;
  onClose: () => void;
  onSubmitted: () => void;
};

function buildErrorResult(message: string): AssetActionResult {
  return {
    ok: false,
    errorCode: "validation_error",
    message,
  };
}

export function AssetPreloadModal({
  isOpen,
  onClose,
  onSubmitted,
}: AssetPreloadModalProps) {
  const [isPending, startTransition] = useTransition();
  const [preview, setPreview] = useState<AssetPreloadParseResult | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [result, setResult] = useState<AssetActionResult | null>(null);

  if (!isOpen) {
    return null;
  }

  async function handleFileChange(file: File | null) {
    setSelectedFile(file);
    setResult(null);

    if (!file) {
      setPreview(null);
      return;
    }

    try {
      const nextPreview = await parseAssetPreloadFile(file);
      setPreview(nextPreview);
    } catch (error) {
      setPreview(null);
      setResult(
        buildErrorResult(
          error instanceof Error ? error.message : "Unable to parse the selected file.",
        ),
      );
    }
  }

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!selectedFile) {
      setResult(buildErrorResult("Select a CSV or XLSX file before submitting."));
      return;
    }

    if (!preview) {
      setResult(buildErrorResult("Preview the file successfully before submitting."));
      return;
    }

    if (preview.invalidRows.length > 0 || preview.validRows.length === 0) {
      setResult(buildErrorResult("Resolve invalid rows before submitting the preload batch."));
      return;
    }

    const formData = new FormData();
    formData.set("file", selectedFile);

    startTransition(async () => {
      const nextResult = await preloadAssetsAction(formData);
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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(4,10,8,0.72)] px-4 py-6">
      <div className="max-h-[90vh] w-full max-w-4xl overflow-y-auto rounded-[28px] border border-border bg-surface p-6 shadow-[0_30px_90px_rgba(2,8,6,0.38)]">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
              Preload Assets
            </p>
            <h2 className="text-2xl font-semibold tracking-tight text-foreground">
              Upload CSV or XLSX
            </h2>
            <p className="max-w-2xl text-sm leading-6 text-muted">
              Bulk upsert asset master data with fixed headers. The file is previewed client-side first,
              then validated again server-side before the batch is committed.
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
          <div className="space-y-3 rounded-[24px] border border-border bg-surface-strong px-5 py-5">
            <label className="space-y-2 text-sm text-muted">
              <span className="block font-medium text-foreground">Upload CSV or XLSX</span>
              <input
                accept=".csv,.xlsx"
                className="block w-full rounded-2xl border border-border bg-surface px-4 py-3 text-sm text-foreground outline-none transition file:mr-4 file:rounded-full file:border-0 file:bg-accent file:px-4 file:py-2 file:text-sm file:font-semibold file:text-accent-foreground"
                onChange={(event) => {
                  const file = event.target.files?.[0] ?? null;
                  void handleFileChange(file);
                }}
                type="file"
              />
            </label>

            <div className="grid gap-4 md:grid-cols-3">
              <div className="rounded-2xl border border-border bg-surface px-4 py-4">
                <p className="text-xs uppercase tracking-[0.18em] text-muted">Total rows</p>
                <p className="mt-2 text-2xl font-semibold text-foreground">
                  {preview?.summary.totalRows ?? 0}
                </p>
              </div>
              <div className="rounded-2xl border border-border bg-surface px-4 py-4">
                <p className="text-xs uppercase tracking-[0.18em] text-muted">Valid rows</p>
                <p className="mt-2 text-2xl font-semibold text-emerald-300">
                  {preview?.summary.validRows ?? 0}
                </p>
              </div>
              <div className="rounded-2xl border border-border bg-surface px-4 py-4">
                <p className="text-xs uppercase tracking-[0.18em] text-muted">Invalid rows</p>
                <p className="mt-2 text-2xl font-semibold text-rose-300">
                  {preview?.summary.invalidRows ?? 0}
                </p>
              </div>
            </div>
          </div>

          <div className="rounded-[24px] border border-border bg-surface-strong px-5 py-5">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-lg font-semibold text-foreground">Validation summary</h3>
              <p className="text-sm text-muted">
                Required headers: <span className="font-mono">assetCode, name, assetType</span>
              </p>
            </div>

            {preview?.invalidRows.length ? (
              <ul className="mt-4 space-y-3">
                {preview.invalidRows.slice(0, 8).map((row) => (
                  <li
                    key={row.rowNumber}
                    className="rounded-2xl border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-sm text-rose-100"
                  >
                    <p className="font-semibold">Row {row.rowNumber}</p>
                    <ul className="mt-2 space-y-1 text-xs text-rose-100/80">
                      {row.issues.map((issue, index) => (
                        <li key={`${row.rowNumber}-${index}`}>
                          {issue.field ? `${issue.field}: ` : ""}
                          {issue.message}
                        </li>
                      ))}
                    </ul>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="mt-4 text-sm text-muted">
                {preview
                  ? "No validation errors were found in the current file."
                  : "Upload a file to preview valid and invalid rows before submitting."}
              </p>
            )}
          </div>

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
              {isPending ? "Uploading..." : "Submit Preload Batch"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
