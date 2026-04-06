import { getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { requireApiActor } from "@/lib/api/auth";
import { assetPreloadSchema } from "@/lib/assets/assets.schemas";
import { preloadAssets } from "@/lib/assets/assets.service";

export const dynamic = "force-dynamic";

export async function POST(request: Request) {
  const requestId = getRequestId(request);

  try {
    const actor = await requireApiActor("ADMIN");
    const input = await readJsonBody(request, assetPreloadSchema);
    const result = await preloadAssets(actor, input);

    return jsonResponse(
      {
        requestId,
        data: result,
      },
      201,
    );
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "assets",
      action: "preload",
    });
  }
}
