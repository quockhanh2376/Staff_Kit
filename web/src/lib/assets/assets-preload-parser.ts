import * as XLSX from "xlsx";
import { ZodError } from "zod";

import {
  assetPreloadRowSchema,
  type AssetPreloadRowInput,
} from "@/lib/assets/assets.schemas";

const supportedHeaders = [
  "assetCode",
  "name",
  "assetType",
  "status",
  "recordedAt",
  "owningUnit",
  "managingUnit",
  "serialNumber",
  "brand",
  "modelName",
  "notes",
  "retiredAt",
  "disposedAt",
] as const;

const requiredHeaders = ["assetCode", "name", "assetType"] as const;

type SupportedHeader = (typeof supportedHeaders)[number];

type RawAssetPreloadRow = Partial<Record<SupportedHeader, string | undefined>>;

type AssetPreloadIssue = {
  field?: string;
  message: string;
};

export type AssetPreloadInvalidRow = {
  rowNumber: number;
  rawRow: RawAssetPreloadRow;
  issues: AssetPreloadIssue[];
};

export type AssetPreloadParseResult = {
  rows: Array<{
    rowNumber: number;
    rawRow: RawAssetPreloadRow;
  }>;
  validRows: AssetPreloadRowInput[];
  invalidRows: AssetPreloadInvalidRow[];
  summary: {
    totalRows: number;
    validRows: number;
    invalidRows: number;
  };
};

function normalizeHeader(value: unknown) {
  return String(value ?? "").trim();
}

function normalizeCellValue(value: unknown) {
  const stringValue = String(value ?? "").trim();
  return stringValue.length > 0 ? stringValue : undefined;
}

function getWorkbook(file: File, content: string | ArrayBuffer) {
  const extension = file.name.split(".").pop()?.toLowerCase();

  if (extension === "csv") {
    return XLSX.read(content as string, { type: "string" });
  }

  if (extension === "xlsx") {
    return XLSX.read(content as ArrayBuffer, { type: "array" });
  }

  throw new Error(`Unsupported preload file type: ${file.name}`);
}

async function readSheetRows(file: File) {
  const extension = file.name.split(".").pop()?.toLowerCase();
  const content = extension === "csv" ? await file.text() : await file.arrayBuffer();
  const workbook = getWorkbook(file, content);
  const sheetName = workbook.SheetNames[0];

  if (!sheetName) {
    throw new Error("The preload file does not contain any worksheets.");
  }

  const sheet = workbook.Sheets[sheetName];
  const headerRows = XLSX.utils.sheet_to_json<unknown[]>(sheet, {
    header: 1,
    blankrows: false,
    raw: false,
  });
  const headers = headerRows[0]?.map(normalizeHeader) ?? [];

  if (headers.length === 0) {
    throw new Error("The preload file is empty.");
  }

  const missingHeaders = requiredHeaders.filter((header) => !headers.includes(header));

  if (missingHeaders.length > 0) {
    throw new Error(`Missing required headers: ${missingHeaders.join(", ")}`);
  }

  const unsupported = headers.filter(
    (header) => header.length > 0 && !supportedHeaders.includes(header as SupportedHeader),
  );

  if (unsupported.length > 0) {
    throw new Error(`Unsupported headers: ${unsupported.join(", ")}`);
  }

  const rawRows = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet, {
    defval: "",
    raw: false,
    blankrows: false,
  });

  return rawRows.map((rawRow, index) => {
    const normalizedRow = Object.fromEntries(
      supportedHeaders.map((header) => [header, normalizeCellValue(rawRow[header])]),
    ) as RawAssetPreloadRow;

    return {
      rowNumber: index + 2,
      rawRow: normalizedRow,
    };
  });
}

function mapZodIssues(error: ZodError): AssetPreloadIssue[] {
  return error.issues.map((issue) => ({
    field: issue.path[0] ? String(issue.path[0]) : undefined,
    message: issue.message,
  }));
}

export async function parseAssetPreloadFile(file: File): Promise<AssetPreloadParseResult> {
  const rows = await readSheetRows(file);
  const parsedRows = rows.map(({ rowNumber, rawRow }) => {
    const parsed = assetPreloadRowSchema.safeParse(rawRow);

    if (!parsed.success) {
      return {
        rowNumber,
        rawRow,
        status: "invalid" as const,
        issues: mapZodIssues(parsed.error),
      };
    }

    return {
      rowNumber,
      rawRow,
      status: "valid" as const,
      data: parsed.data,
    };
  });

  const duplicateCounts = new Map<string, number>();

  for (const row of parsedRows) {
    if (row.status === "valid") {
      duplicateCounts.set(row.data.assetCode, (duplicateCounts.get(row.data.assetCode) ?? 0) + 1);
    }
  }

  const validRows: AssetPreloadRowInput[] = [];
  const invalidRows: AssetPreloadInvalidRow[] = [];

  for (const row of parsedRows) {
    if (row.status === "invalid") {
      invalidRows.push({
        rowNumber: row.rowNumber,
        rawRow: row.rawRow,
        issues: row.issues,
      });
      continue;
    }

    if ((duplicateCounts.get(row.data.assetCode) ?? 0) > 1) {
      invalidRows.push({
        rowNumber: row.rowNumber,
        rawRow: row.rawRow,
        issues: [
          {
            field: "assetCode",
            message: `Duplicate assetCode ${row.data.assetCode} in the same upload.`,
          },
        ],
      });
      continue;
    }

    validRows.push(row.data);
  }

  return {
    rows,
    validRows,
    invalidRows,
    summary: {
      totalRows: rows.length,
      validRows: validRows.length,
      invalidRows: invalidRows.length,
    },
  };
}
