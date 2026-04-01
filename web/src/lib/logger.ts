type LogLevel = "info" | "warn" | "error";

type LogPayload = {
  level: LogLevel;
  feature: string;
  action: string;
  result: string;
  requestId?: string;
  context?: Record<string, unknown>;
};

export function logEvent(payload: LogPayload) {
  const record = {
    timestamp: new Date().toISOString(),
    ...payload,
  };

  const line = JSON.stringify(record);

  if (payload.level === "error") {
    console.error(line);
    return;
  }

  if (payload.level === "warn") {
    console.warn(line);
    return;
  }

  console.info(line);
}
