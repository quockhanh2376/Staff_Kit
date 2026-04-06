import type { Prisma } from "@/generated/prisma/client";

import { prisma } from "@/lib/prisma";
import type { ApiActor } from "@/lib/api/auth";

import {
  createAuditLog,
  listAuditLogs,
  type AuditLogFilters,
  type CreateAuditLogInput,
} from "./audit.repository";

type AuditWriteClient = Pick<typeof prisma, "auditLog">;

export type AuditLogWriteInput = Omit<
  CreateAuditLogInput,
  "actorAccountId" | "actorUsername"
> & {
  actor?: ApiActor | null;
  metadata?: Prisma.InputJsonValue;
};

export async function writeAuditLog(
  db: AuditWriteClient,
  input: AuditLogWriteInput,
) {
  return createAuditLog(db, {
    ...input,
    actorAccountId: input.actor?.accountId,
    actorUsername: input.actor?.username,
  });
}

export async function writeAuditLogs(
  db: AuditWriteClient,
  inputs: AuditLogWriteInput[],
) {
  return Promise.all(inputs.map((input) => writeAuditLog(db, input)));
}

export async function getAuditLogs(filters: AuditLogFilters) {
  return listAuditLogs(prisma, filters);
}
