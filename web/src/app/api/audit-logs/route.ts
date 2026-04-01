import { z } from "zod";

import { requireApiActor } from "@/lib/api/auth";
import { ApiError, getRequestId, handleRouteError, jsonResponse } from "@/lib/api/errors";
import { getAuditLogs } from "@/lib/audit/audit.service";

export const dynamic = "force-dynamic";

const auditLogQuerySchema = z.object({
  actorUsername: z.string().trim().min(1).optional(),
  actionType: z.string().trim().min(1).optional(),
  entityType: z.string().trim().min(1).optional(),
  result: z.enum(["SUCCESS", "FAILURE", "REJECTED", "DENIED"]).optional(),
  from: z.coerce.date().optional(),
  to: z.coerce.date().optional(),
  take: z.coerce.number().int().min(1).max(200).optional(),
});

export async function GET(request: Request) {
  const requestId = getRequestId(request);

  try {
    await requireApiActor("ADMIN");

    const parsed = auditLogQuerySchema.safeParse(
      Object.fromEntries(new URL(request.url).searchParams.entries()),
    );

    if (!parsed.success) {
      throw new ApiError(400, "validation_error", "Query validation failed.", parsed.error.flatten());
    }

    const auditLogs = await getAuditLogs(parsed.data);

    return jsonResponse({
      requestId,
      data: auditLogs,
    });
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "audit",
      action: "list",
    });
  }
}
