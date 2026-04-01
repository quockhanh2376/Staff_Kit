import "dotenv/config";

import { describe, expect, it } from "vitest";

import { getReviewRequestDetail } from "@/lib/workflows/workflows.service";

const describeWithDatabase = process.env.DATABASE_URL ? describe : describe.skip;

describeWithDatabase("getReviewRequestDetail", () => {
  it("builds receive review detail with submitted snapshot and reviewed defaults", async () => {
    const detail = await getReviewRequestDetail("RECEIVE", "seed-receive-pending-001");

    expect(detail).toMatchObject({
      requestType: "RECEIVE",
      requestKey: "seed-receive-pending-001",
      status: "PENDING",
      submittedSnapshot: {
        employeeId: "ADP006",
      },
      reviewedDraft: {
        employeeId: "ADP006",
        assetCodes: ["ADP-MON-1001", "ADP-NB-1001"],
      },
      rules: {
        canAddAssets: true,
        canRemoveAssets: true,
      },
    });
  });

  it("builds return review detail with removal-only rules", async () => {
    const detail = await getReviewRequestDetail("RETURN", "seed-return-pending-001");

    expect(detail).toMatchObject({
      requestType: "RETURN",
      requestKey: "seed-return-pending-001",
      status: "PENDING",
      submittedSnapshot: {
        employeeId: "ADP002",
      },
      reviewedDraft: {
        employeeId: "ADP002",
        assetCodes: ["ADP-PH-2001"],
      },
      rules: {
        canAddAssets: false,
        canRemoveAssets: true,
      },
    });
  });
});
