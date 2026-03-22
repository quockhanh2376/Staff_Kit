import type { WorkflowRequestType } from "@/generated/prisma/client";

import type { ApiActor } from "@/lib/api/auth";
import { ApiError } from "@/lib/api/errors";
import { prisma } from "@/lib/prisma";
import { writeAuditLog } from "@/lib/audit/audit.service";

import {
  createReceiveRequestRecord,
  createReceiveSessionRecord,
  createReturnRequestRecord,
  createReturnSessionRecord,
  findActiveReceiveSession,
  findActiveAssignmentsByAssetCodes,
  findActiveReceiveSessionByQrToken,
  findActiveReturnSessionByQrToken,
  findAssetsByAssetCodes,
  findEmployeeByEmployeeId,
  findReceiveRequestForReview,
  findReturnRequestForReview,
  listPendingReceiveRequests,
  listPendingReturnRequests,
} from "./workflows.repository";
import type {
  CreateSessionInput,
  PendingRequestFiltersInput,
  ReviewPendingRequestInput,
  SubmitReceiveRequestInput,
  SubmitReturnRequestInput,
} from "./workflows.schemas";

export type ReviewRequestDetail = {
  requestType: WorkflowRequestType;
  requestKey: string;
  status: "PENDING" | "APPROVED" | "REJECTED";
  submittedAt: Date;
  submittedNotes?: string | null;
  submittedSnapshot: {
    employeeId: string;
    employeeName: string;
    assetCodes: string[];
    items: Array<{
      assetCode: string;
      assetName?: string | null;
      assignmentEmployeeId?: string;
      assignmentEmployeeName?: string;
    }>;
  };
  reviewedDraft: {
    employeeId: string;
    assetCodes: string[];
    notes?: string | null;
  };
  rules: {
    canAddAssets: boolean;
    canRemoveAssets: boolean;
  };
};

function normalizeAssetCodes(assetCodes: string[]) {
  const normalized = assetCodes.map((code) => code.trim());
  const seen = new Set<string>();
  const duplicates = new Set<string>();

  for (const code of normalized) {
    const key = code.toUpperCase();

    if (seen.has(key)) {
      duplicates.add(code);
      continue;
    }

    seen.add(key);
  }

  if (duplicates.size > 0) {
    throw new ApiError(
      400,
      "duplicate_asset_codes",
      "Duplicate asset codes are not allowed within the same request.",
      {
        duplicates: Array.from(duplicates),
      },
    );
  }

  return normalized;
}

function ensurePendingRequestState(
  status: "PENDING" | "APPROVED" | "REJECTED",
  approvalReviewId?: number | null,
) {
  if (status !== "PENDING" || approvalReviewId) {
    throw new ApiError(
      409,
      "request_not_pending",
      "This request has already been reviewed.",
    );
  }
}

function resolveReviewedEmployeeId(
  submittedEmployeeId: string,
  reviewedEmployeeId?: string,
) {
  return reviewedEmployeeId?.trim() || submittedEmployeeId;
}

function resolveReviewedAssetCodes(
  submittedAssetCodes: string[],
  reviewedAssetCodes?: string[],
) {
  return normalizeAssetCodes(reviewedAssetCodes ?? submittedAssetCodes);
}

export async function createReceiveSession(
  actor: ApiActor,
  input: CreateSessionInput,
) {
  return prisma.$transaction(async (tx) => {
    const now = new Date();
    const activeSession = await findActiveReceiveSession(tx, now);

    if (activeSession) {
      throw new ApiError(
        409,
        "receive_session_already_active",
        "An active receive session already exists.",
      );
    }

    const session = await createReceiveSessionRecord(tx, {
      createdByAccountId: actor.accountId,
      expiresAt: input.expiresAt,
      notes: input.notes,
    });

    await writeAuditLog(tx, {
      actor,
      actionType: "receive_session.create",
      entityType: "receive_session",
      entityId: session.sessionKey,
      entityLabel: session.qrToken,
      metadata: {
        status: session.status,
        expiresAt: session.expiresAt?.toISOString(),
      },
    });

    return session;
  });
}

