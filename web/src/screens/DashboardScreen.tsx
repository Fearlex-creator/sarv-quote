import { useQuery } from '@tanstack/react-query'
import { useMemo, useState } from 'react'
import { formatDate, formatMoney } from '@/lib/format'
import { ipc } from '@/lib/ipc'
import type { ScreenId } from '@/lib/navigation'
import { useSettings } from '@/lib/settings'

type Range = '30' | '90' | 'all'

export function DashboardScreen({
  onNavigate,
}: {
  onNavigate: (id: ScreenId) => void
}) {
  const { unit, persianDigits, dateStyle, isBusiness } = useSettings()
  const [range, setRange] = useState<Range>('30')
  const { data, isLoading, isError, error } = useQuery({
    queryKey: ['dashboard'],
    queryFn: () => ipc.getDashboard(),
  })

  const recent = useMemo(() => {
    if (!data) return []
    if (range === 'all') return data.recent
    const days = Number(range)
    const cutoff = Date.now() - days * 86400000
    return data.recent.filter((r) => {
      const t = Date.parse(r.date)
      return Number.isFinite(t) ? t >= cutoff : true
    })
  }, [data, range])

  if (isLoading) {
    return <div className="sarv-page text-[var(--ink-muted)]">در حال خواندن…</div>
  }
  if (isError || !data) {
    return (
      <div className="sarv-page text-[var(--destructive)]">
        {(error as Error)?.message ?? 'خطا'}
      </div>
    )
  }

  const money = (n: number) => formatMoney(n, unit, persianDigits)

  return (
    <div className="sarv-page space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-[20px] font-semibold tracking-tight">
            {data.businessName}
          </h1>
          <p className="text-[13px] text-[var(--ink-muted)]">
            {data.modeLabel} · {data.payableHint}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {(['30', '90', 'all'] as const).map((id) => (
            <button
              key={id}
              type="button"
              onClick={() => setRange(id)}
              className={`min-h-10 rounded-[8px] px-3 text-[12px] font-medium ${
                range === id
                  ? 'bg-[var(--cta)] text-[var(--primary-foreground)]'
                  : 'bg-[var(--surface)] text-[var(--ink-soft)] ring-1 ring-[var(--line)]'
              }`}
            >
              {id === 'all' ? 'همه' : `${id} روز`}
            </button>
          ))}
          <button
            type="button"
            className="min-h-10 text-[13px] font-medium text-[var(--focus)]"
            onClick={() => onNavigate('quick-add')}
          >
            ثبت سریع ⌃N
          </button>
        </div>
      </div>

      <div className="sarv-strip">
        <div>
          <div className="label">موجودی نقد</div>
          <div className="value">{money(data.cashOnHandRial)}</div>
        </div>
        <div>
          <div className="label">درآمد</div>
          <div className="value text-emerald-700 dark:text-emerald-400">
            {money(data.incomeRial)}
          </div>
        </div>
        <div>
          <div className="label">هزینه</div>
          <div className="value text-rose-700 dark:text-rose-400">
            {money(data.expenseRial)}
          </div>
        </div>
        <div>
          <div className="label">سود / زیان</div>
          <div
            className={`value ${
              data.profitRial >= 0
                ? 'text-emerald-700 dark:text-emerald-400'
                : 'text-rose-700 dark:text-rose-400'
            }`}
          >
            {money(data.profitRial)}
          </div>
        </div>
      </div>

      <div className="grid gap-3 md:grid-cols-2">
        <section className="sarv-panel overflow-hidden">
          <header className="flex items-center justify-between px-4 py-3">
            <h2 className="text-[13px] font-semibold">طلب‌ها</h2>
            <span className="text-[11px] text-[var(--ink-muted)]">
              {data.receivables.length}
            </span>
          </header>
          {data.receivables.length === 0 ? (
            <p className="px-4 py-6 text-[12px] text-[var(--ink-muted)]">
              طلبی ثبت نشده
            </p>
          ) : (
            data.receivables.map((p) => (
              <div key={p.name} className="sarv-row grid-cols-[1fr_auto]">
                <span className="truncate text-[13px]">{p.name}</span>
                <span className="tabular-nums text-[13px] font-medium">
                  {money(p.amountRial)}
                </span>
              </div>
            ))
          )}
        </section>
        <section className="sarv-panel overflow-hidden">
          <header className="flex items-center justify-between px-4 py-3">
            <h2 className="text-[13px] font-semibold">بدهی‌ها</h2>
            <span className="text-[11px] text-[var(--ink-muted)]">
              {data.payables.length}
            </span>
          </header>
          {data.payables.length === 0 ? (
            <p className="px-4 py-6 text-[12px] text-[var(--ink-muted)]">
              بدهی ثبت نشده
            </p>
          ) : (
            data.payables.map((p) => (
              <div key={p.name} className="sarv-row grid-cols-[1fr_auto]">
                <span className="truncate text-[13px]">{p.name}</span>
                <span className="tabular-nums text-[13px] font-medium">
                  {money(p.amountRial)}
                </span>
              </div>
            ))
          )}
        </section>
      </div>

      {isBusiness ? (
        <section className="sarv-panel px-4 py-3 text-[12px] text-[var(--ink-muted)]">
          هشدار موجودی کم و چک‌های سررسید — فاز ۲ (جایگاه آماده است).
        </section>
      ) : null}

      <section className="sarv-panel overflow-hidden">
        <div className="flex items-center justify-between px-4 py-3">
          <h2 className="text-[13px] font-semibold">اخیر</h2>
          <button
            type="button"
            className="text-[12px] text-[var(--ink-muted)]"
            onClick={() => onNavigate('ledger')}
          >
            همه
          </button>
        </div>
        {recent.length === 0 ? (
          <button
            type="button"
            onClick={() => onNavigate('quick-add')}
            className="block w-full px-4 py-10 text-center text-[13px] text-[var(--ink-muted)]"
          >
            هنوز چیزی ثبت نشده — اولین هزینه یا درآمد را بزنید
          </button>
        ) : (
          recent.map((row) => (
            <div key={row.id} className="sarv-row">
              <span className="tabular-nums text-[12px] text-[var(--ink-muted)]">
                {formatDate(row.date, dateStyle, persianDigits)}
              </span>
              <div className="min-w-0">
                <p className="truncate text-[14px] font-medium">{row.memo}</p>
                <p className="truncate text-[12px] text-[var(--ink-muted)]">
                  {row.accountSummary}
                </p>
              </div>
              <span
                className={`tabular-nums text-[14px] font-medium ${
                  row.signedAmountRial < 0
                    ? 'text-rose-700 dark:text-rose-400'
                    : row.signedAmountRial > 0
                      ? 'text-emerald-700 dark:text-emerald-400'
                      : ''
                }`}
              >
                {money(row.amountRial)}
              </span>
            </div>
          ))
        )}
      </section>
    </div>
  )
}
