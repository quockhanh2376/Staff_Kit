import "dotenv/config";

import { randomUUID } from "node:crypto";

import { afterEach, describe, expect, it } from "vitest";

import { ApiError } from "@/lib/api/errors";
import { prisma } from "@/lib/prisma";
import { reviewPendingRequest } from "@/lib/workflows/workflows.service";

type ReturnFixture = {
  actor: {
    accountId: number;
    username: string;
    displayName: string;
    role: "ADMIN";
  };
  teamId: number;
  reviewerId: number;
  employeeId: number;
  sessionId: number;
  requestId: number;
  requestKey: string;
  requestAssetIds: number[];
  extraAssetId: number;
  assignmentIds: number[];
};

const fixtures: ReturnFixture[] = [];
const describeWithDatabase = process.env.DATABASE_URL ? describe : describe.skip;

async function createReturnFixture() {
  const token = `return-review-${randomUUID()}`;

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

  const employee = await prisma.employee.create({
    data: {
      employeeId: `EMP-${token}`,
      fullName: `Employee ${token}`,
      teamId: team.id,
      email: `${token}@assetdesk-pro.local`,
      staffGroup: "employee_list",
    },
  });

  const requestAssetA = await prisma.asset.create({
    data: {
      assetCode: `RET-A-${token}`,
      name: `Return asset A ${token}`,
      assetType: "Laptop",
      status: "ASSIGNED",
    },
  });

  const requestAssetB = await prisma.asset.create({
    data: {
      assetCode: `RET-B-${token}`,
      name: `Return asset B ${token}`,
      assetType: "Monitor",
      status: "ASSIGNED",
    },
  });

  const extraAsset = await prisma.asset.create({
    data: {
      assetCode: `RET-C-${token}`,
      name: `Return asset C ${token}`,
      assetType: "Dock",
      status: "ASSIGNED",
    },
  });

  const assignmentA = await prisma.assetAssignment.create({
    data: {
      assetId: requestAssetA.id,
      employeeId: employee.id,
      status: "ACTIVE",
      assignedAt: new Date("2026-03-01T08:00:00.000Z"),
    },
  });

  const assignmentB = await prisma.assetAssignment.create({
    data: {
      assetId: requestAssetB.id,
      employeeId: employee.id,
      status: "ACTIVE",
      assignedAt: new Date("2026-03-01T08:05:00.000Z"),
    },
  });

  const assignmentC = await prisma.assetAssignment.create({
    data: {
      assetId: extraAsset.id,
      employeeId: employee.id,
      status: "ACTIVE",
      assignedAt: new Date("2026-03-01T08:10:00.000Z"),
    },
  });

  const session = await prisma.returnSession.create({
    data: {
      sessionKey: `SESSION-${token}`,
      qrToken: `QR-${token}`,
      createdByAccountId: reviewer.id,
      status: "ACTIVE",
    },
  });

  const request = await prisma.returnRequest.create({
    data: {
      requestKey: `REQ-${token}`,
      sessionId: session.id,
      employeeId: employee.id,
      status: "PENDING",
      employeeCodeSnapshot: employee.employeeId,
      employeeNameSnapshot: employee.fullName,
      notes: "submitted return payload",
      items: {
        create: [
          {
            assetId: requestAssetA.id,
            assetAssignmentId: assignmentA.id,
            assetCodeSnapshot: requestAssetA.assetCode,
            assetNameSnapshot: requestAssetA.name,
          },
          {
            assetId: requestAssetB.id,
            assetAssignmentId: assignmentB.id,
            assetCodeSnapshot: requestAssetB.assetCode,
            assetNameSnapshot: requestAssetB.name,
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
    employeeId: employee.id,
    sessionId: session.id,
    requestId: request.id,
    requestKey: request.requestKey,
    requestAssetIds: [requestAssetA.id, requestAssetB.id],
    extraAssetId: extraAsset.id,
    assignmentIds: [assignmentA.id, assignmentB.id, assignmentC.id],
  } satisfies ReturnFixture;

  fixtures.push(fixture);
  return fixture;
}

afterEach(async () => {
  const fixture = fixtures.pop();

  if (!fixture) {
    return;
  }

  await prisma.auditLog.deleteMany({
    where: {
      OR: [
        { returnRequestId: fixture.requestId },
        { actorAccountId: fixture.reviewerId },
      ],
    },
  });

  await prisma.approvalReview.deleteMany({
    where: {
      returnRequestId: fixture.requestId,
    },
  });

  await prisma.returnRequest.delete({
    where: {
      id: fixture.requestId,
    },
  });

  await prisma.returnSession.delete({
    where: {
      id: fixture.sessionId,
    },
  });

  await prisma.assetAssignment.deleteMany({
    where: {
      id: {
        in: fixture.assignmentIds,
      },
    },
  });

  await prisma.asset.deleteMany({
    where: {
      id: {
        in: [...fixture.requestAssetIds, fixture.extraAssetId],
      },
    },
  });

  await prisma.employee.delete({
    where: {
      id: fixture.employeeId,
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

describeWithDatabase("reviewPendingRequest for return requests", () => {
  it("approves a return request after removing an invalid reviewed asset", async () => {
    const fixture = await createReturnFixture();
    const request = await prisma.returnRequest.findUniqueOrThrow({
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

    const keptAssetCode = request.items[0]?.assetCodeSnapshot;

    if (!keptAssetCode) {
      throw new Error("Return fixture is missing request items.");
    }

    const result = await reviewPendingRequest(fixture.actor, {
      requestType: "RETURN",
      requestKey: fixture.requestKey,
      decision: "APPROVED",
      notes: "Removed invalid return asset",
      reviewedEmployeeId: request.employeeCodeSnapshot,
      reviewedAssetCodes: [keptAssetCode],
    } as never);

    expect(result).toMatchObject({
      requestType: "RETURN",
      status: "APPROVED",
      approvalDecision: "APPROVED",
      requestKey: fixture.requestKey,
    });

    const assignments = await prisma.assetAssignment.findMany({
      where: {
        id: {
          in: fixture.assignmentIds,
        },
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

    const returnedAssignments = assignments.filter((assignment) => assignment.status === "RETURNED");
    const activeAssignments = assignments.filter((assignment) => assignment.status === "ACTIVE");

    expect(returnedAssignments.map((assignment) => assignment.asset.assetCode)).toEqual([keptAssetCode]);
    expect(activeAssignments.map((assignment) => assignment.asset.assetCode)).toContain(request.items[1]?.assetCodeSnapshot);

    const auditEvent = await prisma.auditLog.findFirstOrThrow({
      where: {
        returnRequestId: fixture.requestId,
        actionType: "approval.review",
      },
      orderBy: {
        occurredAt: "desc",
      },
    });

    expect(auditEvent.metadata).toMatchObject({
      submittedEmployeeId: request.employeeCodeSnapshot,
      reviewedEmployeeId: request.employeeCodeSnapshot,
      reviewedAssetCodes: [keptAssetCode],
    });
  });

  it("rejects a return review that tries to add a new asset code", async () => {
    const fixture = await createReturnFixture();
    const request = await prisma.returnRequest.findUniqueOrThrow({
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

    const error = await reviewPendingRequest(fixture.actor, {
      requestType: "RETURN",
      requestKey: fixture.requestKey,
      decision: "APPROVED",
      notes: "Invalid add attempt",
      reviewedEmployeeId: request.employeeCodeSnapshot,
      reviewedAssetCodes: [
        request.items[0]?.assetCodeSnapshot ?? "",
        request.items[1]?.assetCodeSnapshot ?? "",
        `RET-C-${fixture.requestKey.replace("REQ-", "")}`,
      ],
    } as never).catch((caughtError: unknown) => caughtError);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).code).toBe("return_review_asset_add_not_allowed");
  });
});
