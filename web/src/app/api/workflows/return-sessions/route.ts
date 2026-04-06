import { getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { requireApiActor } from "@/lib/api/auth";
import { createReturnSession } from "@/lib/workflows/workflows.service";
import { createSessionSchema } from "@/lib/workflows/workflows.schemas";

export const dynamic = "force-dynamic";

export async function POST(request: Request) {
  const requestId = getRequestId(request);

  try {
    const actor = await requireApiActor("ADMIN");
    const input = await readJsonBody(request, createSessionSchema);
    const session = await createReturnSession(actor, input);

    return jsonResponse(
      {
        requestId,
        data: session,
      },
      201,
    );
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "workflows",
      action: "return_session.create",
    });
  }
}