export async function createReturnSession(
  actor: ApiActor,
  input: CreateSessionInput,
) {
  return prisma.$transaction(async (tx) => {
    const session = await createReturnSessionRecord(tx, {
      createdByAccountId: actor.accountId,
      expiresAt: input.expiresAt,
      notes: input.notes,
    });

    await writeAuditLog(tx, {
      actor,
      actionType: "return_session.create",
      entityType: "return_session",
      entityId: session.sessionKey,
      entityLabel: session.qrToken,
      metadata: {
        status: session.status,
        expiresAt: session.expiresAt?.toISOString(),
      },
    });

    return session;
  });
}

export async function submitReceiveRequest(input: SubmitReceiveRequestInput) {
  // Pre-flight validation outside tx (fast rejection before acquiring connection)
  const assetCodes = normalizeAssetCodes(input.assetCodes);

  const employee = await findEmployeeByEmployeeId(prisma, input.employeeId);

  if (!employee) {
    throw new ApiError(404, "employee_not_found", `Employee ${input.employeeId} was not found.`);
  }

  const assets = await findAssetsByAssetCodes(prisma, assetCodes);
  const assetMap = new Map(assets.map((asset) => [asset.assetCode, asset]));
  const missingCodes = assetCodes.filter((code) => !assetMap.has(code));

  if (missingCodes.length > 0) {
    throw new ApiError(400, "unknown_asset_codes", "Some asset codes do not exist.", {
      missingCodes,
    });
  }

  const nonAssignableAssets = assets.filter((asset) => asset.status !== "IN_STOCK");

  if (nonAssignableAssets.length > 0) {
    throw new ApiError(
      409,
      "non_assignable_assets",
      "Some assets are not eligible for receive.",
      {
        assetCodes: nonAssignableAssets.map((asset) => asset.assetCode),
      },
    );
  }

  return prisma.$transaction(async (tx) => {
    // P1 fix: re-validate session inside the transaction so the check is
    // atomic — concurrent submits serialise here and at most one can proceed.
    const session = await findActiveReceiveSession(tx, new Date());

    if (!session) {
      throw new ApiError(
        403,
        "workflow_session_inactive",
        "This QR link is no longer active. Please ask IT to refresh it.",
      );
    }

    const request = await createReceiveRequestRecord(tx, {
      sessionId: session.id,
      employeeId: employee.id,
      employeeCodeSnapshot: employee.employeeId,
      employeeNameSnapshot: employee.fullName,
      notes: input.notes,
      items: assetCodes.map((code) => {
        const asset = assetMap.get(code);

        if (!asset) {
          throw new ApiError(500, "asset_resolution_failed", "Asset lookup failed unexpectedly.");
        }

        return {
          assetId: asset.id,
          assetCodeSnapshot: asset.assetCode,
          assetNameSnapshot: asset.name,
        };
      }),
    });

    await writeAuditLog(tx, {
      actionType: "receive_request.submit",
      entityType: "receive_request",
      entityId: request.requestKey,
      entityLabel: request.requestKey,
      receiveRequestId: request.id,
      employeeId: employee.id,
      metadata: {
        sessionKey: session.sessionKey,
        assetCodes,
      },
    });

    return request;
  });
}

