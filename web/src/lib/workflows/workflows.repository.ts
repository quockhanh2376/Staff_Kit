import { prisma } from "@/lib/prisma";

type WorkflowRepositoryClient = Pick<
  typeof prisma,
  | "employee"
  | "asset"
  | "assetAssignment"
  | "receiveSession"
  | "receiveRequest"
  | "returnSession"
  | "returnRequest"
  | "approvalReview"
>;

export async function findEmployeeByEmployeeId(
  db: WorkflowRepositoryClient,
  employeeId: string,
) {
  return db.employee.findUnique({
    where: {
      employeeId,
    },
    select: {
      id: true,
      employeeId: true,
      fullName: true,
      teamId: true,
    },
  });
}

export async function createReceiveSessionRecord(
  db: WorkflowRepositoryClient,
  data: {
    createdByAccountId: number;
    expiresAt?: Date;
    notes?: string;
  },
) {
  return db.receiveSession.create({
    data,
  });
}

export async function createReturnSessionRecord(
  db: WorkflowRepositoryClient,
  data: {
    createdByAccountId: number;
    expiresAt?: Date;
    notes?: string;
  },
) {
  return db.returnSession.create({
    data,
  });
}

export async function findActiveReceiveSession(
  db: WorkflowRepositoryClient,
  now: Date,
) {
  return db.receiveSession.findFirst({
    where: {
      status: "ACTIVE",
      OR: [
        {
          expiresAt: null,
        },
        {
          expiresAt: {
            gt: now,
          },
        },
      ],
    },
  });
}

export async function findActiveReceiveSessionByQrToken(
  db: WorkflowRepositoryClient,
  qrToken: string,
  now: Date,
) {
  return db.receiveSession.findFirst({
    where: {
      qrToken,
      status: "ACTIVE",
      OR: [
        {
          expiresAt: null,
        },
        {
          expiresAt: {
            gt: now,
          },
        },
      ],
    },
  });
}

export async function findActiveReturnSessionByQrToken(
  db: WorkflowRepositoryClient,
  qrToken: string,
  now: Date,
) {
  return db.returnSession.findFirst({
    where: {
      qrToken,
      status: "ACTIVE",
      OR: [
        {
          expiresAt: null,
        },
        {
          expiresAt: {
            gt: now,
          },
        },
      ],
    },
  });
}

export async function findAssetsByAssetCodes(
  db: WorkflowRepositoryClient,
  assetCodes: string[],
) {
  return db.asset.findMany({
    where: {
      assetCode: {
        in: assetCodes,
      },
    },
    orderBy: {
      assetCode: "asc",
    },
  });
}

export async function findActiveAssignmentsByAssetCodes(
  db: WorkflowRepositoryClient,
  assetCodes: string[],
) {
  return db.assetAssignment.findMany({
    where: {
      status: "ACTIVE",
      asset: {
        assetCode: {
          in: assetCodes,
        },
      },
    },
    include: {
      asset: true,
      employee: {
        select: {
          id: true,
          employeeId: true,
          fullName: true,
        },
      },
    },
  });
}

export async function createReceiveRequestRecord(
  db: WorkflowRepositoryClient,
  data: {
    sessionId: number;
    employeeId: number;
    employeeCodeSnapshot: string;
    employeeNameSnapshot: string;
    notes?: string;
    items: Array<{
      assetId: number;
      assetCodeSnapshot: string;
      assetNameSnapshot?: string;
    }>;
  },
) {
  return db.receiveRequest.create({
    data: {
      sessionId: data.sessionId,
      employeeId: data.employeeId,
      employeeCodeSnapshot: data.employeeCodeSnapshot,
      employeeNameSnapshot: data.employeeNameSnapshot,
      notes: data.notes,
      items: {
        create: data.items,
      },
    },
    include: {
      items: {
        orderBy: {
          assetCodeSnapshot: "asc",
        },
      },
    },
  });
}

export async function createReturnRequestRecord(
  db: WorkflowRepositoryClient,
  data: {
    sessionId: number;
    employeeId: number;
    employeeCodeSnapshot: string;
    employeeNameSnapshot: string;
    notes?: string;
    items: Array<{
      assetId: number;
      assetAssignmentId: number;
      assetCodeSnapshot: string;
      assetNameSnapshot?: string;
    }>;
  },
) {
  return db.returnRequest.create({
    data: {
      sessionId: data.sessionId,
      employeeId: data.employeeId,
      employeeCodeSnapshot: data.employeeCodeSnapshot,
      employeeNameSnapshot: data.employeeNameSnapshot,
      notes: data.notes,
      items: {
        create: data.items,
      },
    },
    include: {
      items: {
        orderBy: {
          assetCodeSnapshot: "asc",
        },
      },
    },
  });
}

export async function findReceiveRequestForReview(
  db: WorkflowRepositoryClient,
  requestKey: string,
) {
  return db.receiveRequest.findUnique({
    where: {
      requestKey,
    },
    include: {
      employee: {
        select: {
          id: true,
          employeeId: true,
          fullName: true,
        },
      },
      items: {
        include: {
          asset: true,
        },
        orderBy: {
          assetCodeSnapshot: "asc",
        },
      },
      approvalReview: true,
    },
  });
}

export async function findReturnRequestForReview(
  db: WorkflowRepositoryClient,
  requestKey: string,
) {
  return db.returnRequest.findUnique({
    where: {
      requestKey,
    },
    include: {
      employee: {
        select: {
          id: true,
          employeeId: true,
          fullName: true,
        },
      },
      items: {
        include: {
          asset: true,
          assetAssignment: {
            include: {
              employee: {
                select: {
                  id: true,
                  employeeId: true,
                  fullName: true,
                },
              },
            },
          },
        },
        orderBy: {
          assetCodeSnapshot: "asc",
        },
      },
      approvalReview: true,
    },
  });
}

export async function listPendingReceiveRequests(
  db: WorkflowRepositoryClient,
  take: number,
) {
  return db.receiveRequest.findMany({
    where: {
      status: "PENDING",
    },
    orderBy: {
      submittedAt: "desc",
    },
    take,
    include: {
      employee: {
        select: {
          id: true,
          employeeId: true,
          fullName: true,
        },
      },
      items: {
        select: {
          assetCodeSnapshot: true,
        },
      },
    },
  });
}

export async function listPendingReturnRequests(
  db: WorkflowRepositoryClient,
  take: number,
) {
  return db.returnRequest.findMany({
    where: {
      status: "PENDING",
    },
    orderBy: {
      submittedAt: "desc",
    },
    take,
    include: {
      employee: {
        select: {
          id: true,
          employeeId: true,
          fullName: true,
        },
      },
      items: {
        select: {
          assetCodeSnapshot: true,
        },
      },
    },
  });
}
