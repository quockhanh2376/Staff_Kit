import "dotenv/config";

import { prisma } from "../src/lib/prisma";
import { hashPassword } from "../src/lib/password";

const TEAM_NAMES = ["IT Operations", "EML", "Finance", "HR", "PMD"] as const;

const EMPLOYEE_FIXTURES = [
  { employeeId: "ADP001", fullName: "Nguyen Minh Anh", teamName: "IT Operations", jobTitle: "IT Support Engineer", email: "minh.anh@assetdesk-pro.local" },
  { employeeId: "ADP002", fullName: "Tran Hoang Bao", teamName: "EML", jobTitle: "Security Analyst", email: "hoang.bao@assetdesk-pro.local" },
  { employeeId: "ADP003", fullName: "Le Thu Giang", teamName: "Finance", jobTitle: "Finance Executive", email: "thu.giang@assetdesk-pro.local" },
  { employeeId: "ADP004", fullName: "Pham Quoc Huy", teamName: "PMD", jobTitle: "Project Coordinator", email: "quoc.huy@assetdesk-pro.local" },
  { employeeId: "ADP005", fullName: "Vo Thanh Lam", teamName: "HR", jobTitle: "HR Specialist", email: "thanh.lam@assetdesk-pro.local" },
  { employeeId: "ADP006", fullName: "Bui Gia Linh", teamName: "IT Operations", jobTitle: "IT Intern", email: "gia.linh@assetdesk-pro.local" },
  { employeeId: "ADP007", fullName: "Do Khanh Nhi", teamName: "EML", jobTitle: "Endpoint Engineer", email: "khanh.nhi@assetdesk-pro.local" },
  { employeeId: "ADP008", fullName: "Nguyen Tuan Kiet", teamName: "Finance", jobTitle: "Accountant", email: "tuan.kiet@assetdesk-pro.local" },
  { employeeId: "ADP009", fullName: "Truong Phuong Vy", teamName: "HR", jobTitle: "HR Coordinator", email: "phuong.vy@assetdesk-pro.local" },
  { employeeId: "ADP010", fullName: "Le Gia Phuc", teamName: "PMD", jobTitle: "Project Analyst", email: "gia.phuc@assetdesk-pro.local" },
  { employeeId: "ADP011", fullName: "Phan Quynh Mai", teamName: "IT Operations", jobTitle: "Infrastructure Engineer", email: "quynh.mai@assetdesk-pro.local" },
  { employeeId: "ADP012", fullName: "Dang Anh Tuan", teamName: "EML", jobTitle: "Compliance Specialist", email: "anh.tuan@assetdesk-pro.local" },
] as const;

const ASSET_FIXTURES = [
  { assetCode: "ADP-NB-1001", name: "Dell Latitude 7450", assetType: "Laptop", status: "IN_STOCK", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "DL7450-1001", brand: "Dell", modelName: "Latitude 7450" },
  { assetCode: "ADP-MON-1001", name: "Dell P2425H", assetType: "Monitor", status: "IN_STOCK", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "MON-1001", brand: "Dell", modelName: "P2425H" },
  { assetCode: "ADP-NB-1002", name: "HP EliteBook 840", assetType: "Laptop", status: "IN_STOCK", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "HP840-1002", brand: "HP", modelName: "EliteBook 840" },
  { assetCode: "ADP-KM-1001", name: "Logitech MK850", assetType: "Keyboard/Mouse", status: "IN_STOCK", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "LGMK850-1001", brand: "Logitech", modelName: "MK850" },
  { assetCode: "ADP-HS-1001", name: "Jabra Evolve2 65", assetType: "Headset", status: "IN_STOCK", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "JABRA-1001", brand: "Jabra", modelName: "Evolve2 65" },
  { assetCode: "ADP-DOCK-1001", name: "Dell WD19 Dock", assetType: "Docking Station", status: "IN_STOCK", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "WD19-1001", brand: "Dell", modelName: "WD19" },
  { assetCode: "ADP-NB-2001", name: "Dell Latitude 5440", assetType: "Laptop", status: "ASSIGNED", owningUnit: "IT Operations", managingUnit: "EML", serialNumber: "DL5440-2001", brand: "Dell", modelName: "Latitude 5440" },
  { assetCode: "ADP-PH-2001", name: "iPhone 14", assetType: "Phone", status: "ASSIGNED", owningUnit: "IT Operations", managingUnit: "EML", serialNumber: "IPH14-2001", brand: "Apple", modelName: "iPhone 14" },
  { assetCode: "ADP-NB-2002", name: "Lenovo ThinkPad T14", assetType: "Laptop", status: "ASSIGNED", owningUnit: "IT Operations", managingUnit: "Finance", serialNumber: "LVT14-2002", brand: "Lenovo", modelName: "ThinkPad T14" },
  { assetCode: "ADP-MON-2001", name: "LG 27UP850", assetType: "Monitor", status: "ASSIGNED", owningUnit: "IT Operations", managingUnit: "PMD", serialNumber: "LG27UP850-2001", brand: "LG", modelName: "27UP850" },
  { assetCode: "ADP-KB-2001", name: "Keychron K8", assetType: "Keyboard", status: "ASSIGNED", owningUnit: "IT Operations", managingUnit: "HR", serialNumber: "KCK8-2001", brand: "Keychron", modelName: "K8" },
  { assetCode: "ADP-NB-3001", name: "MacBook Pro 14", assetType: "Laptop", status: "ASSIGNED", owningUnit: "IT Operations", managingUnit: "EML", serialNumber: "MBP14-3001", brand: "Apple", modelName: "MacBook Pro 14" },
  { assetCode: "ADP-NB-9001", name: "Old Latitude 5410", assetType: "Laptop", status: "RETIRED", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "DL5410-9001", brand: "Dell", modelName: "Latitude 5410" },
  { assetCode: "ADP-MON-9001", name: "Broken Samsung F24", assetType: "Monitor", status: "DISPOSED", owningUnit: "IT Operations", managingUnit: "IT Operations", serialNumber: "SMSF24-9001", brand: "Samsung", modelName: "F24" },
] as const;

