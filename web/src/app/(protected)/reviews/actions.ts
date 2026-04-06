"use server";

import type { Route } from "next";
import { redirect } from "next/navigation";
import { revalidatePath } from "next/cache";
import { z } from "zod";

import { requireApiActor } from "@/lib/api/auth";
import { ApiError } from "@/lib/api/errors";
import { reviewPendingRequest } from "@/lib/workflows/workflows.service";

const reviewActionSchema = z.object({
  requestType: z.enum(["RECEIVE", "RETURN"]),
  requestTypeSegment: z.enum(["receive", "return"]),
  requestKey: z.string().trim().min(1),
  reviewedEmployeeId: z.string().trim().min(1),
  reviewedAssetCodesInput: z.string().trim().min(1),
  notes: z
    .string()
    .trim()
    .max(1000)
    .optional()
    .transform((value) => (value ? value : undefined)),
});

function readActionInput(formData: FormData) {
  const parsed = reviewActionSchema.parse({
    requestType: formData.get("requestType"),
    requestTypeSegment: formData.get("requestTypeSegment"),
    requestKey: formData.get("requestKey"),
    reviewedEmployeeId: formData.get("reviewedEmployeeId"),
    reviewedAssetCodesInput: formData.get("reviewedAssetCodesInput"),
    notes: formData.get("notes"),
  });

  return {
    ...parsed,
    reviewedAssetCodes: parsed.reviewedAssetCodesInput
      .split(/\r?\n|,/)
      .map((value) => value.trim())
      .filter(Boolean),
  };
}

function buildDetailPath(requestTypeSegment: "receive" | "return", requestKey: string) {
  return `/reviews/${requestTypeSegment}/${requestKey}`;
}

function revalidateReviewPaths(detailPath: string) {
  revalidatePath("/dashboard");
  revalidatePath("/assets");
  revalidatePath("/employees");
  revalidatePath("/audit");
  revalidatePath("/reviews");
  revalidatePath(detailPath);
}

function getErrorCode(error: unknown) {
  if (error instanceof ApiError) {
    return error.code;
  }

  if (error instanceof z.ZodError) {
    return "validation_error";
  }

  return "internal_error";
}

export async function approveReviewAction(formData: FormData) {
  const actor = await requireApiActor("ADMIN");
  const input = readActionInput(formData);
  const detailPath = buildDetailPath(input.requestTypeSegment, input.requestKey);

  try {
    await reviewPendingRequest(actor, {
      requestType: input.requestType,
      requestKey: input.requestKey,
      decision: "APPROVED",
      notes: input.notes,
      reviewedEmployeeId: input.reviewedEmployeeId,
      reviewedAssetCodes: input.reviewedAssetCodes,
    });
  } catch (error) {
    redirect(`${detailPath}?error=${encodeURIComponent(getErrorCode(error))}` as Route);
  }

  revalidateReviewPaths(detailPath);
  redirect(`/reviews?updated=${encodeURIComponent(input.requestKey)}` as Route);
}

export async function rejectReviewAction(formData: FormData) {
  const actor = await requireApiActor("ADMIN");
  const input = readActionInput(formData);
  const detailPath = buildDetailPath(input.requestTypeSegment, input.requestKey);

  try {
    await reviewPendingRequest(actor, {
      requestType: input.requestType,
      requestKey: input.requestKey,
      decision: "REJECTED",
      notes: input.notes,
      reviewedEmployeeId: input.reviewedEmployeeId,
      reviewedAssetCodes: input.reviewedAssetCodes,
    });
  } catch (error) {
    redirect(`${detailPath}?error=${encodeURIComponent(getErrorCode(error))}` as Route);
  }

  revalidateReviewPaths(detailPath);
  redirect(`/reviews?updated=${encodeURIComponent(input.requestKey)}` as Route);
}
