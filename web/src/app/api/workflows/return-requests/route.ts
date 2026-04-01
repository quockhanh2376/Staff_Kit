import { getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { submitReturnRequest } from "@/lib/workflows/workflows.service";
import { submitReturnRequestSchema } from "@/lib/workflows/workflows.schemas";

export const dynamic = "force-dynamic";

export async function POST(request: Request) {
  const requestId = getRequestId(request);

  try {
    const input = await readJsonBody(request, submitReturnRequestSchema);
    const returnRequest = await submitReturnRequest(input);

    return jsonResponse(
      {
        requestId,
        data: returnRequest,
      },
      201,
    );
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "workflows",
      action: "return_request.submit",
    });
  }
}
