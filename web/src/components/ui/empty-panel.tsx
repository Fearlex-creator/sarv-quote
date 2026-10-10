import type { LucideIcon } from 'lucide-react'
import { DashboardButton } from '@/components/block/dashboard-shell'

export function EmptyPanel({
  icon: Icon,
  title,
  description,
  actionLabel,
  onAction,
}: {
  icon: LucideIcon
  title: string
  description: string
  actionLabel?: string
  onAction?: () => void
}) {
  return (
    <div className="flex min-h-[280px] flex-col items-center justify-center gap-3 rounded-[12px] border border-dashed border-[var(--obsidian-dashboard-shell-border)] bg-[var(--obsidian-dashboard-shell-panel)] px-6 py-12 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-[var(--obsidian-dashboard-shell-muted)]">
        <Icon className="size-5 text-[var(--obsidian-dashboard-shell-foreground)]" />
      </div>
      <div className="space-y-1">
        <h3 className="text-sm font-semibold">{title}</h3>
        <p className="max-w-md text-sm text-[var(--obsidian-dashboard-shell-text-muted)]">
          {description}
        </p>
      </div>
      {actionLabel && onAction ? (
        <DashboardButton variant="primary" size="sm" onClick={onAction}>
          {actionLabel}
        </DashboardButton>
      ) : null}
    </div>
  )
}
