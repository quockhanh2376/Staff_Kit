import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";

import type { ReviewRequestDetail } from "@/lib/workflows/workflows.service";
import { ReviewDetailView } from "@/components/reviews/ReviewDetailView";

const receiveDetail: ReviewRequestDetail = {
  requestType: "RECEIVE",
  requestKey: "seed-receive-pending-001",
  status: "PENDING",
  submittedAt: new Date("2026-03-10T09:00:00.000Z"),
  submittedNotes: "submitted payload",
  submittedSnapshot: {
    employeeId: "ADP006",
    employeeName: "Bui Gia Linh",
    assetCodes: ["ADP-NB-1001", "ADP-MON-1001"],
    items: [
      {
        assetCode: "ADP-NB-1001",
        assetName: "Dell Latitude 7450",
      },
      {
        assetCode: "ADP-MON-1001",
        assetName: "Dell P2425H",
      },
    ],
  },
  reviewedDraft: {
    employeeId: "ADP006",
    assetCodes: ["ADP-NB-1001", "ADP-MON-1001"],
    notes: "",
  },
  rules: {
    canAddAssets: true,
    canRemoveAssets: true,
  },
};

describe("ReviewDetailView", () => {
  it("renders the submitted snapshot, review form, and decision actions", () => {
    const markup = renderToStaticMarkup(
      <ReviewDetailView
        detail={receiveDetail}
        requestTypeSegment="receive"
      />,
    );

    expect(markup).toContain("Submitted snapshot");
    expect(markup).toContain("IT review form");
    expect(markup).toContain("Decision panel");
    expect(markup).toContain("Approve");
    expect(markup).toContain("Reject");
    expect(markup).toContain("Review asset codes");
    expect(markup).toContain("seed-receive-pending-001");
  });
});