function toDate(value: string) {
  return new Date(value);
}

function shouldSeedSampleData() {
  const explicit = process.env.SEED_SAMPLE_DATA?.trim();

  if (explicit) {
    return explicit === "true";
  }

  return process.env.NODE_ENV !== "production";
}

async function seedAccounts() {
  const isNonProduction = process.env.NODE_ENV !== "production";
  const username = process.env.SEED_SUPER_ADMIN_USERNAME?.trim() || "adman";
  const displayName = process.env.SEED_SUPER_ADMIN_DISPLAY_NAME?.trim() || username;
  const password =
    process.env.SEED_SUPER_ADMIN_PASSWORD?.trim() ||
    (isNonProduction && username === "adman" ? "20252026" : "ChangeMe-2026!");
  const recoveryCode =
    process.env.SEED_SUPER_ADMIN_RECOVERY_CODE?.trim() || "ADP-DEV-RECOVERY";
  const forcePasswordReset =
    process.env.SEED_SUPER_ADMIN_FORCE_PASSWORD_RESET === "true";
  const adminPasswordHash = await hashPassword(password);
  const recoveryCodeHash = recoveryCode ? await hashPassword(recoveryCode) : null;
  const itAdminPasswordHash = await hashPassword("ChangeMe-2026!");

  await prisma.localAccount.upsert({
    where: { username },
    create: {
      displayName,
      username,
      passwordHash: adminPasswordHash,
      recoveryCodeHash,
      role: "SUPER_ADMIN",
      forcePasswordReset,
      isActive: true,
    },
    update: {
      displayName,
      passwordHash: adminPasswordHash,
      recoveryCodeHash,
      role: "SUPER_ADMIN",
      forcePasswordReset,
      isActive: true,
    },
  });

  await prisma.localAccount.upsert({
    where: { username: "itadmin" },
    create: {
      displayName: "IT Admin",
      username: "itadmin",
      passwordHash: itAdminPasswordHash,
      role: "ADMIN",
      forcePasswordReset: true,
      isActive: true,
    },
    update: {
      displayName: "IT Admin",
      passwordHash: itAdminPasswordHash,
      role: "ADMIN",
      forcePasswordReset: true,
      isActive: true,
    },
  });

  console.info(`Seeded AssetDesk-Pro admin accounts: ${username}, itadmin`);
}

