import type { Prisma } from "@/generated/prisma/client";

import { prisma } from "@/lib/prisma";

type AuditRepositoryClient = Pick<typeof prisma, "auditLog">;

export type CreateAuditLogInput = {
  actorAccountId?: number;
  actorUsername?: string | null;
  actionType: string;
  entityType: string;
  entityId?: string | null;
  entityLabel?: string | null;
  result?: "SUCCESS" | "FAILURE" | "REJECTED" | "DENIED";
  reason?: string | null;
  metadata?: Prisma.InputJsonValue;
  assetId?: number | null;
  employeeId?: number | null;
  receiveRequestId?: number | null;
  returnRequestId?: number | null;
  approvalReviewId?: number | null;
  occurredAt?: Date;
};

export type AuditLogFilters = {
  actorUsername?: string;
  actionType?: string;
  entityType?: string;
  result?: "SUCCESS" | "FAILURE" | "REJECTED" | "DENIED";
  from?: Date;
  to?: Date;
  take?: number;
};

export async function createAuditLog(
  db: AuditRepositoryClient,
  input: CreateAuditLogInput,
) {
  return db.auditLog.create({
    data: {
      actorAccountId: input.actorAccountId,
      actorUsername: input.actorUsername ?? undefined,
      actionType: input.actionType,
      entityType: input.entityType,
      entityId: input.entityId ?? undefined,
      entityLabel: input.entityLabel ?? undefined,
      result: input.result ?? "SUCCESS",
      reason: input.reason ?? undefined,
      metadata: input.metadata,
      assetId: input.assetId ?? undefined,
      employeeId: input.employeeId ?? undefined,
      receiveRequestId: input.receiveRequestId ?? undefined,
      returnRequestId: input.returnRequestId ?? undefined,
      approvalReviewId: input.approvalReviewId ?? undefined,
      occurredAt: input.occurredAt,
    },
  });
}

export async function listAuditLogs(
  db: AuditRepositoryClient,
  filters: AuditLogFilters,
) {
  return db.auditLog.findMany({
    where: {
      actorUsername: filters.actorUsername
        ? {
            equals: filters.actorUsername,
            mode: "insensitive",
          }
        : undefined,
      actionType: filters.actionType
        ? {
            contains: filters.actionType,
            mode: "insensitive",
          }
        : undefined,
      entityType: filters.entityType
        ? {
            equals: filters.entityType,
            mode: "insensitive",
          }
        : undefined,
      result: filters.result,
      occurredAt:
        filters.from || filters.to
          ? {
              gte: filters.from,
              lte: filters.to,
            }
          : undefined,
    },
    orderBy: {
      occurredAt: "desc",
    },
    take: Math.min(filters.take ?? 50, 200),
    include: {
      actorAccount: {
        select: {
          id: true,
          username: true,
          displayName: true,
          role: true,
        },
      },
    },
  });
}
