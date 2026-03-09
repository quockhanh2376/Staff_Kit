import type { ReactNode } from "react"
import { X } from "lucide-react"

export function Drawer({
    open,
    onClose,
    title,
    widthClass,
    children,
}: {
    open: boolean
    onClose: () => void
    title: string
    widthClass: string
    children: ReactNode
}) {
    if (!open) return null

    return (
        <div className="fixed inset-0 z-50 flex justify-end bg-black/50 backdrop-blur-[1px]">
            <div className={`h-full ${widthClass} max-w-[100vw] border-l border-[var(--border)] bg-[var(--surface)] shadow-2xl`}>
                <div className="flex h-16 items-center justify-between border-b border-[var(--border)] px-5">
                    <h3 className="text-lg font-semibold text-[var(--text-primary)]">{title}</h3>
                    <button
                        className="rounded-md p-2 text-[var(--text-secondary)] transition hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
                        onClick={onClose}
                        type="button"
                    >
                        <X size={18} />
                    </button>
                </div>
                <div className="h-[calc(100%-4rem)] overflow-auto p-5">{children}</div>
            </div>
        </div>
    )
}
