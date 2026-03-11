import { hash, verify } from "@node-rs/argon2";

const ARGON_OPTIONS = {
  memoryCost: 19_456,
  timeCost: 2,
  parallelism: 1,
  outputLen: 32,
} as const;

export function hashPassword(value: string) {
  return hash(value, ARGON_OPTIONS);
}

export function verifyPassword(hashValue: string, value: string) {
  return verify(hashValue, value, ARGON_OPTIONS);
}
