import "dotenv/config";

import { randomUUID } from "node:crypto";

import { afterEach, describe, expect, it } from "vitest";

import { ApiError } from "@/lib/api/errors";
import { prisma } from "@/lib/prisma";
import { reviewPendingRequest } from "@/lib/workflows/workflows.service";

type ReceiveFixture = {
  actor: {
    accountId: number;
    username: string;
    displayName: string;
    role: "ADMIN";
  };
  teamId: number;
  reviewerId: number;
  submittedEmployeeId: number;
  reviewedEmployeeId: number;
  sessionId: number;
  requestId: number;
  requestKey: string;
  assetIds: number[];
};

const fixtureKeys: ReceiveFixture[] = [];
const describeWithDatabase = process.env.DATABASE_URL ? describe : describe.skip;

async function createReceiveFixture() {
  const token = `review-test-${randomUUID()}`;

  const team = await prisma.team.create({
    data: {
      name: `T-${token}`,
    },
  });

  const reviewer = await prisma.localAccount.create({
    data: {
      displayName: `Reviewer ${token}`,
      username: `reviewer-${token}`,
      passwordHash: "hash",
      role: "ADMIN",
      forcePasswordReset: false,
      isActive: true,
    },
  });

  const submittedEmployee = await prisma.employee.create({
    data: {
      employeeId: `EMP-S-${token}`,
      fullName: `Submitted ${token}`,
      teamId: team.id,
      email: `submitted-${token}@assetdesk-pro.local`,
      staffGroup: "employee_list",
    },
  });

  const reviewedEmployee = await prisma.employee.create({
    data: {
      employeeId: `EMP-R-${token}`,
      fullName: `Reviewed ${token}`,
      teamId: team.id,
      email: `reviewed-${token}@assetdesk-pro.local`,
      staffGroup: "employee_list",
    },
  });

  const submittedAsset = await prisma.asset.create({
    data: {
      assetCode: `AST-S-${token}`,
      name: `Submitted asset ${token}`,
      assetType: "Laptop",
      status: "IN_STOCK",
    },
  });

  const removedAsset = await prisma.asset.create({
    data: {
      assetCode: `AST-RM-${token}`,
      name: `Removed asset ${token}`,
      assetType: "Monitor",
      status: "IN_STOCK",
    },
  });

  const addedAsset = await prisma.asset.create({
    data: {
      assetCode: `AST-ADD-${token}`,
      name: `Added asset ${token}`,
      assetType: "Dock",
      status: "IN_STOCK",
    },
  });

  const session = await prisma.receiveSession.create({
    data: {
      sessionKey: `SESSION-${token}`,
      qrToken: `QR-${token}`,
      createdByAccountId: reviewer.id,
      status: "ACTIVE",
    },
  });

  const request = await prisma.receiveRequest.create({
    data: {
      requestKey: `REQ-${token}`,
      sessionId: session.id,
      employeeId: submittedEmployee.id,
      status: "PENDING",
      employeeCodeSnapshot: submittedEmployee.employeeId,
      employeeNameSnapshot: submittedEmployee.fullName,
      notes: "submitted payload",
      items: {
        create: [
          {
            assetId: submittedAsset.id,
            assetCodeSnapshot: submittedAsset.assetCode,
            assetNameSnapshot: submittedAsset.name,
          },
          {
            assetId: removedAsset.id,
            assetCodeSnapshot: removedAsset.assetCode,
            assetNameSnapshot: removedAsset.name,
          },
        ],
      },
    },
  });

  const fixture = {
    actor: {
      accountId: reviewer.id,
      username: reviewer.username,
      displayName: reviewer.displayName,
      role: "ADMIN" as const,
    },
    teamId: team.id,
    reviewerId: reviewer.id,
    submittedEmployeeId: submittedEmployee.id,
    reviewedEmployeeId: reviewedEmployee.id,
    sessionId: session.id,
    requestId: request.id,
    requestKey: request.requestKey,
    assetIds: [submittedAsset.id, removedAsset.id, addedAsset.id],
  } satisfies ReceiveFixture;

  fixtureKeys.push(fixture);
  return fixture;
}

