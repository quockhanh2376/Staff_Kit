import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

export function proxy(request: NextRequest) {
  const headers = new Headers(request.headers);

  if (!headers.has("x-request-id")) {
    headers.set("x-request-id", crypto.randomUUID());
  }

  return NextResponse.next({
    request: {
      headers,
    },
  });
}

export const config = {
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)",
  ],
};
