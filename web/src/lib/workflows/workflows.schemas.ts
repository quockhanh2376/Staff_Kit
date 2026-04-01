import { z } from "zod";

const trimmedRequiredText = z.string().trim().min(1).max(255);

const trimmedOptionalText = z
  .string()
  .trim()
  .max(1000)
  .nullish()
  .transform((value) => {
    const trimmed = value?.trim();
    return trimmed ? trimmed : undefined;
  });

export const createSessionSchema = z.object({
  expiresAt: z.coerce.date().optional(),
  notes: trimmedOptionalText,
});

export const submitReceiveRequestSchema = z.object({
  qrToken: trimmedRequiredText,
  employeeId: trimmedRequiredText,
  assetCodes: z.array(trimmedRequiredText).min(1).max(50),
  notes: trimmedOptionalText,
});

export const submitReturnRequestSchema = z.object({
  qrToken: trimmedRequiredText,
  employeeId: trimmedRequiredText,
  assetCodes: z.array(trimmedRequiredText).min(1).max(50),
  notes: trimmedOptionalText,
});

export const reviewPendingRequestSchema = z.object({
  requestType: z.enum(["RECEIVE", "RETURN"]),
  requestKey: trimmedRequiredText,
  decision: z.enum(["APPROVED", "REJECTED"]),
  notes: trimmedOptionalText,
  reviewedEmployeeId: trimmedOptionalText,
  reviewedAssetCodes: z.array(trimmedRequiredText).min(1).max(50).optional(),
});

export const pendingRequestFiltersSchema = z.object({
  requestType: z.enum(["RECEIVE", "RETURN"]).optional(),
  take: z.coerce.number().int().min(1).max(100).optional(),
});

export type CreateSessionInput = z.infer<typeof createSessionSchema>;
export type SubmitReceiveRequestInput = z.infer<typeof submitReceiveRequestSchema>;
export type SubmitReturnRequestInput = z.infer<typeof submitReturnRequestSchema>;
export type ReviewPendingRequestInput = z.infer<typeof reviewPendingRequestSchema>;
export type PendingRequestFiltersInput = z.infer<typeof pendingRequestFiltersSchema>;
