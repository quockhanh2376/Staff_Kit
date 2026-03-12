import { prisma } from "@/lib/prisma";
import { getAuditLogs } from "@/lib/audit/audit.service";
import { getPendingRequests } from "@/lib/workflows/workflows.service";

export async function getDashboardSnapshot() {
  const [
    employeeCount,
    assetCount,
    assignedAssetCount,
    pendingReceiveCount,
    pendingReturnCount,
    activeReceiveSessions,
    activeReturnSessions,
  ] = await Promise.all([
    prisma.employee.count(),
    prisma.asset.count(),
    prisma.asset.count({
      where: {
        status: "ASSIGNED",
      },
    }),
    prisma.receiveRequest.count({
      where: {
        status: "PENDING",
      },
    }),
    prisma.returnRequest.count({
      where: {
        status: "PENDING",
      },
    }),
    prisma.receiveSession.findMany({
      where: {
        status: "ACTIVE",
      },
      orderBy: {
        createdAt: "desc",
      },
      take: 3,
    }),
    prisma.returnSession.findMany({
      where: {
        status: "ACTIVE",
      },
      orderBy: {
        createdAt: "desc",
      },
      take: 3,
    }),
  ]);

  return {
    employeeCount,
    assetCount,
    assignedAssetCount,
    pendingReceiveCount,
    pendingReturnCount,
    activeReceiveSessions,
    activeReturnSessions,
  };
}

export async function getEmployeeDirectoryPreview() {
  return prisma.employee.findMany({
    orderBy: {
      employeeId: "asc",
    },
    take: 20,
    include: {
      team: {
        select: {
          name: true,
        },
      },
      assetAssignments: {
        where: {
          status: "ACTIVE",
        },
        select: {
          id: true,
        },
      },
    },
  });
}

export async function getAssetCatalogPreview() {
  return prisma.asset.findMany({
    orderBy: {
      assetCode: "asc",
    },
    take: 24,
    include: {
      assignments: {
        where: {
          status: "ACTIVE",
        },
        take: 1,
        include: {
          employee: {
            select: {
              employeeId: true,
              fullName: true,
            },
          },
        },
      },
    },
  });
}

export async function getPendingReviewPreview() {
  return getPendingRequests({
    take: 20,
  });
}

export async function getReceiveSessionPreview() {
  return prisma.receiveSession.findMany({
    orderBy: [
      {
        status: "asc",
      },
      {
        createdAt: "desc",
      },
    ],
    take: 10,
  });
}

export async function getReturnSessionPreview() {
  return prisma.returnSession.findMany({
    orderBy: [
      {
        status: "asc",
      },
      {
        createdAt: "desc",
      },
    ],
    take: 10,
  });
}

export async function getAuditTrailPreview() {
  return getAuditLogs({
    take: 20,
  });
}
