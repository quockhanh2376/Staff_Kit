import { ApiError, getRequestId, handleRouteError, jsonResponse } from "@/lib/api/errors";
import { requireApiActor } from "@/lib/api/auth";
import { getPendingRequests } from "@/lib/workflows/workflows.service";
import { pendingRequestFiltersSchema } from "@/lib/workflows/workflows.schemas";

export const dynamic = "force-dynamic";

export async function GET(request: Request) {
  const requestId = getRequestId(request);

  try {
    await requireApiActor("ADMIN");

    const parsed = pendingRequestFiltersSchema.safeParse(
      Object.fromEntries(new URL(request.url).searchParams.entries()),
    );

    if (!parsed.success) {
      throw new ApiError(400, "validation_error", "Query validation failed.", parsed.error.flatten());
    }

    const pendingRequests = await getPendingRequests(parsed.data);

    return jsonResponse({
      requestId,
      data: pendingRequests,
    });
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "workflows",
      action: "pending_requests.list",
    });
  }
}