async function seedCoreDirectory() {
  for (const name of TEAM_NAMES) {
    await prisma.team.upsert({
      where: { name },
      create: { name },
      update: {},
    });
  }

  const teams = await prisma.team.findMany();
  const teamMap = new Map(teams.map((team) => [team.name, team]));

  for (const employee of EMPLOYEE_FIXTURES) {
    const team = teamMap.get(employee.teamName);

    if (!team) {
      throw new Error(`Missing team for employee seed: ${employee.teamName}`);
    }

    await prisma.employee.upsert({
      where: { employeeId: employee.employeeId },
      create: {
        employeeId: employee.employeeId,
        fullName: employee.fullName,
        teamId: team.id,
        email: employee.email,
        jobTitle: employee.jobTitle,
        staffGroup: "employee_list",
      },
      update: {
        fullName: employee.fullName,
        teamId: team.id,
        email: employee.email,
        jobTitle: employee.jobTitle,
        staffGroup: "employee_list",
      },
    });
  }

  for (const asset of ASSET_FIXTURES) {
    await prisma.asset.upsert({
      where: { assetCode: asset.assetCode },
      create: {
        assetCode: asset.assetCode,
        name: asset.name,
        assetType: asset.assetType,
        status: asset.status,
        recordedAt: toDate("2026-01-15T09:00:00.000Z"),
        owningUnit: asset.owningUnit,
        managingUnit: asset.managingUnit,
        serialNumber: asset.serialNumber,
        brand: asset.brand,
        modelName: asset.modelName,
        notes: "seed:core",
      },
      update: {
        name: asset.name,
        assetType: asset.assetType,
        status: asset.status,
        owningUnit: asset.owningUnit,
        managingUnit: asset.managingUnit,
        serialNumber: asset.serialNumber,
        brand: asset.brand,
        modelName: asset.modelName,
        notes: "seed:core",
      },
    });
  }
}