export async function submitReturnRequest(input: SubmitReturnRequestInput) {
  const now = new Date();
  const assetCodes = normalizeAssetCodes(input.assetCodes);
  const session = await findActiveReturnSessionByQrToken(prisma, input.qrToken, now);

  if (!session) {
    throw new ApiError(404, "return_session_not_found", "Return session is invalid or expired.");
  }

  const employee = await findEmployeeByEmployeeId(prisma, input.employeeId);

  if (!employee) {
    throw new ApiError(404, "employee_not_found", `Employee ${input.employeeId} was not found.`);
  }

  const assignments = await findActiveAssignmentsByAssetCodes(prisma, assetCodes);
  const assignmentMap = new Map(assignments.map((assignment) => [assignment.asset.assetCode, assignment]));
  const missingCodes = assetCodes.filter((code) => !assignmentMap.has(code));

  if (missingCodes.length > 0) {
    throw new ApiError(
      409,
      "return_assets_not_eligible",
      "Some assets are not currently assigned and eligible for return.",
      {
        assetCodes: missingCodes,
      },
    );
  }

  const wrongEmployeeAssignments = assignments.filter(
    (assignment) => assignment.employeeId !== employee.id,
  );

  if (wrongEmployeeAssignments.length > 0) {
    throw new ApiError(
      409,
      "asset_not_assigned_to_employee",
      "Some assets are not assigned to the submitted employee.",
      {
        assetCodes: wrongEmployeeAssignments.map((assignment) => assignment.asset.assetCode),
      },
    );
  }

  return prisma.$transaction(async (tx) => {
    const request = await createReturnRequestRecord(tx, {
      sessionId: session.id,
      employeeId: employee.id,
      employeeCodeSnapshot: employee.employeeId,
      employeeNameSnapshot: employee.fullName,
      notes: input.notes,
      items: assetCodes.map((code) => {
        const assignment = assignmentMap.get(code);

        if (!assignment) {
          throw new ApiError(
            500,
            "assignment_resolution_failed",
            "Assignment lookup failed unexpectedly.",
          );
        }

        return {
          assetId: assignment.assetId,
          assetAssignmentId: assignment.id,
          assetCodeSnapshot: assignment.asset.assetCode,
          assetNameSnapshot: assignment.asset.name,
        };
      }),
    });

    await writeAuditLog(tx, {
      actionType: "return_request.submit",
      entityType: "return_request",
      entityId: request.requestKey,
      entityLabel: request.requestKey,
      returnRequestId: request.id,
      employeeId: employee.id,
      metadata: {
        sessionKey: session.sessionKey,
        assetCodes,
      },
    });

    return request;
  });
}

async function resolveReceiveReviewPayload(
  tx: Parameters<Parameters<typeof prisma.$transaction>[0]>[0],
  request: NonNullable<Awaited<ReturnType<typeof findReceiveRequestForReview>>>,
  input: ReviewPendingRequestInput,
) {
  const submittedAssetCodes = request.items.map((item) => item.assetCodeSnapshot);
  const reviewedEmployeeId = resolveReviewedEmployeeId(
    request.employee.employeeId,
    input.reviewedEmployeeId,
  );
  const reviewedAssetCodes = resolveReviewedAssetCodes(
    submittedAssetCodes,
    input.reviewedAssetCodes,
  );

  const reviewedEmployee = await findEmployeeByEmployeeId(tx, reviewedEmployeeId);

  if (!reviewedEmployee) {
    throw new ApiError(
      404,
      "review_employee_not_found",
      `Employee ${reviewedEmployeeId} was not found.`,
    );
  }

  const reviewedAssets = await findAssetsByAssetCodes(tx, reviewedAssetCodes);
  const reviewedAssetMap = new Map(reviewedAssets.map((asset) => [asset.assetCode, asset]));
  const missingCodes = reviewedAssetCodes.filter((code) => !reviewedAssetMap.has(code));

  if (missingCodes.length > 0) {
    throw new ApiError(
      400,
      "unknown_reviewed_asset_codes",
      "Some reviewed asset codes do not exist.",
      {
        missingCodes,
      },
    );
  }

  return {
    reviewedEmployee,
    reviewedEmployeeId,
    reviewedAssetCodes,
    reviewedAssets,
    submittedAssetCodes,
  };
}

