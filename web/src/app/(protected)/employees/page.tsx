import { getEmployeeDirectoryPreview } from "@/lib/admin/admin.service";

export default async function EmployeesPage() {
  const employees = await getEmployeeDirectoryPreview();

  return (
    <main className="rounded-[28px] border border-border bg-surface px-6 py-6 backdrop-blur">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-accent">
            Employees
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight">
            Staff directory preview
          </h1>
          <p className="mt-2 max-w-3xl text-sm leading-6 text-muted">
            This is the minimal admin screen that proves seeded Staff IDs and team mapping
            are ready for asset flows and future import screens.
          </p>
        </div>
        <div className="rounded-full border border-border bg-surface-strong px-4 py-2 text-sm text-muted">
          Showing {employees.length} seeded employees
        </div>
      </div>

      <div className="mt-6 overflow-hidden rounded-[24px] border border-border">
        <table className="min-w-full divide-y divide-border text-sm">
          <thead className="bg-surface-strong">
            <tr className="text-left text-xs uppercase tracking-[0.18em] text-muted">
              <th className="px-4 py-3">Staff ID</th>
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Team</th>
              <th className="px-4 py-3">Job Title</th>
              <th className="px-4 py-3">Active Assets</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {employees.map((employee) => (
              <tr key={employee.id} className="bg-surface/50">
                <td className="px-4 py-3 font-mono text-sm text-foreground">
                  {employee.employeeId}
                </td>
                <td className="px-4 py-3 font-medium text-foreground">
                  {employee.fullName}
                </td>
                <td className="px-4 py-3 text-muted">{employee.team?.name ?? "Unassigned"}</td>
                <td className="px-4 py-3 text-muted">{employee.jobTitle ?? "-"}</td>
                <td className="px-4 py-3 text-muted">{employee.assetAssignments.length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </main>
  );
}
