import { getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { requireApiActor } from "@/lib/api/auth";
import { reviewPendingRequest } from "@/lib/workflows/workflows.service";
import { reviewPendingRequestSchema } from "@/lib/workflows/workflows.schemas";

export const dynamic = "force-dynamic";

export async function POST(request: Request) {
  const requestId = getRequestId(request);

  try {
    const actor = await requireApiActor("ADMIN");
    const input = await readJsonBody(request, reviewPendingRequestSchema);
    const review = await reviewPendingRequest(actor, input);

    return jsonResponse({
      requestId,
      data: review,
    });
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "workflows",
      action: "approval.review",
    });
  }
}