async function approveReceiveRequest(
  actor: ApiActor,
  requestKey: string,
  input: ReviewPendingRequestInput,
) {
  return prisma.$transaction(async (tx) => {
    const request = await findReceiveRequestForReview(tx, requestKey);

    if (!request) {
      throw new ApiError(404, "receive_request_not_found", "Receive request was not found.");
    }

    ensurePendingRequestState(request.status, request.approvalReview?.id);

    const reviewedPayload = await resolveReceiveReviewPayload(tx, request, input);
    const assetIds = reviewedPayload.reviewedAssets.map((asset) => asset.id);
    const conflictingAssets = await tx.asset.findMany({
      where: {
        id: {
          in: assetIds,
        },
        NOT: {
          status: "IN_STOCK",
        },
      },
    });

    if (conflictingAssets.length > 0) {
      throw new ApiError(
        409,
        "receive_assets_conflict",
        "Some assets are no longer assignable.",
        {
          assetCodes: conflictingAssets.map((asset) => asset.assetCode),
        },
      );
    }

    const activeAssignments = await tx.assetAssignment.findMany({
      where: {
        assetId: {
          in: assetIds,
        },
        status: "ACTIVE",
      },
    });

    if (activeAssignments.length > 0) {
      throw new ApiError(
        409,
        "receive_assets_already_assigned",
        "Some assets are already assigned.",
      );
    }

    const requestItemsByAssetCode = new Map(
      request.items.map((item) => [item.assetCodeSnapshot, item]),
    );

    for (const asset of reviewedPayload.reviewedAssets) {
      const receiveRequestItem = requestItemsByAssetCode.get(asset.assetCode);

      await tx.assetAssignment.create({
        data: {
          assetId: asset.id,
          employeeId: reviewedPayload.reviewedEmployee.id,
          status: "ACTIVE",
          receiveRequestItemId: receiveRequestItem?.id,
          assignedAt: new Date(),
        },
      });

      await tx.asset.update({
        where: {
          id: asset.id,
        },
        data: {
          status: "ASSIGNED",
        },
      });
    }

    const approvalReview = await tx.approvalReview.create({
      data: {
        requestType: "RECEIVE",
        reviewerAccountId: actor.accountId,
        decision: "APPROVED",
        receiveRequestId: request.id,
        notes: input.notes,
      },
    });

    const updatedRequest = await tx.receiveRequest.update({
      where: {
        id: request.id,
      },
      data: {
        status: "APPROVED",
        finalizedAt: new Date(),
      },
      include: {
        items: {
          orderBy: {
            assetCodeSnapshot: "asc",
          },
        },
      },
    });

    await writeAuditLog(tx, {
      actor,
      actionType: "approval.review",
      entityType: "receive_request",
      entityId: updatedRequest.requestKey,
      entityLabel: updatedRequest.requestKey,
      result: "SUCCESS",
      receiveRequestId: updatedRequest.id,
      employeeId: updatedRequest.employeeId,
      approvalReviewId: approvalReview.id,
      metadata: {
        decision: "APPROVED",
        requestType: "RECEIVE",
        submittedEmployeeId: request.employee.employeeId,
        reviewedEmployeeId: reviewedPayload.reviewedEmployee.employeeId,
        submittedAssetCodes: reviewedPayload.submittedAssetCodes,
        reviewedAssetCodes: reviewedPayload.reviewedAssetCodes,
      },
    });

    return {
      requestType: "RECEIVE" as const,
      status: updatedRequest.status,
      approvalDecision: approvalReview.decision,
      requestKey: updatedRequest.requestKey,
      approvalReviewKey: approvalReview.reviewKey,
    };
  });
}

async function rejectReceiveRequest(
  actor: ApiActor,
  requestKey: string,
  input: ReviewPendingRequestInput,
) {
  return prisma.$transaction(async (tx) => {
    const request = await findReceiveRequestForReview(tx, requestKey);

    if (!request) {
      throw new ApiError(404, "receive_request_not_found", "Receive request was not found.");
    }

    ensurePendingRequestState(request.status, request.approvalReview?.id);
    const reviewedPayload = await resolveReceiveReviewPayload(tx, request, input);

    const approvalReview = await tx.approvalReview.create({
      data: {
        requestType: "RECEIVE",
        reviewerAccountId: actor.accountId,
        decision: "REJECTED",
        receiveRequestId: request.id,
        notes: input.notes,
      },
    });

    const updatedRequest = await tx.receiveRequest.update({
      where: {
        id: request.id,
      },
      data: {
        status: "REJECTED",
        finalizedAt: new Date(),
      },
      include: {
        items: {
          orderBy: {
            assetCodeSnapshot: "asc",
          },
        },
      },
    });

    await writeAuditLog(tx, {
      actor,
      actionType: "approval.review",
      entityType: "receive_request",
      entityId: updatedRequest.requestKey,
      entityLabel: updatedRequest.requestKey,
      result: "REJECTED",
      reason: input.notes,
      receiveRequestId: updatedRequest.id,
      employeeId: updatedRequest.employeeId,
      approvalReviewId: approvalReview.id,
      metadata: {
        decision: "REJECTED",
        requestType: "RECEIVE",
        submittedEmployeeId: request.employee.employeeId,
        reviewedEmployeeId: reviewedPayload.reviewedEmployee.employeeId,
        submittedAssetCodes: reviewedPayload.submittedAssetCodes,
        reviewedAssetCodes: reviewedPayload.reviewedAssetCodes,
      },
    });

    return {
      requestType: "RECEIVE" as const,
      status: updatedRequest.status,
      approvalDecision: approvalReview.decision,
      requestKey: updatedRequest.requestKey,
      approvalReviewKey: approvalReview.reviewKey,
    };
  });
}

