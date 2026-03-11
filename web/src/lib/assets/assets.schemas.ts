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

export const assetCreateSchema = z.object({
  assetCode: trimmedOptionalText,
  name: trimmedRequiredText,
  assetType: trimmedRequiredText,
  status: assetStatusSchema.optional(),
  recordedAt: z.coerce.date().optional(),
  owningUnit: trimmedOptionalText,
  managingUnit: trimmedOptionalText,
  serialNumber: trimmedOptionalText,
  brand: trimmedOptionalText,
  modelName: trimmedOptionalText,
  notes: trimmedOptionalText,
});

export const assetUpdateSchema = z
  .object({
    name: trimmedRequiredText.optional(),
    assetType: trimmedRequiredText.optional(),
    status: assetStatusSchema.optional(),
    recordedAt: z.coerce.date().optional(),
    owningUnit: trimmedOptionalText,
    managingUnit: trimmedOptionalText,
    serialNumber: trimmedOptionalText,
    brand: trimmedOptionalText,
    modelName: trimmedOptionalText,
    notes: trimmedOptionalText,
  })
  .refine((value) => Object.keys(value).length > 0, {
    message: "At least one field must be provided.",
  });

export const assetPreloadSchema = z.object({
  assets: z.array(assetCreateSchema.extend({ assetCode: trimmedRequiredText })).min(1).max(500),
});

export const assetListFiltersSchema = z.object({
  q: trimmedOptionalText,
  status: assetStatusSchema.optional(),
  assetType: trimmedOptionalText,
  take: z.coerce.number().int().min(1).max(200).optional(),
});

export type AssetCreateInput = z.infer<typeof assetCreateSchema>;
export type AssetUpdateInput = z.infer<typeof assetUpdateSchema>;
export type AssetPreloadInput = z.infer<typeof assetPreloadSchema>;
export type AssetListFiltersInput = z.infer<typeof assetListFiltersSchema>;
