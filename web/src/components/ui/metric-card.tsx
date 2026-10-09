import type { LucideIcon } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

export function MetricCard({
  title,
  value,
  hint,
  icon: Icon,
  className,
}: {
  title: string
  value: string
  hint?: string
  icon?: LucideIcon
  className?: string
}) {
  return (
    <Card
      className={cn(
        'border-[var(--obsidian-dashboard-shell-border)] bg-[var(--obsidian-dashboard-shell-control)] shadow-[var(--obsidian-dashboard-shell-control-shadow)] transition-colors duration-150 hover:bg-[var(--obsidian-dashboard-shell-muted)]',
        className,
      )}
    >
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-xs font-medium text-[var(--obsidian-dashboard-shell-text-muted)]">
          {title}
        </CardTitle>
        {Icon ? (
          <Icon className="size-4 text-[var(--obsidian-dashboard-shell-text-subtle)]" />
        ) : null}
      </CardHeader>
      <CardContent>
        <div className="text-xl font-semibold tracking-tight">{value}</div>
        {hint ? (
          <p className="mt-1 text-xs text-[var(--obsidian-dashboard-shell-text-subtle)]">
            {hint}
          </p>
        ) : null}
      </CardContent>
    </Card>
  )
}
