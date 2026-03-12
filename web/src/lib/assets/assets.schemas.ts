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

const assetStatusSchema = z.enum(["IN_STOCK", "ASSIGNED", "RETIRED", "DISPOSED"]);
const assetDateSchema = z.coerce.date().optional();

function validateAssetLifecycleDates(
  value: {
    status?: "IN_STOCK" | "ASSIGNED" | "RETIRED" | "DISPOSED";
    retiredAt?: Date;
    disposedAt?: Date;
  },
  ctx: z.RefinementCtx,
) {
  if (value.retiredAt && value.status !== "RETIRED") {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ["retiredAt"],
      message: "retiredAt is only allowed when status is RETIRED.",
    });
  }

  if (value.disposedAt && value.status !== "DISPOSED") {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ["disposedAt"],
      message: "disposedAt is only allowed when status is DISPOSED.",
    });
  }
}

export const assetCreateSchema = z
  .object({
    assetCode: trimmedOptionalText,
    name: trimmedRequiredText,
    assetType: trimmedRequiredText,
    status: assetStatusSchema.optional(),
    recordedAt: assetDateSchema,
    owningUnit: trimmedOptionalText,
    managingUnit: trimmedOptionalText,
    serialNumber: trimmedOptionalText,
    brand: trimmedOptionalText,
    modelName: trimmedOptionalText,
    notes: trimmedOptionalText,
    retiredAt: assetDateSchema,
    disposedAt: assetDateSchema,
  })
  .superRefine(validateAssetLifecycleDates);

export const assetUpdateSchema = z
  .object({
    name: trimmedRequiredText.optional(),
    assetType: trimmedRequiredText.optional(),
    status: assetStatusSchema.optional(),
    recordedAt: assetDateSchema,
    owningUnit: trimmedOptionalText,
    managingUnit: trimmedOptionalText,
    serialNumber: trimmedOptionalText,
    brand: trimmedOptionalText,
    modelName: trimmedOptionalText,
    notes: trimmedOptionalText,
    retiredAt: assetDateSchema,
    disposedAt: assetDateSchema,
  })
  .refine((value) => Object.keys(value).length > 0, {
    message: "At least one field must be provided.",
  })
  .superRefine(validateAssetLifecycleDates);

export const assetPreloadRowSchema = assetCreateSchema.safeExtend({
  assetCode: trimmedRequiredText,
});

export const assetPreloadSchema = z.object({
  assets: z.array(assetPreloadRowSchema).min(1).max(500),
});

export const assetListFiltersSchema = z.object({
  q: trimmedOptionalText,
  status: assetStatusSchema.optional(),
  assetType: trimmedOptionalText,
  take: z.coerce.number().int().min(1).max(200).optional(),
});

export type AssetCreateInput = z.infer<typeof assetCreateSchema>;
export type AssetUpdateInput = z.infer<typeof assetUpdateSchema>;
export type AssetPreloadRowInput = z.infer<typeof assetPreloadRowSchema>;
export type AssetPreloadInput = z.infer<typeof assetPreloadSchema>;
export type AssetListFiltersInput = z.infer<typeof assetListFiltersSchema>;
