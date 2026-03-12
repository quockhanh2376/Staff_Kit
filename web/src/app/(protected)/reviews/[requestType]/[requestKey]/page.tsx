import { notFound } from "next/navigation";

import { ApiError } from "@/lib/api/errors";
import {
  getReviewRequestDetail,
  type ReviewRequestDetail,
} from "@/lib/workflows/workflows.service";
import { ReviewDetailView } from "@/components/reviews/ReviewDetailView";

import { approveReviewAction, rejectReviewAction } from "../../actions";

type ReviewPageProps = {
  params: Promise<{
    requestType: string;
    requestKey: string;
  }>;
  searchParams?: Promise<{
    error?: string;
  }>;
};

function parseRequestTypeSegment(requestType: string) {
  if (requestType === "receive") {
    return "RECEIVE" as const;
  }

  if (requestType === "return") {
    return "RETURN" as const;
  }

  notFound();
}

async function loadDetail(
  requestType: "RECEIVE" | "RETURN",
  requestKey: string,
): Promise<ReviewRequestDetail> {
  try {
    return await getReviewRequestDetail(requestType, requestKey);
  } catch (error) {
    if (error instanceof ApiError && error.statusCode === 404) {
      notFound();
    }

    throw error;
  }
}

export default async function ReviewRequestDetailPage({
  params,
  searchParams,
}: ReviewPageProps) {
  const { requestKey, requestType } = await params;
  const resolvedSearchParams = searchParams ? await searchParams : undefined;
  const normalizedRequestType = parseRequestTypeSegment(requestType);
  const detail = await loadDetail(normalizedRequestType, requestKey);

  return (
    <ReviewDetailView
      detail={detail}
      requestTypeSegment={requestType as "receive" | "return"}
      approveAction={approveReviewAction}
      rejectAction={rejectReviewAction}
      errorCode={resolvedSearchParams?.error}
    />
  );
}
