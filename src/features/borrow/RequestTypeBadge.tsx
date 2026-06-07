type RequestTypeBadgeProps = {
  requestType: string | null | undefined
  size?: "sm" | "md"
}

export function RequestTypeBadge({ requestType, size = "sm" }: RequestTypeBadgeProps) {
  const isReturn = requestType === "return"
  const label = isReturn ? "Return" : "Borrow"
  const colorClass = isReturn
    ? "border-amber-500/40 bg-amber-500/10 text-amber-300"
    : "border-emerald-500/40 bg-emerald-500/10 text-emerald-300"
  const paddingClass = size === "md" ? "px-3 py-1" : "px-2 py-0.5"
  const textClass = size === "md" ? "text-[11px]" : "text-[10px]"
  return (
    <div
      className={`rounded-[999px] border ${colorClass} ${paddingClass} ${textClass} uppercase tracking-[0.06em]`}
    >
      {label}
    </div>
  )
}