async function resolveReturnReviewPayload(
  tx: Parameters<Parameters<typeof prisma.$transaction>[0]>[0],
  request: NonNullable<Awaited<ReturnType<typeof findReturnRequestForReview>>>,
  input: ReviewPendingRequestInput,
) {
  const submittedAssetCodes = request.items.map((item) => item.assetCodeSnapshot);
  const reviewedEmployeeId = resolveReviewedEmployeeId(
    request.employee.employeeId,
    input.reviewedEmployeeId,
  );
  const reviewedAssetCodes = resolveReviewedAssetCodes(
    submittedAssetCodes,
    input.reviewedAssetCodes,
  );

  const reviewedEmployee = await findEmployeeByEmployeeId(tx, reviewedEmployeeId);

  if (!reviewedEmployee) {
    throw new ApiError(
      404,
      "review_employee_not_found",
      `Employee ${reviewedEmployeeId} was not found.`,
    );
  }

  if (reviewedAssetCodes.length === 0) {
    throw new ApiError(
      409,
      "return_review_empty_asset_set",
      "Return review must keep at least one asset.",
    );
  }

  const submittedItemsByAssetCode = new Map(
    request.items.map((item) => [item.assetCodeSnapshot, item]),
  );
  const addedAssetCodes = reviewedAssetCodes.filter((code) => !submittedItemsByAssetCode.has(code));

  if (addedAssetCodes.length > 0) {
    throw new ApiError(
      409,
      "return_review_asset_add_not_allowed",
      "Return review cannot add new assets that were not submitted.",
      {
        assetCodes: addedAssetCodes,
      },
    );
  }

  const reviewedItems = reviewedAssetCodes.map((code) => {
    const item = submittedItemsByAssetCode.get(code);

    if (!item) {
      throw new ApiError(
        500,
        "return_review_item_resolution_failed",
        "Return review item lookup failed unexpectedly.",
      );
    }

    return item;
  });

  const wrongEmployeeAssignments = reviewedItems.filter(
    (item) => item.assetAssignment.employee.employeeId !== reviewedEmployee.employeeId,
  );

  if (wrongEmployeeAssignments.length > 0) {
    throw new ApiError(
      409,
      "return_review_employee_conflict",
      "Some reviewed assets are not assigned to the reviewed employee.",
      {
        assetCodes: wrongEmployeeAssignments.map((item) => item.assetCodeSnapshot),
      },
    );
  }

  return {
    reviewedEmployee,
    reviewedEmployeeId,
    reviewedAssetCodes,
    reviewedItems,
    submittedAssetCodes,
  };
}

