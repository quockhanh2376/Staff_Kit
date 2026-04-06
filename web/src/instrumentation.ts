import { logEvent } from "@/lib/logger";

export function register() {
  logEvent({
    level: "info",
    feature: "bootstrap",
    action: "instrumentation.register",
    result: "ready",
    context: {
      runtime: process.env.NEXT_RUNTIME ?? "nodejs",
    },
  });
}
