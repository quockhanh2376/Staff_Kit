import { z } from "zod";

const serverEnvSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]).default("development"),
  APP_URL: z.string().url().optional(),
  AUTH_URL: z.string().url().optional(),
  DATABASE_URL: z.string().min(1).optional(),
  REDIS_URL: z.string().min(1).optional(),
  AUTH_SECRET: z.string().min(16).optional(),
  SEED_SUPER_ADMIN_USERNAME: z.string().min(1).optional(),
  SEED_SUPER_ADMIN_DISPLAY_NAME: z.string().min(1).optional(),
  SEED_SUPER_ADMIN_PASSWORD: z.string().min(8).optional(),
});

export type ServerEnv = z.infer<typeof serverEnvSchema>;

export function readServerEnv() {
  return serverEnvSchema.safeParse({
    NODE_ENV: process.env.NODE_ENV,
    APP_URL: process.env.APP_URL,
    AUTH_URL: process.env.AUTH_URL,
    DATABASE_URL: process.env.DATABASE_URL,
    REDIS_URL: process.env.REDIS_URL,
    AUTH_SECRET: process.env.AUTH_SECRET,
    SEED_SUPER_ADMIN_USERNAME: process.env.SEED_SUPER_ADMIN_USERNAME,
    SEED_SUPER_ADMIN_DISPLAY_NAME: process.env.SEED_SUPER_ADMIN_DISPLAY_NAME,
    SEED_SUPER_ADMIN_PASSWORD: process.env.SEED_SUPER_ADMIN_PASSWORD,
  });
}

export function requiredServerEnvKeys() {
  return ["APP_URL", "AUTH_URL", "DATABASE_URL", "REDIS_URL", "AUTH_SECRET"] as const;
}
