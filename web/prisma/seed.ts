import "dotenv/config";

import { prisma } from "../src/lib/prisma";
import { hashPassword } from "../src/lib/password";

async function main() {
  const isNonProduction = process.env.NODE_ENV !== "production";
  const username = process.env.SEED_SUPER_ADMIN_USERNAME?.trim() || "adman";
  const displayName = process.env.SEED_SUPER_ADMIN_DISPLAY_NAME?.trim() || username;
  const password =
    process.env.SEED_SUPER_ADMIN_PASSWORD?.trim() ||
    (isNonProduction && username === "adman" ? "20252026" : "ChangeMe-2026!");
  const recoveryCode =
    process.env.SEED_SUPER_ADMIN_RECOVERY_CODE?.trim() || "ADP-DEV-RECOVERY";
  const forcePasswordReset =
    process.env.SEED_SUPER_ADMIN_FORCE_PASSWORD_RESET === "true";

  const passwordHash = await hashPassword(password);
  const recoveryCodeHash = recoveryCode ? await hashPassword(recoveryCode) : null;

  await prisma.localAccount.upsert({
    where: {
      username,
    },
    create: {
      displayName,
      username,
      passwordHash,
      recoveryCodeHash,
      role: "SUPER_ADMIN",
      forcePasswordReset,
      isActive: true,
    },
    update: {
      displayName,
      passwordHash,
      recoveryCodeHash,
      role: "SUPER_ADMIN",
      forcePasswordReset,
      isActive: true,
    },
  });

  console.info(`Seeded AssetDesk-Pro bootstrap admin account: ${username}`);
}

main()
  .catch((error) => {
    console.error("Prisma seed failed", error);
    process.exitCode = 1;
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
