import { NextResponse } from "next/server";
import { ZodError, type ZodType } from "zod";

import { logEvent } from "@/lib/logger";

type RouteMeta = {
  feature: string;
  action: string;
};

export class ApiError extends Error {
  statusCode: number;
  code: string;
  details?: unknown;

  constructor(statusCode: number, code: string, message: string, details?: unknown) {
    super(message);
    this.name = "ApiError";
    this.statusCode = statusCode;
    this.code = code;
    this.details = details;
  }
}

export function getRequestId(request: Request) {
  return request.headers.get("x-request-id") ?? crypto.randomUUID();
}

export async function readJsonBody<T>(
  request: Request,
  schema: ZodType<T>,
): Promise<T> {
  let rawBody: unknown;

  try {
    rawBody = await request.json();
  } catch {
    throw new ApiError(400, "invalid_json", "Request body must be valid JSON.");
  }

  try {
    return schema.parse(rawBody);
  } catch (error) {
    if (error instanceof ZodError) {
      throw new ApiError(
        400,
        "validation_error",
        "Request body validation failed.",
        error.flatten(),
      );
    }

    throw error;
  }
}

export function jsonResponse(body: unknown, status = 200) {
  return NextResponse.json(body, {
    status,
    headers: {
      "cache-control": "no-store",
    },
  });
}

export function handleRouteError(
  error: unknown,
  requestId: string,
  meta: RouteMeta,
) {
  const apiError =
    error instanceof ApiError
      ? error
      : new ApiError(500, "internal_error", "An unexpected error occurred.");

  logEvent({
    level: apiError.statusCode >= 500 ? "error" : "warn",
    feature: meta.feature,
    action: meta.action,
    requestId,
    result: apiError.code,
    context: {
      message: apiError.message,
      details: apiError.details,
      cause:
        error instanceof Error && !(error instanceof ApiError) ? error.message : undefined,
    },
  });

  return jsonResponse(
    {
      requestId,
      error: {
        code: apiError.code,
        message: apiError.message,
        details: apiError.details,
      },
    },
    apiError.statusCode,
  );
}
