import { NextResponse } from "next/server";

import { readServerEnv, requiredServerEnvKeys } from "@/lib/env";
import { logEvent } from "@/lib/logger";

export const dynamic = "force-dynamic";

export async function GET(request: Request) {
  const requestId = request.headers.get("x-request-id") ?? crypto.randomUUID();
  const env = readServerEnv();
  const missing = requiredServerEnvKeys().filter((key) => !process.env[key]);
  const status = missing.length === 0 ? "ok" : "degraded";

  logEvent({
    level: "info",
    feature: "ops",
    action: "health.check",
    requestId,
    result: status,
    context: {
      missingKeys: missing,
    },
  });

  return NextResponse.json(
    {
      service: "assetdesk-pro-web",
      framework: "nextjs-16",
      status,
      requestId,
      timestamp: new Date().toISOString(),
      env: {
        valid: env.success,
        missing,
      },
    },
    {
      status: status === "ok" ? 200 : 503,
      headers: {
        "cache-control": "no-store",
      },
    },
  );
}
