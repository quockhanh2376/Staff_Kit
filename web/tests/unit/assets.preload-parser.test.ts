import { describe, expect, it } from "vitest";
import * as XLSX from "xlsx";

import { parseAssetPreloadFile } from "@/lib/assets/assets-preload-parser";

describe("parseAssetPreloadFile", () => {
  it("parses a CSV file into preload asset rows", async () => {
    const file = new File(
      [
        [
          "assetCode,name,assetType,status,owningUnit,managingUnit",
          "AST-1,Latitude 5440,Laptop,IN_STOCK,IT,IT",
        ].join("\n"),
      ],
      "assets.csv",
      { type: "text/csv" },
    );

    const result = await parseAssetPreloadFile(file);

    expect(result.summary.totalRows).toBe(1);
    expect(result.summary.validRows).toBe(1);
    expect(result.summary.invalidRows).toBe(0);
    expect(result.validRows).toHaveLength(1);
    expect(result.validRows[0]?.assetCode).toBe("AST-1");
  });

  it("parses an XLSX file into preload asset rows", async () => {
    const workbook = XLSX.utils.book_new();
    const worksheet = XLSX.utils.json_to_sheet([
      {
        assetCode: "AST-2",
        name: "Dell Dock",
        assetType: "Dock",
        status: "IN_STOCK",
      },
    ]);

    XLSX.utils.book_append_sheet(workbook, worksheet, "Assets");

    const buffer = XLSX.write(workbook, {
      type: "buffer",
      bookType: "xlsx",
    });

    const file = new File([buffer], "assets.xlsx", {
      type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    });

    const result = await parseAssetPreloadFile(file);

    expect(result.summary.totalRows).toBe(1);
    expect(result.summary.validRows).toBe(1);
    expect(result.validRows[0]?.assetCode).toBe("AST-2");
  });

  it("rejects duplicate asset codes within the same upload", async () => {
    const file = new File(
      [
        [
          "assetCode,name,assetType,status",
          "AST-3,Latitude,Laptop,IN_STOCK",
          "AST-3,Latitude Spare,Laptop,IN_STOCK",
        ].join("\n"),
      ],
      "duplicate-assets.csv",
      { type: "text/csv" },
    );

    const result = await parseAssetPreloadFile(file);

    expect(result.summary.totalRows).toBe(2);
    expect(result.summary.validRows).toBe(0);
    expect(result.summary.invalidRows).toBe(2);
    expect(result.invalidRows).toHaveLength(2);
    expect(result.invalidRows[0]?.issues[0]?.message).toMatch(/duplicate/i);
    expect(result.invalidRows[1]?.issues[0]?.message).toMatch(/duplicate/i);
  });
});
