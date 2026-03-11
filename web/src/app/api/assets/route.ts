import { ApiError, getRequestId, handleRouteError, jsonResponse, readJsonBody } from "@/lib/api/errors";
import { requireApiActor } from "@/lib/api/auth";
import {
  assetCreateSchema,
  assetListFiltersSchema,
} from "@/lib/assets/assets.schemas";
import { createAssetRecord, getAssets } from "@/lib/assets/assets.service";

export const dynamic = "force-dynamic";

export async function GET(request: Request) {
  const requestId = getRequestId(request);

  try {
    await requireApiActor("ADMIN");

    const parsed = assetListFiltersSchema.safeParse(
      Object.fromEntries(new URL(request.url).searchParams.entries()),
    );

    if (!parsed.success) {
      throw new ApiError(400, "validation_error", "Query validation failed.", parsed.error.flatten());
    }

    const assets = await getAssets(parsed.data);

    return jsonResponse({
      requestId,
      data: assets,
    });
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "assets",
      action: "list",
    });
  }
}

export async function POST(request: Request) {
  const requestId = getRequestId(request);

  try {
    const actor = await requireApiActor("ADMIN");
    const input = await readJsonBody(request, assetCreateSchema);
    const asset = await createAssetRecord(actor, input);

    return jsonResponse(
      {
        requestId,
        data: asset,
      },
      201,
    );
  } catch (error) {
    return handleRouteError(error, requestId, {
      feature: "assets",
      action: "create",
    });
  }
}