async function seedScenarioData() {
  const [adman, employees, assets] = await Promise.all([
    prisma.localAccount.findUniqueOrThrow({
      where: { username: process.env.SEED_SUPER_ADMIN_USERNAME?.trim() || "adman" },
    }),
    prisma.employee.findMany(),
    prisma.asset.findMany(),
  ]);

  const employeeMap = new Map(employees.map((employee) => [employee.employeeId, employee]));
  const assetMap = new Map(assets.map((asset) => [asset.assetCode, asset]));

  const ensureEmployee = (employeeId: string) => {
    const employee = employeeMap.get(employeeId);

    if (!employee) {
      throw new Error(`Missing employee seed: ${employeeId}`);
    }

    return employee;
  };

  const ensureAsset = (assetCode: string) => {
    const asset = assetMap.get(assetCode);

    if (!asset) {
      throw new Error(`Missing asset seed: ${assetCode}`);
    }

    return asset;
  };

  const receiveActiveSession = await prisma.receiveSession.upsert({
    where: { qrToken: "seed-receive-active-qr" },
    create: {
      sessionKey: "seed-receive-active-session",
      qrToken: "seed-receive-active-qr",
      createdByAccountId: adman.id,
      status: "ACTIVE",
      expiresAt: toDate("2026-12-31T23:59:59.000Z"),
      notes: "seed:active receive session",
    },
    update: {
      sessionKey: "seed-receive-active-session",
      createdByAccountId: adman.id,
      status: "ACTIVE",
      expiresAt: toDate("2026-12-31T23:59:59.000Z"),
      closedAt: null,
      notes: "seed:active receive session",
    },
  });

  const receiveReviewSession = await prisma.receiveSession.upsert({
    where: { qrToken: "seed-receive-review-qr" },
    create: {
      sessionKey: "seed-receive-review-session",
      qrToken: "seed-receive-review-qr",
      createdByAccountId: adman.id,
      status: "CLOSED",
      closedAt: toDate("2026-02-20T10:00:00.000Z"),
      notes: "seed:review receive session",
    },
    update: {
      sessionKey: "seed-receive-review-session",
      createdByAccountId: adman.id,
      status: "CLOSED",
      closedAt: toDate("2026-02-20T10:00:00.000Z"),
      notes: "seed:review receive session",
    },
  });

  const returnActiveSession = await prisma.returnSession.upsert({
    where: { qrToken: "seed-return-active-qr" },
    create: {
      sessionKey: "seed-return-active-session",
      qrToken: "seed-return-active-qr",
      createdByAccountId: adman.id,
      status: "ACTIVE",
      expiresAt: toDate("2026-12-31T23:59:59.000Z"),
      notes: "seed:active return session",
    },
    update: {
      sessionKey: "seed-return-active-session",
      createdByAccountId: adman.id,
      status: "ACTIVE",
      expiresAt: toDate("2026-12-31T23:59:59.000Z"),
      closedAt: null,
      notes: "seed:active return session",
    },
  });

  const returnReviewSession = await prisma.returnSession.upsert({
    where: { qrToken: "seed-return-review-qr" },
    create: {
      sessionKey: "seed-return-review-session",
      qrToken: "seed-return-review-qr",
      createdByAccountId: adman.id,
      status: "CLOSED",
      closedAt: toDate("2026-02-25T16:30:00.000Z"),
      notes: "seed:review return session",
    },
    update: {
      sessionKey: "seed-return-review-session",
      createdByAccountId: adman.id,
      status: "CLOSED",
      closedAt: toDate("2026-02-25T16:30:00.000Z"),
      notes: "seed:review return session",
    },
  });

  const activeAssignments = [
    { assignmentKey: "seed-assignment-nb-2001", assetCode: "ADP-NB-2001", employeeId: "ADP002", assignedAt: "2026-02-10T08:30:00.000Z" },
    { assignmentKey: "seed-assignment-ph-2001", assetCode: "ADP-PH-2001", employeeId: "ADP002", assignedAt: "2026-02-10T08:45:00.000Z" },
    { assignmentKey: "seed-assignment-nb-2002", assetCode: "ADP-NB-2002", employeeId: "ADP003", assignedAt: "2026-01-28T09:15:00.000Z" },
    { assignmentKey: "seed-assignment-mon-2001", assetCode: "ADP-MON-2001", employeeId: "ADP004", assignedAt: "2026-02-03T10:15:00.000Z" },
    { assignmentKey: "seed-assignment-kb-2001", assetCode: "ADP-KB-2001", employeeId: "ADP005", assignedAt: "2026-02-01T13:20:00.000Z" },
  ] as const;

  for (const assignment of activeAssignments) {
    const asset = ensureAsset(assignment.assetCode);
    const employee = ensureEmployee(assignment.employeeId);

    await prisma.assetAssignment.upsert({
      where: { assignmentKey: assignment.assignmentKey },
      create: {
        assignmentKey: assignment.assignmentKey,
        assetId: asset.id,
        employeeId: employee.id,
        status: "ACTIVE",
        assignedAt: toDate(assignment.assignedAt),
        notes: "seed:active assignment",
      },
      update: {
        assetId: asset.id,
        employeeId: employee.id,
        status: "ACTIVE",
        receiveRequestItemId: null,
        closedByReturnItemId: null,
        returnedAt: null,
        assignedAt: toDate(assignment.assignedAt),
        notes: "seed:active assignment",
      },
    });
  }

  const receivePendingRequest = await prisma.receiveRequest.upsert({
    where: { requestKey: "seed-receive-pending-001" },
    create: {
      requestKey: "seed-receive-pending-001",
      sessionId: receiveActiveSession.id,
      employeeId: ensureEmployee("ADP006").id,
      status: "PENDING",
      employeeCodeSnapshot: "ADP006",
      employeeNameSnapshot: ensureEmployee("ADP006").fullName,
      notes: "seed:pending receive request",
      submittedAt: toDate("2026-03-10T09:00:00.000Z"),
      items: {
        create: [
          { assetId: ensureAsset("ADP-NB-1001").id, assetCodeSnapshot: "ADP-NB-1001", assetNameSnapshot: ensureAsset("ADP-NB-1001").name },
          { assetId: ensureAsset("ADP-MON-1001").id, assetCodeSnapshot: "ADP-MON-1001", assetNameSnapshot: ensureAsset("ADP-MON-1001").name },
        ],
      },
    },
    update: {
      sessionId: receiveActiveSession.id,
      employeeId: ensureEmployee("ADP006").id,
      status: "PENDING",
      employeeCodeSnapshot: "ADP006",
      employeeNameSnapshot: ensureEmployee("ADP006").fullName,
      notes: "seed:pending receive request",
      submittedAt: toDate("2026-03-10T09:00:00.000Z"),
      finalizedAt: null,
      items: {
        deleteMany: {},
        create: [
          { assetId: ensureAsset("ADP-NB-1001").id, assetCodeSnapshot: "ADP-NB-1001", assetNameSnapshot: ensureAsset("ADP-NB-1001").name },
          { assetId: ensureAsset("ADP-MON-1001").id, assetCodeSnapshot: "ADP-MON-1001", assetNameSnapshot: ensureAsset("ADP-MON-1001").name },
        ],
      },
    },
    include: { items: true },
  });

  const receiveApprovedRequest = await prisma.receiveRequest.upsert({
    where: { requestKey: "seed-receive-approved-001" },
    create: {
      requestKey: "seed-receive-approved-001",
      sessionId: receiveReviewSession.id,
      employeeId: ensureEmployee("ADP007").id,
      status: "APPROVED",
      employeeCodeSnapshot: "ADP007",
      employeeNameSnapshot: ensureEmployee("ADP007").fullName,
      notes: "seed:approved receive request",
      submittedAt: toDate("2026-02-18T09:10:00.000Z"),
      finalizedAt: toDate("2026-02-18T11:00:00.000Z"),
      items: {
        create: [
          { assetId: ensureAsset("ADP-NB-3001").id, assetCodeSnapshot: "ADP-NB-3001", assetNameSnapshot: ensureAsset("ADP-NB-3001").name },
        ],
      },
    },
    update: {
      sessionId: receiveReviewSession.id,
      employeeId: ensureEmployee("ADP007").id,
      status: "APPROVED",
      employeeCodeSnapshot: "ADP007",
      employeeNameSnapshot: ensureEmployee("ADP007").fullName,
      notes: "seed:approved receive request",
      submittedAt: toDate("2026-02-18T09:10:00.000Z"),
      finalizedAt: toDate("2026-02-18T11:00:00.000Z"),
      items: {
        deleteMany: {},
        create: [
          { assetId: ensureAsset("ADP-NB-3001").id, assetCodeSnapshot: "ADP-NB-3001", assetNameSnapshot: ensureAsset("ADP-NB-3001").name },
        ],
      },
    },
    include: { items: true },
  });

  const receiveRejectedRequest = await prisma.receiveRequest.upsert({
    where: { requestKey: "seed-receive-rejected-001" },
    create: {
      requestKey: "seed-receive-rejected-001",
      sessionId: receiveReviewSession.id,
      employeeId: ensureEmployee("ADP008").id,
      status: "REJECTED",
      employeeCodeSnapshot: "ADP008",
      employeeNameSnapshot: ensureEmployee("ADP008").fullName,
      notes: "seed:rejected receive request",
      submittedAt: toDate("2026-02-19T09:20:00.000Z"),
      finalizedAt: toDate("2026-02-19T12:30:00.000Z"),
      items: {
        create: [
          { assetId: ensureAsset("ADP-NB-1002").id, assetCodeSnapshot: "ADP-NB-1002", assetNameSnapshot: ensureAsset("ADP-NB-1002").name },
        ],
      },
    },
    update: {
      sessionId: receiveReviewSession.id,
      employeeId: ensureEmployee("ADP008").id,
      status: "REJECTED",
      employeeCodeSnapshot: "ADP008",
      employeeNameSnapshot: ensureEmployee("ADP008").fullName,
      notes: "seed:rejected receive request",
      submittedAt: toDate("2026-02-19T09:20:00.000Z"),
      finalizedAt: toDate("2026-02-19T12:30:00.000Z"),
      items: {
        deleteMany: {},
        create: [
          { assetId: ensureAsset("ADP-NB-1002").id, assetCodeSnapshot: "ADP-NB-1002", assetNameSnapshot: ensureAsset("ADP-NB-1002").name },
        ],
      },
    },
    include: { items: true },
  });

  const approvedReceiveItem = receiveApprovedRequest.items[0];

  await prisma.assetAssignment.upsert({
    where: { assignmentKey: "seed-assignment-nb-3001" },
    create: {
      assignmentKey: "seed-assignment-nb-3001",
      assetId: approvedReceiveItem.assetId,
      employeeId: ensureEmployee("ADP007").id,
      status: "ACTIVE",
      assignedAt: toDate("2026-02-18T11:00:00.000Z"),
      receiveRequestItemId: approvedReceiveItem.id,
      notes: "seed:approved receive assignment",
    },
    update: {
      assetId: approvedReceiveItem.assetId,
      employeeId: ensureEmployee("ADP007").id,
      status: "ACTIVE",
      assignedAt: toDate("2026-02-18T11:00:00.000Z"),
      receiveRequestItemId: approvedReceiveItem.id,
      closedByReturnItemId: null,
      returnedAt: null,
      notes: "seed:approved receive assignment",
    },
  });

  await prisma.asset.update({
    where: { id: approvedReceiveItem.assetId },
    data: { status: "ASSIGNED" },
  });

  await prisma.approvalReview.upsert({
    where: { reviewKey: "seed-review-receive-approved-001" },
    create: {
      reviewKey: "seed-review-receive-approved-001",
      requestType: "RECEIVE",
      reviewerAccountId: adman.id,
      decision: "APPROVED",
      receiveRequestId: receiveApprovedRequest.id,
      notes: "seed:approved receive review",
      reviewedAt: toDate("2026-02-18T11:00:00.000Z"),
    },
    update: {
      requestType: "RECEIVE",
      reviewerAccountId: adman.id,
      decision: "APPROVED",
      receiveRequestId: receiveApprovedRequest.id,
      returnRequestId: null,
      notes: "seed:approved receive review",
      reviewedAt: toDate("2026-02-18T11:00:00.000Z"),
    },
  });

  await prisma.approvalReview.upsert({
    where: { reviewKey: "seed-review-receive-rejected-001" },
    create: {
      reviewKey: "seed-review-receive-rejected-001",
      requestType: "RECEIVE",
      reviewerAccountId: adman.id,
      decision: "REJECTED",
      receiveRequestId: receiveRejectedRequest.id,
      notes: "seed:rejected receive review",
      reviewedAt: toDate("2026-02-19T12:30:00.000Z"),
    },
    update: {
      requestType: "RECEIVE",
      reviewerAccountId: adman.id,
      decision: "REJECTED",
      receiveRequestId: receiveRejectedRequest.id,
      returnRequestId: null,
      notes: "seed:rejected receive review",
      reviewedAt: toDate("2026-02-19T12:30:00.000Z"),
    },
  });

  const approvedReturnAssignment = await prisma.assetAssignment.upsert({
    where: { assignmentKey: "seed-assignment-dock-1001" },
    create: {
      assignmentKey: "seed-assignment-dock-1001",
      assetId: ensureAsset("ADP-DOCK-1001").id,
      employeeId: ensureEmployee("ADP006").id,
      status: "RETURNED",
      assignedAt: toDate("2026-02-05T08:00:00.000Z"),
      returnedAt: toDate("2026-02-25T16:00:00.000Z"),
      notes: "seed:approved return assignment",
    },
    update: {
      assetId: ensureAsset("ADP-DOCK-1001").id,
      employeeId: ensureEmployee("ADP006").id,
      status: "RETURNED",
      assignedAt: toDate("2026-02-05T08:00:00.000Z"),
      returnedAt: toDate("2026-02-25T16:00:00.000Z"),
      notes: "seed:approved return assignment",
    },
  });

  const returnPendingAssignment = await prisma.assetAssignment.findUniqueOrThrow({
    where: { assignmentKey: "seed-assignment-ph-2001" },
  });

  const returnRejectedAssignment = await prisma.assetAssignment.findUniqueOrThrow({
    where: { assignmentKey: "seed-assignment-kb-2001" },
  });

  const returnPendingRequest = await prisma.returnRequest.upsert({
    where: { requestKey: "seed-return-pending-001" },
    create: {
      requestKey: "seed-return-pending-001",
      sessionId: returnActiveSession.id,
      employeeId: ensureEmployee("ADP002").id,
      status: "PENDING",
      employeeCodeSnapshot: "ADP002",
      employeeNameSnapshot: ensureEmployee("ADP002").fullName,
      notes: "seed:pending return request",
      submittedAt: toDate("2026-03-10T15:20:00.000Z"),
      items: {
        create: [
          {
            assetId: ensureAsset("ADP-PH-2001").id,
            assetAssignmentId: returnPendingAssignment.id,
            assetCodeSnapshot: "ADP-PH-2001",
            assetNameSnapshot: ensureAsset("ADP-PH-2001").name,
          },
        ],
      },
    },
    update: {
      sessionId: returnActiveSession.id,
      employeeId: ensureEmployee("ADP002").id,
      status: "PENDING",
      employeeCodeSnapshot: "ADP002",
      employeeNameSnapshot: ensureEmployee("ADP002").fullName,
      notes: "seed:pending return request",
      submittedAt: toDate("2026-03-10T15:20:00.000Z"),
      finalizedAt: null,
      items: {
        deleteMany: {},
        create: [
          {
            assetId: ensureAsset("ADP-PH-2001").id,
            assetAssignmentId: returnPendingAssignment.id,
            assetCodeSnapshot: "ADP-PH-2001",
            assetNameSnapshot: ensureAsset("ADP-PH-2001").name,
          },
        ],
      },
    },
    include: { items: true },
  });

  const returnApprovedRequest = await prisma.returnRequest.upsert({
    where: { requestKey: "seed-return-approved-001" },
    create: {
      requestKey: "seed-return-approved-001",
      sessionId: returnReviewSession.id,
      employeeId: ensureEmployee("ADP006").id,
      status: "APPROVED",
      employeeCodeSnapshot: "ADP006",
      employeeNameSnapshot: ensureEmployee("ADP006").fullName,
      notes: "seed:approved return request",
      submittedAt: toDate("2026-02-25T15:10:00.000Z"),
      finalizedAt: toDate("2026-02-25T16:00:00.000Z"),
      items: {
        create: [
          {
            assetId: ensureAsset("ADP-DOCK-1001").id,
            assetAssignmentId: approvedReturnAssignment.id,
            assetCodeSnapshot: "ADP-DOCK-1001",
            assetNameSnapshot: ensureAsset("ADP-DOCK-1001").name,
          },
        ],
      },
    },
    update: {
      sessionId: returnReviewSession.id,
      employeeId: ensureEmployee("ADP006").id,
      status: "APPROVED",
      employeeCodeSnapshot: "ADP006",
      employeeNameSnapshot: ensureEmployee("ADP006").fullName,
      notes: "seed:approved return request",
      submittedAt: toDate("2026-02-25T15:10:00.000Z"),
      finalizedAt: toDate("2026-02-25T16:00:00.000Z"),
      items: {
        deleteMany: {},
        create: [
          {
            assetId: ensureAsset("ADP-DOCK-1001").id,
            assetAssignmentId: approvedReturnAssignment.id,
            assetCodeSnapshot: "ADP-DOCK-1001",
            assetNameSnapshot: ensureAsset("ADP-DOCK-1001").name,
          },
        ],
      },
    },
    include: { items: true },
  });

  const returnRejectedRequest = await prisma.returnRequest.upsert({
    where: { requestKey: "seed-return-rejected-001" },
    create: {
      requestKey: "seed-return-rejected-001",
      sessionId: returnReviewSession.id,
      employeeId: ensureEmployee("ADP005").id,
      status: "REJECTED",
      employeeCodeSnapshot: "ADP005",
      employeeNameSnapshot: ensureEmployee("ADP005").fullName,
      notes: "seed:rejected return request",
      submittedAt: toDate("2026-02-26T14:40:00.000Z"),
      finalizedAt: toDate("2026-02-26T15:05:00.000Z"),
      items: {
        create: [
          {
            assetId: ensureAsset("ADP-KB-2001").id,
            assetAssignmentId: returnRejectedAssignment.id,
            assetCodeSnapshot: "ADP-KB-2001",
            assetNameSnapshot: ensureAsset("ADP-KB-2001").name,
          },
        ],
      },
    },
    update: {
      sessionId: returnReviewSession.id,
      employeeId: ensureEmployee("ADP005").id,
      status: "REJECTED",
      employeeCodeSnapshot: "ADP005",
      employeeNameSnapshot: ensureEmployee("ADP005").fullName,
      notes: "seed:rejected return request",
      submittedAt: toDate("2026-02-26T14:40:00.000Z"),
      finalizedAt: toDate("2026-02-26T15:05:00.000Z"),
      items: {
        deleteMany: {},
        create: [
          {
            assetId: ensureAsset("ADP-KB-2001").id,
            assetAssignmentId: returnRejectedAssignment.id,
            assetCodeSnapshot: "ADP-KB-2001",
            assetNameSnapshot: ensureAsset("ADP-KB-2001").name,
          },
        ],
      },
    },
    include: { items: true },
  });

  const approvedReturnItem = returnApprovedRequest.items[0];

  await prisma.assetAssignment.update({
    where: { id: approvedReturnAssignment.id },
    data: {
      status: "RETURNED",
      returnedAt: toDate("2026-02-25T16:00:00.000Z"),
      closedByReturnItemId: approvedReturnItem.id,
      notes: "seed:approved return assignment",
    },
  });

  await prisma.asset.update({
    where: { id: approvedReturnItem.assetId },
    data: { status: "IN_STOCK" },
  });

  await prisma.approvalReview.upsert({
    where: { reviewKey: "seed-review-return-approved-001" },
    create: {
      reviewKey: "seed-review-return-approved-001",
      requestType: "RETURN",
      reviewerAccountId: adman.id,
      decision: "APPROVED",
      returnRequestId: returnApprovedRequest.id,
      notes: "seed:approved return review",
      reviewedAt: toDate("2026-02-25T16:00:00.000Z"),
    },
    update: {
      requestType: "RETURN",
      reviewerAccountId: adman.id,
      decision: "APPROVED",
      receiveRequestId: null,
      returnRequestId: returnApprovedRequest.id,
      notes: "seed:approved return review",
      reviewedAt: toDate("2026-02-25T16:00:00.000Z"),
    },
  });

  await prisma.approvalReview.upsert({
    where: { reviewKey: "seed-review-return-rejected-001" },
    create: {
      reviewKey: "seed-review-return-rejected-001",
      requestType: "RETURN",
      reviewerAccountId: adman.id,
      decision: "REJECTED",
      returnRequestId: returnRejectedRequest.id,
      notes: "seed:rejected return review",
      reviewedAt: toDate("2026-02-26T15:05:00.000Z"),
    },
    update: {
      requestType: "RETURN",
      reviewerAccountId: adman.id,
      decision: "REJECTED",
      receiveRequestId: null,
      returnRequestId: returnRejectedRequest.id,
      notes: "seed:rejected return review",
      reviewedAt: toDate("2026-02-26T15:05:00.000Z"),
    },
  });

  const eventFixtures = [
    {
      eventKey: "seed-audit-asset-preload",
      actionType: "asset.preload",
      entityType: "asset_batch",
      entityId: "seed-core-assets",
      entityLabel: "seed-core-assets",
      metadata: { totalAssets: ASSET_FIXTURES.length },
    },
    {
      eventKey: "seed-audit-receive-pending",
      actionType: "receive_request.submit",
      entityType: "receive_request",
      entityId: receivePendingRequest.requestKey,
      entityLabel: receivePendingRequest.requestKey,
      receiveRequestId: receivePendingRequest.id,
      employeeId: ensureEmployee("ADP006").id,
      metadata: { assetCodes: receivePendingRequest.items.map((item) => item.assetCodeSnapshot) },
    },
    {
      eventKey: "seed-audit-receive-approved",
      actionType: "approval.review",
      entityType: "receive_request",
      entityId: receiveApprovedRequest.requestKey,
      entityLabel: receiveApprovedRequest.requestKey,
      receiveRequestId: receiveApprovedRequest.id,
      employeeId: ensureEmployee("ADP007").id,
      result: "SUCCESS" as const,
      metadata: { decision: "APPROVED", assetCodes: receiveApprovedRequest.items.map((item) => item.assetCodeSnapshot) },
    },
    {
      eventKey: "seed-audit-return-pending",
      actionType: "return_request.submit",
      entityType: "return_request",
      entityId: returnPendingRequest.requestKey,
      entityLabel: returnPendingRequest.requestKey,
      returnRequestId: returnPendingRequest.id,
      employeeId: ensureEmployee("ADP002").id,
      metadata: { assetCodes: returnPendingRequest.items.map((item) => item.assetCodeSnapshot) },
    },
    {
      eventKey: "seed-audit-return-approved",
      actionType: "approval.review",
      entityType: "return_request",
      entityId: returnApprovedRequest.requestKey,
      entityLabel: returnApprovedRequest.requestKey,
      returnRequestId: returnApprovedRequest.id,
      employeeId: ensureEmployee("ADP006").id,
      result: "SUCCESS" as const,
      metadata: { decision: "APPROVED", assetCodes: returnApprovedRequest.items.map((item) => item.assetCodeSnapshot) },
    },
    {
      eventKey: "seed-audit-return-rejected",
      actionType: "approval.review",
      entityType: "return_request",
      entityId: returnRejectedRequest.requestKey,
      entityLabel: returnRejectedRequest.requestKey,
      returnRequestId: returnRejectedRequest.id,
      employeeId: ensureEmployee("ADP005").id,
      result: "REJECTED" as const,
      metadata: { decision: "REJECTED", assetCodes: returnRejectedRequest.items.map((item) => item.assetCodeSnapshot) },
    },
  ];

  for (const event of eventFixtures) {
    await prisma.auditLog.upsert({
      where: { eventKey: event.eventKey },
      create: {
        eventKey: event.eventKey,
        actorAccountId: adman.id,
        actorUsername: adman.username,
        actionType: event.actionType,
        entityType: event.entityType,
        entityId: event.entityId,
        entityLabel: event.entityLabel,
        result: event.result ?? "SUCCESS",
        metadata: event.metadata,
        employeeId: event.employeeId,
        receiveRequestId: event.receiveRequestId,
        returnRequestId: event.returnRequestId,
        occurredAt: toDate("2026-03-01T09:00:00.000Z"),
      },
      update: {
        actorAccountId: adman.id,
        actorUsername: adman.username,
        actionType: event.actionType,
        entityType: event.entityType,
        entityId: event.entityId,
        entityLabel: event.entityLabel,
        result: event.result ?? "SUCCESS",
        metadata: event.metadata,
        employeeId: event.employeeId,
        receiveRequestId: event.receiveRequestId,
        returnRequestId: event.returnRequestId,
      },
    });
  }

  console.info("Seeded AssetDesk-Pro sample employees, assets, requests, reviews, and audit logs.");
}

async function main() {
  await seedAccounts();
  await seedCoreDirectory();

  if (shouldSeedSampleData()) {
    await seedScenarioData();
  }
}

main()
  .catch((error) => {
    console.error("Prisma seed failed", error);
    process.exitCode = 1;
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
