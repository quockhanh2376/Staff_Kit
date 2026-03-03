export type TeamName = "Engineering" | "Design" | "Marketing" | "HR"

export type Employee = {
  id: number
  employeeId: string
  fullName: string
  team: TeamName
  email: string
  computerName: string
  startDate: string
  notes: string
  status: "Active" | "Onboarding" | "Inactive"
}

export const employees: Employee[] = [
  {
    id: 1,
    employeeId: "SK001",
    fullName: "Nguyen Van An",
    team: "Engineering",
    email: "an.nguyen@staffkit.io",
    computerName: "MAC-PRO-001",
    startDate: "2023-01-12",
    notes: "Lead UI Developer",
    status: "Active",
  },
  {
    id: 2,
    employeeId: "SK002",
    fullName: "Tran Thi Bich",
    team: "Design",
    email: "bich.tran@staffkit.io",
    computerName: "WIN-DSG-042",
    startDate: "2023-02-05",
    notes: "Senior Product Designer",
    status: "Active",
  },
  {
    id: 3,
    employeeId: "SK003",
    fullName: "Le Hoang Nam",
    team: "Marketing",
    email: "nam.le@staffkit.io",
    computerName: "MAC-AIR-015",
    startDate: "2023-03-10",
    notes: "Content Specialist",
    status: "Active",
  },
  {
    id: 4,
    employeeId: "SK004",
    fullName: "Pham Minh Duc",
    team: "Engineering",
    email: "duc.pham@staffkit.io",
    computerName: "MAC-PRO-088",
    startDate: "2023-05-20",
    notes: "DevOps Engineer",
    status: "Onboarding",
  },
  {
    id: 5,
    employeeId: "SK005",
    fullName: "Hoang Anh Dung",
    team: "HR",
    email: "dung.hoang@staffkit.io",
    computerName: "WIN-HR-011",
    startDate: "2023-06-01",
    notes: "Talent Acquisition",
    status: "Active",
  },
  {
    id: 6,
    employeeId: "SK006",
    fullName: "Dang Thu Thao",
    team: "Engineering",
    email: "thao.dang@staffkit.io",
    computerName: "MAC-PRO-102",
    startDate: "2023-07-15",
    notes: "Frontend Developer",
    status: "Inactive",
  },
  {
    id: 7,
    employeeId: "SK007",
    fullName: "Vu Minh Huy",
    team: "Design",
    email: "huy.vu@staffkit.io",
    computerName: "WIN-DSG-009",
    startDate: "2023-08-22",
    notes: "Graphic Designer",
    status: "Active",
  },
]

export const teams: Array<{ name: TeamName; members: number }> = [
  { name: "Engineering", members: 12 },
  { name: "Design", members: 8 },
  { name: "Marketing", members: 6 },
  { name: "HR", members: 4 },
]

export const excelPreview: Array<Record<string, string>> = [
  {
    employeeId: "NV001",
    name: "Nguyen Van A",
    team: "IT",
    email: "it@company.vn",
  },
  {
    employeeId: "NV002",
    name: "Tran Thi B",
    team: "HR",
    email: "hr@company.vn",
  },
  {
    employeeId: "NV003",
    name: "Le Van C",
    team: "Sales",
    email: "sales@company.vn",
  },
  {
    employeeId: "NV004",
    name: "Pham Thi D",
    team: "Marketing",
    email: "mkt@company.vn",
  },
  {
    employeeId: "NV005",
    name: "Hoang Van E",
    team: "IT",
    email: "it2@company.vn",
  },
]