afterEach(async () => {
  const fixture = fixtureKeys.pop();

  if (!fixture) {
    return;
  }

  await prisma.auditLog.deleteMany({
    where: {
      OR: [
        { receiveRequestId: fixture.requestId },
        { actorAccountId: fixture.reviewerId },
      ],
    },
  });

  await prisma.approvalReview.deleteMany({
    where: {
      receiveRequestId: fixture.requestId,
    },
  });

  await prisma.assetAssignment.deleteMany({
    where: {
      OR: [
        {
          assetId: {
            in: fixture.assetIds,
          },
        },
        {
          employeeId: {
            in: [fixture.submittedEmployeeId, fixture.reviewedEmployeeId],
          },
        },
      ],
    },
  });

  await prisma.receiveRequest.delete({
    where: {
      id: fixture.requestId,
    },
  });

  await prisma.receiveSession.delete({
    where: {
      id: fixture.sessionId,
    },
  });

  await prisma.asset.deleteMany({
    where: {
      id: {
        in: fixture.assetIds,
      },
    },
  });

  await prisma.employee.deleteMany({
    where: {
      id: {
        in: [fixture.submittedEmployeeId, fixture.reviewedEmployeeId],
      },
    },
  });

  await prisma.localAccount.delete({
    where: {
      id: fixture.reviewerId,
    },
  });

  await prisma.team.delete({
    where: {
      id: fixture.teamId,
    },
  });
});

describeWithDatabase("reviewPendingRequest for receive requests", () => {
  it("approves a receive request with corrected employee and reviewed asset codes", async () => {
    const fixture = await createReceiveFixture();

    const submittedRequest = await prisma.receiveRequest.findUniqueOrThrow({
      where: {
        id: fixture.requestId,
      },
      include: {
        items: {
          orderBy: {
            assetCodeSnapshot: "asc",
          },
        },
      },
    });

    const submittedAssetCode = submittedRequest.items.find((item) =>
      item.assetCodeSnapshot.startsWith("AST-S-"),
    )?.assetCodeSnapshot;

    if (!submittedAssetCode) {
      throw new Error("Submitted asset code was not created for the fixture.");
    }

    const reviewedAssetCodes = [
      submittedAssetCode,
      `AST-ADD-${fixture.requestKey.replace("REQ-", "")}`,
    ];

    const result = await reviewPendingRequest(fixture.actor, {
      requestType: "RECEIVE",
      requestKey: fixture.requestKey,
      decision: "APPROVED",
      notes: "IT corrected the payload",
      reviewedEmployeeId: `EMP-R-${fixture.requestKey.replace("REQ-", "")}`,
      reviewedAssetCodes,
    } as never);

    expect(result).toMatchObject({
      requestType: "RECEIVE",
      status: "APPROVED",
      approvalDecision: "APPROVED",
      requestKey: fixture.requestKey,
    });

    const reviewedEmployee = await prisma.employee.findUniqueOrThrow({
      where: {
        id: fixture.reviewedEmployeeId,
      },
    });

    const assignments = await prisma.assetAssignment.findMany({
      where: {
        employeeId: reviewedEmployee.id,
      },
      include: {
        asset: true,
      },
      orderBy: {
        asset: {
          assetCode: "asc",
        },
      },
    });

    const auditEvent = await prisma.auditLog.findFirstOrThrow({
      where: {
        receiveRequestId: fixture.requestId,
        actionType: "approval.review",
      },
      orderBy: {
        occurredAt: "desc",
      },
    });

    expect(
      assignments.map((assignment) => assignment.asset.assetCode).sort(),
    ).toEqual([...reviewedAssetCodes].sort());

    expect(auditEvent.metadata).toMatchObject({
      submittedEmployeeId: submittedRequest.employeeCodeSnapshot,
      reviewedEmployeeId: reviewedEmployee.employeeId,
      reviewedAssetCodes,
    });
  });

  it("rejects a receive review when reviewed asset codes contain duplicates", async () => {
    const fixture = await createReceiveFixture();

    const error = await reviewPendingRequest(fixture.actor, {
      requestType: "RECEIVE",
      requestKey: fixture.requestKey,
      decision: "APPROVED",
      notes: "duplicate review payload",
      reviewedEmployeeId: `EMP-R-${fixture.requestKey.replace("REQ-", "")}`,
      reviewedAssetCodes: ["AST-S-dup", "AST-S-dup"],
    } as never).catch((caughtError: unknown) => caughtError);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).code).toBe("duplicate_asset_codes");
  });

  it("rejects a receive review when reject notes are missing", async () => {
    const fixture = await createReceiveFixture();

    const error = await reviewPendingRequest(fixture.actor, {
      requestType: "RECEIVE",
      requestKey: fixture.requestKey,
      decision: "REJECTED",
      reviewedEmployeeId: `EMP-R-${fixture.requestKey.replace("REQ-", "")}`,
      reviewedAssetCodes: [`AST-S-${fixture.requestKey.replace("REQ-", "")}`],
    } as never).catch((caughtError: unknown) => caughtError);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).code).toBe("review_notes_required");
  });
});