async function approveReturnRequest(
  actor: ApiActor,
  requestKey: string,
  input: ReviewPendingRequestInput,
) {
  return prisma.$transaction(async (tx) => {
    const request = await findReturnRequestForReview(tx, requestKey);

    if (!request) {
      throw new ApiError(404, "return_request_not_found", "Return request was not found.");
    }

    ensurePendingRequestState(request.status, request.approvalReview?.id);
    const reviewedPayload = await resolveReturnReviewPayload(tx, request, input);

    const inactiveAssignments = reviewedPayload.reviewedItems.filter(
      (item) => item.assetAssignment.status !== "ACTIVE",
    );

    if (inactiveAssignments.length > 0) {
      throw new ApiError(
        409,
        "return_assignments_conflict",
        "Some assignments are no longer active.",
        {
          assetCodes: inactiveAssignments.map((item) => item.assetCodeSnapshot),
        },
      );
    }

    for (const item of reviewedPayload.reviewedItems) {
      await tx.assetAssignment.update({
        where: {
          id: item.assetAssignmentId,
        },
        data: {
          status: "RETURNED",
          returnedAt: new Date(),
          closedByReturnItemId: item.id,
        },
      });

      await tx.asset.update({
        where: {
          id: item.assetId,
        },
        data: {
          status: "IN_STOCK",
        },
      });
    }

    const approvalReview = await tx.approvalReview.create({
      data: {
        requestType: "RETURN",
        reviewerAccountId: actor.accountId,
        decision: "APPROVED",
        returnRequestId: request.id,
        notes: input.notes,
      },
    });

    const updatedRequest = await tx.returnRequest.update({
      where: {
        id: request.id,
      },
      data: {
        status: "APPROVED",
        finalizedAt: new Date(),
      },
      include: {
        items: {
          orderBy: {
            assetCodeSnapshot: "asc",
          },
        },
      },
    });

    await writeAuditLog(tx, {
      actor,
      actionType: "approval.review",
      entityType: "return_request",
      entityId: updatedRequest.requestKey,
      entityLabel: updatedRequest.requestKey,
      result: "SUCCESS",
      returnRequestId: updatedRequest.id,
      employeeId: updatedRequest.employeeId,
      approvalReviewId: approvalReview.id,
      metadata: {
        decision: "APPROVED",
        requestType: "RETURN",
        submittedEmployeeId: request.employee.employeeId,
        reviewedEmployeeId: reviewedPayload.reviewedEmployee.employeeId,
        submittedAssetCodes: reviewedPayload.submittedAssetCodes,
        reviewedAssetCodes: reviewedPayload.reviewedAssetCodes,
      },
    });

    return {
      requestType: "RETURN" as const,
      status: updatedRequest.status,
      approvalDecision: approvalReview.decision,
      requestKey: updatedRequest.requestKey,
      approvalReviewKey: approvalReview.reviewKey,
    };
  });
}

async function rejectReturnRequest(
  actor: ApiActor,
  requestKey: string,
  input: ReviewPendingRequestInput,
) {
  return prisma.$transaction(async (tx) => {
    const request = await findReturnRequestForReview(tx, requestKey);

    if (!request) {
      throw new ApiError(404, "return_request_not_found", "Return request was not found.");
    }

    ensurePendingRequestState(request.status, request.approvalReview?.id);
    const reviewedPayload = await resolveReturnReviewPayload(tx, request, input);

    const approvalReview = await tx.approvalReview.create({
      data: {
        requestType: "RETURN",
        reviewerAccountId: actor.accountId,
        decision: "REJECTED",
        returnRequestId: request.id,
        notes: input.notes,
      },
    });

    const updatedRequest = await tx.returnRequest.update({
      where: {
        id: request.id,
      },
      data: {
        status: "REJECTED",
        finalizedAt: new Date(),
      },
      include: {
        items: {
          orderBy: {
            assetCodeSnapshot: "asc",
          },
        },
      },
    });

    await writeAuditLog(tx, {
      actor,
      actionType: "approval.review",
      entityType: "return_request",
      entityId: updatedRequest.requestKey,
      entityLabel: updatedRequest.requestKey,
      result: "REJECTED",
      reason: input.notes,
      returnRequestId: updatedRequest.id,
      employeeId: updatedRequest.employeeId,
      approvalReviewId: approvalReview.id,
      metadata: {
        decision: "REJECTED",
        requestType: "RETURN",
        submittedEmployeeId: request.employee.employeeId,
        reviewedEmployeeId: reviewedPayload.reviewedEmployee.employeeId,
        submittedAssetCodes: reviewedPayload.submittedAssetCodes,
        reviewedAssetCodes: reviewedPayload.reviewedAssetCodes,
      },
    });

    return {
      requestType: "RETURN" as const,
      status: updatedRequest.status,
      approvalDecision: approvalReview.decision,
      requestKey: updatedRequest.requestKey,
      approvalReviewKey: approvalReview.reviewKey,
    };
  });
}

export async function reviewPendingRequest(
  actor: ApiActor,
  input: ReviewPendingRequestInput,
) {
  if (input.decision === "REJECTED" && !input.notes?.trim()) {
    throw new ApiError(
      400,
      "review_notes_required",
      "Review notes are required when rejecting a pending request.",
    );
  }

  if (input.requestType === "RECEIVE") {
    return input.decision === "APPROVED"
      ? approveReceiveRequest(actor, input.requestKey, input)
      : rejectReceiveRequest(actor, input.requestKey, input);
  }

  return input.decision === "APPROVED"
    ? approveReturnRequest(actor, input.requestKey, input)
    : rejectReturnRequest(actor, input.requestKey, input);
}

