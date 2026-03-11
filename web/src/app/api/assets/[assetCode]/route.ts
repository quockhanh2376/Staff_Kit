import { getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { requireApiActor } from "@/lib/api/auth";
import { assetUpdateSchema } from "@/lib/assets/assets.schemas";
import { getAssetByCode, updateAssetRecord } from "@/lib/assets/assets.service";

export const dynamic = "force-dynamic";

type RouteContext = {
  params: Promise<{
    assetCode: string;
  }>;
};

export async function GET(request: Request, context: RouteContext) {
  const requestId = getRequestId(request);

  try {
    await requireApiActor("ADMIN");
    const { assetCode } = await context.params;
    const asset = await getAssetByCode(assetCode);

    return jsonResponse({
      requestId,
      data: asset,
    });
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "assets",
      action: "detail",
    });
  }
}

export async function PATCH(request: Request, context: RouteContext) {
  const requestId = getRequestId(request);

  try {
    const actor = await requireApiActor("ADMIN");
    const { assetCode } = await context.params;
    const input = await readJsonBody(request, assetUpdateSchema);
    const asset = await updateAssetRecord(actor, assetCode, input);

    return jsonResponse({
      requestId,
      data: asset,
    });
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "assets",
      action: "update",
    });
  }
}
