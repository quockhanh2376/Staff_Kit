import type { Route } from "next";
import { redirect } from "next/navigation";

import { auth } from "@/auth";

export default async function Home() {
  const session = await auth();
  const destination: Route = session?.user ? "/dashboard" : "/login";

  redirect(destination);
}