export async function getReviewRequestDetail(
  requestType: WorkflowRequestType,
  requestKey: string,
): Promise<ReviewRequestDetail> {
  if (requestType === "RECEIVE") {
    const request = await findReceiveRequestForReview(prisma, requestKey);

    if (!request) {
      throw new ApiError(404, "receive_request_not_found", "Receive request was not found.");
    }

    if (request.status !== "PENDING") {
      throw new ApiError(409, "request_not_pending", "This request is no longer pending.");
    }

    const assetCodes = request.items.map((item) => item.assetCodeSnapshot);

    return {
      requestType,
      requestKey: request.requestKey,
      status: request.status,
      submittedAt: request.submittedAt,
      submittedNotes: request.notes,
      submittedSnapshot: {
        employeeId: request.employee.employeeId,
        employeeName: request.employee.fullName,
        assetCodes,
        items: request.items.map((item) => ({
          assetCode: item.assetCodeSnapshot,
          assetName: item.assetNameSnapshot,
        })),
      },
      reviewedDraft: {
        employeeId: request.employee.employeeId,
        assetCodes,
        notes: undefined,
      },
      rules: {
        canAddAssets: true,
        canRemoveAssets: true,
      },
    };
  }

  const request = await findReturnRequestForReview(prisma, requestKey);

  if (!request) {
    throw new ApiError(404, "return_request_not_found", "Return request was not found.");
  }

  if (request.status !== "PENDING") {
    throw new ApiError(409, "request_not_pending", "This request is no longer pending.");
  }

  const assetCodes = request.items.map((item) => item.assetCodeSnapshot);

  return {
    requestType,
    requestKey: request.requestKey,
    status: request.status,
    submittedAt: request.submittedAt,
    submittedNotes: request.notes,
    submittedSnapshot: {
      employeeId: request.employee.employeeId,
      employeeName: request.employee.fullName,
      assetCodes,
      items: request.items.map((item) => ({
        assetCode: item.assetCodeSnapshot,
        assetName: item.assetNameSnapshot,
        assignmentEmployeeId: item.assetAssignment.employee.employeeId,
        assignmentEmployeeName: item.assetAssignment.employee.fullName,
      })),
    },
    reviewedDraft: {
      employeeId: request.employee.employeeId,
      assetCodes,
      notes: undefined,
    },
    rules: {
      canAddAssets: false,
      canRemoveAssets: true,
    },
  };
}

export async function getPendingRequests(filters: PendingRequestFiltersInput) {
  const take = filters.take ?? 25;

  if (filters.requestType === "RECEIVE") {
    const receiveRequests = await listPendingReceiveRequests(prisma, take);

    return receiveRequests.map((request) => ({
      requestType: "RECEIVE" as WorkflowRequestType,
      requestKey: request.requestKey,
      status: request.status,
      submittedAt: request.submittedAt,
      employee: request.employee,
      assetCodes: request.items.map((item) => item.assetCodeSnapshot),
    }));
  }

  if (filters.requestType === "RETURN") {
    const returnRequests = await listPendingReturnRequests(prisma, take);

    return returnRequests.map((request) => ({
      requestType: "RETURN" as WorkflowRequestType,
      requestKey: request.requestKey,
      status: request.status,
      submittedAt: request.submittedAt,
      employee: request.employee,
      assetCodes: request.items.map((item) => item.assetCodeSnapshot),
    }));
  }

  const [receiveRequests, returnRequests] = await Promise.all([
    listPendingReceiveRequests(prisma, take),
    listPendingReturnRequests(prisma, take),
  ]);

  return [
    ...receiveRequests.map((request) => ({
      requestType: "RECEIVE" as WorkflowRequestType,
      requestKey: request.requestKey,
      status: request.status,
      submittedAt: request.submittedAt,
      employee: request.employee,
      assetCodes: request.items.map((item) => item.assetCodeSnapshot),
    })),
    ...returnRequests.map((request) => ({
      requestType: "RETURN" as WorkflowRequestType,
      requestKey: request.requestKey,
      status: request.status,
      submittedAt: request.submittedAt,
      employee: request.employee,
      assetCodes: request.items.map((item) => item.assetCodeSnapshot),
    })),
  ]
    .sort((left, right) => right.submittedAt.getTime() - left.submittedAt.getTime())
    .slice(0, take);
}
