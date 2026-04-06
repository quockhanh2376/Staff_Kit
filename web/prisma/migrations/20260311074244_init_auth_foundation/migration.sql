-- CreateEnum
CREATE TYPE "AccountRole" AS ENUM ('SUPER_ADMIN', 'ADMIN', 'USER');

-- CreateTable
CREATE TABLE "local_accounts" (
    "id" SERIAL NOT NULL,
    "accountKey" TEXT NOT NULL,
    "displayName" TEXT NOT NULL,
    "username" TEXT NOT NULL,
    "passwordHash" TEXT NOT NULL,
    "recoveryCodeHash" TEXT,
    "role" "AccountRole" NOT NULL DEFAULT 'USER',
    "forcePasswordReset" BOOLEAN NOT NULL DEFAULT true,
    "isActive" BOOLEAN NOT NULL DEFAULT true,
    "lastLoginAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "local_accounts_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "teams" (
    "id" SERIAL NOT NULL,
    "name" TEXT NOT NULL,
    "parentId" INTEGER,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "teams_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "employees" (
    "id" SERIAL NOT NULL,
    "employeeId" TEXT NOT NULL,
    "fullName" TEXT NOT NULL,
    "nickName" TEXT,
    "teamId" INTEGER,
    "project" TEXT,
    "jobTitle" TEXT,
    "email" TEXT,
    "cellphone" TEXT,
    "dateOfBirth" TIMESTAMP(3),
    "gender" TEXT,
    "aswStartDate" TIMESTAMP(3),
    "clientStartDate" TIMESTAMP(3),
    "contractEndDate" TIMESTAMP(3),
    "clientYearOfServices" TEXT,
    "startDate" TIMESTAMP(3),
    "computerName" TEXT,
    "notes" TEXT,
    "staffGroup" TEXT NOT NULL DEFAULT 'employee_list',
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "employees_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "employee_dynamic_fields" (
    "id" SERIAL NOT NULL,
    "key" TEXT NOT NULL,
    "label" TEXT NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "employee_dynamic_fields_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "employee_dynamic_values" (
    "id" SERIAL NOT NULL,
    "employeeId" INTEGER NOT NULL,
    "fieldId" INTEGER NOT NULL,
    "value" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "employee_dynamic_values_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "app_settings" (
    "key" TEXT NOT NULL,
    "value" TEXT,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "app_settings_pkey" PRIMARY KEY ("key")
);

-- CreateIndex
CREATE UNIQUE INDEX "local_accounts_accountKey_key" ON "local_accounts"("accountKey");

-- CreateIndex
CREATE UNIQUE INDEX "local_accounts_username_key" ON "local_accounts"("username");

-- CreateIndex
CREATE INDEX "local_accounts_username_idx" ON "local_accounts"("username");

-- CreateIndex
CREATE INDEX "local_accounts_role_idx" ON "local_accounts"("role");

-- CreateIndex
CREATE INDEX "local_accounts_isActive_idx" ON "local_accounts"("isActive");

-- CreateIndex
CREATE UNIQUE INDEX "teams_name_key" ON "teams"("name");

-- CreateIndex
CREATE UNIQUE INDEX "employees_employeeId_key" ON "employees"("employeeId");

-- CreateIndex
CREATE UNIQUE INDEX "employees_email_key" ON "employees"("email");

-- CreateIndex
CREATE INDEX "employees_staffGroup_idx" ON "employees"("staffGroup");

-- CreateIndex
CREATE INDEX "employees_teamId_idx" ON "employees"("teamId");

-- CreateIndex
CREATE UNIQUE INDEX "employee_dynamic_fields_key_key" ON "employee_dynamic_fields"("key");

-- CreateIndex
CREATE INDEX "employee_dynamic_values_fieldId_idx" ON "employee_dynamic_values"("fieldId");

-- CreateIndex
CREATE UNIQUE INDEX "employee_dynamic_values_employeeId_fieldId_key" ON "employee_dynamic_values"("employeeId", "fieldId");

-- AddForeignKey
ALTER TABLE "teams" ADD CONSTRAINT "teams_parentId_fkey" FOREIGN KEY ("parentId") REFERENCES "teams"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "employees" ADD CONSTRAINT "employees_teamId_fkey" FOREIGN KEY ("teamId") REFERENCES "teams"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "employee_dynamic_values" ADD CONSTRAINT "employee_dynamic_values_employeeId_fkey" FOREIGN KEY ("employeeId") REFERENCES "employees"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "employee_dynamic_values" ADD CONSTRAINT "employee_dynamic_values_fieldId_fkey" FOREIGN KEY ("fieldId") REFERENCES "employee_dynamic_fields"("id") ON DELETE CASCADE ON UPDATE CASCADE;
