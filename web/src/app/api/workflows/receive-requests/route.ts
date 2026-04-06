import { getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { submitReceiveRequest } from "@/lib/workflows/workflows.service";
import { submitReceiveRequestSchema } from "@/lib/workflows/workflows.schemas";

export const dynamic = "force-dynamic";

export async function POST(request: Request) {
  const requestId = getRequestId(request);

  try {
    const input = await readJsonBody(request, submitReceiveRequestSchema);
    const receiveRequest = await submitReceiveRequest(input);

    return jsonResponse(
      {
        requestId,
        data: receiveRequest,
      },
      201,
    );
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "workflows",
      action: "receive_request.submit",
    });
  }
}
