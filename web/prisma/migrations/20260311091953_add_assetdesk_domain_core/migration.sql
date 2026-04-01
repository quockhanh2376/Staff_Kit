-- CreateEnum
CREATE TYPE "AssetStatus" AS ENUM ('IN_STOCK', 'ASSIGNED', 'RETIRED', 'DISPOSED');

-- CreateEnum
CREATE TYPE "AssetAssignmentStatus" AS ENUM ('ACTIVE', 'RETURNED');

-- CreateEnum
CREATE TYPE "WorkflowSessionStatus" AS ENUM ('ACTIVE', 'CLOSED', 'EXPIRED');

-- CreateEnum
CREATE TYPE "WorkflowRequestStatus" AS ENUM ('PENDING', 'APPROVED', 'REJECTED');

-- CreateEnum
CREATE TYPE "WorkflowRequestType" AS ENUM ('RECEIVE', 'RETURN');

-- CreateEnum
CREATE TYPE "ApprovalDecision" AS ENUM ('APPROVED', 'REJECTED');

-- CreateEnum
CREATE TYPE "AuditResult" AS ENUM ('SUCCESS', 'FAILURE', 'REJECTED', 'DENIED');

-- CreateTable
CREATE TABLE "assets" (
    "id" SERIAL NOT NULL,
    "assetKey" TEXT NOT NULL,
    "assetCode" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "assetType" TEXT NOT NULL,
    "status" "AssetStatus" NOT NULL DEFAULT 'IN_STOCK',
    "recordedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "owningUnit" TEXT,
    "managingUnit" TEXT,
    "serialNumber" TEXT,
    "brand" TEXT,
    "modelName" TEXT,
    "notes" TEXT,
    "retiredAt" TIMESTAMP(3),
    "disposedAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "assets_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "asset_assignments" (
    "id" SERIAL NOT NULL,
    "assignmentKey" TEXT NOT NULL,
    "assetId" INTEGER NOT NULL,
    "employeeId" INTEGER NOT NULL,
    "status" "AssetAssignmentStatus" NOT NULL DEFAULT 'ACTIVE',
    "receiveRequestItemId" INTEGER,
    "closedByReturnItemId" INTEGER,
    "assignedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "returnedAt" TIMESTAMP(3),
    "notes" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "asset_assignments_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "receive_sessions" (
    "id" SERIAL NOT NULL,
    "sessionKey" TEXT NOT NULL,
    "qrToken" TEXT NOT NULL,
    "createdByAccountId" INTEGER NOT NULL,
    "status" "WorkflowSessionStatus" NOT NULL DEFAULT 'ACTIVE',
    "expiresAt" TIMESTAMP(3),
    "closedAt" TIMESTAMP(3),
    "notes" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "receive_sessions_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "receive_requests" (
    "id" SERIAL NOT NULL,
    "requestKey" TEXT NOT NULL,
    "sessionId" INTEGER NOT NULL,
    "employeeId" INTEGER NOT NULL,
    "status" "WorkflowRequestStatus" NOT NULL DEFAULT 'PENDING',
    "employeeCodeSnapshot" TEXT NOT NULL,
    "employeeNameSnapshot" TEXT NOT NULL,
    "notes" TEXT,
    "submittedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "finalizedAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "receive_requests_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "receive_request_items" (
    "id" SERIAL NOT NULL,
    "requestId" INTEGER NOT NULL,
    "assetId" INTEGER NOT NULL,
    "assetCodeSnapshot" TEXT NOT NULL,
    "assetNameSnapshot" TEXT,
    "notes" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "receive_request_items_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "return_sessions" (
    "id" SERIAL NOT NULL,
    "sessionKey" TEXT NOT NULL,
    "qrToken" TEXT NOT NULL,
    "createdByAccountId" INTEGER NOT NULL,
    "status" "WorkflowSessionStatus" NOT NULL DEFAULT 'ACTIVE',
    "expiresAt" TIMESTAMP(3),
    "closedAt" TIMESTAMP(3),
    "notes" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "return_sessions_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "return_requests" (
    "id" SERIAL NOT NULL,
    "requestKey" TEXT NOT NULL,
    "sessionId" INTEGER NOT NULL,
    "employeeId" INTEGER NOT NULL,
    "status" "WorkflowRequestStatus" NOT NULL DEFAULT 'PENDING',
    "employeeCodeSnapshot" TEXT NOT NULL,
    "employeeNameSnapshot" TEXT NOT NULL,
    "notes" TEXT,
    "submittedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "finalizedAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "return_requests_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "return_request_items" (
    "id" SERIAL NOT NULL,
    "requestId" INTEGER NOT NULL,
    "assetId" INTEGER NOT NULL,
    "assetAssignmentId" INTEGER NOT NULL,
    "assetCodeSnapshot" TEXT NOT NULL,
    "assetNameSnapshot" TEXT,
    "notes" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "return_request_items_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "approval_reviews" (
    "id" SERIAL NOT NULL,
    "reviewKey" TEXT NOT NULL,
    "requestType" "WorkflowRequestType" NOT NULL,
    "reviewerAccountId" INTEGER NOT NULL,
    "decision" "ApprovalDecision" NOT NULL,
    "receiveRequestId" INTEGER,
    "returnRequestId" INTEGER,
    "notes" TEXT,
    "reviewedAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "approval_reviews_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "audit_logs" (
    "id" SERIAL NOT NULL,
    "eventKey" TEXT NOT NULL,
    "actorAccountId" INTEGER,
    "actorUsername" TEXT,
    "actionType" TEXT NOT NULL,
    "entityType" TEXT NOT NULL,
    "entityId" TEXT,
    "entityLabel" TEXT,
    "result" "AuditResult" NOT NULL DEFAULT 'SUCCESS',
    "reason" TEXT,
    "metadata" JSONB,
    "assetId" INTEGER,
    "employeeId" INTEGER,
    "receiveRequestId" INTEGER,
    "returnRequestId" INTEGER,
    "approvalReviewId" INTEGER,
    "occurredAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "audit_logs_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "assets_assetKey_key" ON "assets"("assetKey");

-- CreateIndex
CREATE UNIQUE INDEX "assets_assetCode_key" ON "assets"("assetCode");

-- CreateIndex
CREATE INDEX "assets_assetCode_idx" ON "assets"("assetCode");

-- CreateIndex
CREATE INDEX "assets_status_idx" ON "assets"("status");

-- CreateIndex
CREATE INDEX "assets_assetType_idx" ON "assets"("assetType");

-- CreateIndex
CREATE INDEX "assets_recordedAt_idx" ON "assets"("recordedAt");

-- CreateIndex
CREATE UNIQUE INDEX "asset_assignments_assignmentKey_key" ON "asset_assignments"("assignmentKey");

-- CreateIndex
CREATE UNIQUE INDEX "asset_assignments_receiveRequestItemId_key" ON "asset_assignments"("receiveRequestItemId");

-- CreateIndex
CREATE UNIQUE INDEX "asset_assignments_closedByReturnItemId_key" ON "asset_assignments"("closedByReturnItemId");

-- CreateIndex
CREATE INDEX "asset_assignments_assetId_idx" ON "asset_assignments"("assetId");

-- CreateIndex
CREATE INDEX "asset_assignments_employeeId_idx" ON "asset_assignments"("employeeId");

-- CreateIndex
CREATE INDEX "asset_assignments_status_idx" ON "asset_assignments"("status");

-- CreateIndex
CREATE INDEX "asset_assignments_assignedAt_idx" ON "asset_assignments"("assignedAt");

-- CreateIndex
CREATE UNIQUE INDEX "receive_sessions_sessionKey_key" ON "receive_sessions"("sessionKey");

-- CreateIndex
CREATE UNIQUE INDEX "receive_sessions_qrToken_key" ON "receive_sessions"("qrToken");

-- CreateIndex
CREATE INDEX "receive_sessions_qrToken_idx" ON "receive_sessions"("qrToken");

-- CreateIndex
CREATE INDEX "receive_sessions_status_idx" ON "receive_sessions"("status");

-- CreateIndex
CREATE INDEX "receive_sessions_createdByAccountId_idx" ON "receive_sessions"("createdByAccountId");

-- CreateIndex
CREATE INDEX "receive_sessions_expiresAt_idx" ON "receive_sessions"("expiresAt");

-- CreateIndex
CREATE UNIQUE INDEX "receive_requests_requestKey_key" ON "receive_requests"("requestKey");

-- CreateIndex
CREATE INDEX "receive_requests_sessionId_idx" ON "receive_requests"("sessionId");

-- CreateIndex
CREATE INDEX "receive_requests_employeeId_idx" ON "receive_requests"("employeeId");

-- CreateIndex
CREATE INDEX "receive_requests_status_idx" ON "receive_requests"("status");

-- CreateIndex
CREATE INDEX "receive_requests_submittedAt_idx" ON "receive_requests"("submittedAt");

-- CreateIndex
CREATE INDEX "receive_request_items_assetId_idx" ON "receive_request_items"("assetId");

-- CreateIndex
CREATE UNIQUE INDEX "receive_request_items_requestId_assetId_key" ON "receive_request_items"("requestId", "assetId");

-- CreateIndex
CREATE UNIQUE INDEX "return_sessions_sessionKey_key" ON "return_sessions"("sessionKey");

-- CreateIndex
CREATE UNIQUE INDEX "return_sessions_qrToken_key" ON "return_sessions"("qrToken");

-- CreateIndex
CREATE INDEX "return_sessions_qrToken_idx" ON "return_sessions"("qrToken");

-- CreateIndex
CREATE INDEX "return_sessions_status_idx" ON "return_sessions"("status");

-- CreateIndex
CREATE INDEX "return_sessions_createdByAccountId_idx" ON "return_sessions"("createdByAccountId");

-- CreateIndex
CREATE INDEX "return_sessions_expiresAt_idx" ON "return_sessions"("expiresAt");

-- CreateIndex
CREATE UNIQUE INDEX "return_requests_requestKey_key" ON "return_requests"("requestKey");

-- CreateIndex
CREATE INDEX "return_requests_sessionId_idx" ON "return_requests"("sessionId");

-- CreateIndex
CREATE INDEX "return_requests_employeeId_idx" ON "return_requests"("employeeId");

-- CreateIndex
CREATE INDEX "return_requests_status_idx" ON "return_requests"("status");

-- CreateIndex
CREATE INDEX "return_requests_submittedAt_idx" ON "return_requests"("submittedAt");

-- CreateIndex
CREATE INDEX "return_request_items_assetId_idx" ON "return_request_items"("assetId");

-- CreateIndex
CREATE INDEX "return_request_items_assetAssignmentId_idx" ON "return_request_items"("assetAssignmentId");

-- CreateIndex
CREATE UNIQUE INDEX "return_request_items_requestId_assetId_key" ON "return_request_items"("requestId", "assetId");

-- CreateIndex
CREATE UNIQUE INDEX "return_request_items_requestId_assetAssignmentId_key" ON "return_request_items"("requestId", "assetAssignmentId");

-- CreateIndex
CREATE UNIQUE INDEX "approval_reviews_reviewKey_key" ON "approval_reviews"("reviewKey");

-- CreateIndex
CREATE UNIQUE INDEX "approval_reviews_receiveRequestId_key" ON "approval_reviews"("receiveRequestId");

-- CreateIndex
CREATE UNIQUE INDEX "approval_reviews_returnRequestId_key" ON "approval_reviews"("returnRequestId");

-- CreateIndex
CREATE INDEX "approval_reviews_requestType_idx" ON "approval_reviews"("requestType");

-- CreateIndex
CREATE INDEX "approval_reviews_reviewerAccountId_idx" ON "approval_reviews"("reviewerAccountId");

-- CreateIndex
CREATE INDEX "approval_reviews_decision_idx" ON "approval_reviews"("decision");

-- CreateIndex
CREATE INDEX "approval_reviews_reviewedAt_idx" ON "approval_reviews"("reviewedAt");

-- CreateIndex
CREATE UNIQUE INDEX "audit_logs_eventKey_key" ON "audit_logs"("eventKey");

-- CreateIndex
CREATE INDEX "audit_logs_occurredAt_idx" ON "audit_logs"("occurredAt");

-- CreateIndex
CREATE INDEX "audit_logs_actorAccountId_idx" ON "audit_logs"("actorAccountId");

-- CreateIndex
CREATE INDEX "audit_logs_actionType_idx" ON "audit_logs"("actionType");

-- CreateIndex
CREATE INDEX "audit_logs_entityType_idx" ON "audit_logs"("entityType");

-- CreateIndex
CREATE INDEX "audit_logs_result_idx" ON "audit_logs"("result");

-- CreateIndex
CREATE INDEX "audit_logs_assetId_idx" ON "audit_logs"("assetId");

-- CreateIndex
CREATE INDEX "audit_logs_employeeId_idx" ON "audit_logs"("employeeId");

-- CreateIndex
CREATE INDEX "audit_logs_receiveRequestId_idx" ON "audit_logs"("receiveRequestId");

-- CreateIndex
CREATE INDEX "audit_logs_returnRequestId_idx" ON "audit_logs"("returnRequestId");

-- CreateIndex
CREATE INDEX "audit_logs_approvalReviewId_idx" ON "audit_logs"("approvalReviewId");

-- AddForeignKey
ALTER TABLE "asset_assignments" ADD CONSTRAINT "asset_assignments_assetId_fkey" FOREIGN KEY ("assetId") REFERENCES "assets"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "asset_assignments" ADD CONSTRAINT "asset_assignments_employeeId_fkey" FOREIGN KEY ("employeeId") REFERENCES "employees"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "asset_assignments" ADD CONSTRAINT "asset_assignments_receiveRequestItemId_fkey" FOREIGN KEY ("receiveRequestItemId") REFERENCES "receive_request_items"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "asset_assignments" ADD CONSTRAINT "asset_assignments_closedByReturnItemId_fkey" FOREIGN KEY ("closedByReturnItemId") REFERENCES "return_request_items"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "receive_sessions" ADD CONSTRAINT "receive_sessions_createdByAccountId_fkey" FOREIGN KEY ("createdByAccountId") REFERENCES "local_accounts"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "receive_requests" ADD CONSTRAINT "receive_requests_sessionId_fkey" FOREIGN KEY ("sessionId") REFERENCES "receive_sessions"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "receive_requests" ADD CONSTRAINT "receive_requests_employeeId_fkey" FOREIGN KEY ("employeeId") REFERENCES "employees"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "receive_request_items" ADD CONSTRAINT "receive_request_items_requestId_fkey" FOREIGN KEY ("requestId") REFERENCES "receive_requests"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "receive_request_items" ADD CONSTRAINT "receive_request_items_assetId_fkey" FOREIGN KEY ("assetId") REFERENCES "assets"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "return_sessions" ADD CONSTRAINT "return_sessions_createdByAccountId_fkey" FOREIGN KEY ("createdByAccountId") REFERENCES "local_accounts"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "return_requests" ADD CONSTRAINT "return_requests_sessionId_fkey" FOREIGN KEY ("sessionId") REFERENCES "return_sessions"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "return_requests" ADD CONSTRAINT "return_requests_employeeId_fkey" FOREIGN KEY ("employeeId") REFERENCES "employees"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "return_request_items" ADD CONSTRAINT "return_request_items_requestId_fkey" FOREIGN KEY ("requestId") REFERENCES "return_requests"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "return_request_items" ADD CONSTRAINT "return_request_items_assetId_fkey" FOREIGN KEY ("assetId") REFERENCES "assets"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "return_request_items" ADD CONSTRAINT "return_request_items_assetAssignmentId_fkey" FOREIGN KEY ("assetAssignmentId") REFERENCES "asset_assignments"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "approval_reviews" ADD CONSTRAINT "approval_reviews_reviewerAccountId_fkey" FOREIGN KEY ("reviewerAccountId") REFERENCES "local_accounts"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "approval_reviews" ADD CONSTRAINT "approval_reviews_receiveRequestId_fkey" FOREIGN KEY ("receiveRequestId") REFERENCES "receive_requests"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "approval_reviews" ADD CONSTRAINT "approval_reviews_returnRequestId_fkey" FOREIGN KEY ("returnRequestId") REFERENCES "return_requests"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_logs" ADD CONSTRAINT "audit_logs_actorAccountId_fkey" FOREIGN KEY ("actorAccountId") REFERENCES "local_accounts"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_logs" ADD CONSTRAINT "audit_logs_assetId_fkey" FOREIGN KEY ("assetId") REFERENCES "assets"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_logs" ADD CONSTRAINT "audit_logs_employeeId_fkey" FOREIGN KEY ("employeeId") REFERENCES "employees"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_logs" ADD CONSTRAINT "audit_logs_receiveRequestId_fkey" FOREIGN KEY ("receiveRequestId") REFERENCES "receive_requests"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_logs" ADD CONSTRAINT "audit_logs_returnRequestId_fkey" FOREIGN KEY ("returnRequestId") REFERENCES "return_requests"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "audit_logs" ADD CONSTRAINT "audit_logs_approvalReviewId_fkey" FOREIGN KEY ("approvalReviewId") REFERENCES "approval_reviews"("id") ON DELETE SET NULL ON UPDATE CASCADE;
