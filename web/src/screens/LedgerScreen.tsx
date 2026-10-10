import { useQuery } from '@tanstack/react-query'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useMemo, useRef, useState } from 'react'
import { DashboardButton } from '@/components/block/dashboard-shell'
import { Input } from '@/components/ui/input'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { formatDate, formatMoney } from '@/lib/format'
import { ipc, type LedgerRow } from '@/lib/ipc'
import { useSettings } from '@/lib/settings'

export function LedgerScreen({
  onQuickAdd,
  initialPayee,
  initialAccount,
}: {
  onQuickAdd?: () => void
  initialPayee?: string
  initialAccount?: string
}) {
  const { unit, persianDigits, dateStyle, isBusiness } = useSettings()
  const [q, setQ] = useState('')
  const [kind, setKind] = useState<string>('all')
  const [account, setAccount] = useState(initialAccount ?? '')
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [selected, setSelected] = useState<LedgerRow | null>(null)
  const parentRef = useRef<HTMLDivElement>(null)

  const { data = [], isLoading, isError, error } = useQuery({
    queryKey: ['ledger'],
    queryFn: () => ipc.listLedger(),
  })

  const accounts = useMemo(
    () =>
      Array.from(new Set(data.map((r) => r.account).filter(Boolean))).sort(),
    [data],
  )

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase()
    return data.filter((row) => {
      if (kind !== 'all' && row.kind !== kind) return false
      if (account && row.account !== account && !row.accountSummary.includes(account))
        return false
      if (initialPayee && row.payee !== initialPayee && row.actor !== initialPayee)
        return false
      if (from && row.date < from) return false
      if (to && row.date > to) return false
      if (!needle) return true
      return [
        row.memo,
        row.kind,
        row.actor,
        row.payee,
        row.category,
        row.account,
        row.accountSummary,
        row.date,
        String(row.amountRial),
      ]
        .join(' ')
        .toLowerCase()
        .includes(needle)
    })
  }, [data, q, kind, account, from, to, initialPayee])

  const virtualizer = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 52,
    overscan: 20,
  })

  const exportCsv = () => {
    const header = [
      'date',
      'payee',
      'category',
      'account',
      'amount_rial',
      'running_balance_rial',
      'kind',
      'memo',
    ]
    const lines = filtered.map((r) =>
      [
        r.date,
        csv(r.payee),
        csv(r.category),
        csv(r.account),
        r.amountRial,
        r.runningBalanceRial,
        csv(r.kind),
        csv(r.memo),
      ].join(','),
    )
    const blob = new Blob([[header.join(','), ...lines].join('\n')], {
      type: 'text/csv;charset=utf-8',
    })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'ledger.csv'
    a.click()
    URL.revokeObjectURL(url)
  }

  if (isLoading) {
    return <div className="sarv-page text-[var(--ink-muted)]">…</div>
  }
  if (isError) {
    return (
      <div className="sarv-page text-[var(--destructive)]">
        {(error as Error).message}
      </div>
    )
  }

  const money = (n: number) => formatMoney(n, unit, persianDigits)

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 p-3 md:p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-[18px] font-semibold">دفتر تراکنش‌ها</h1>
        <div className="flex gap-2">
          <DashboardButton variant="ghost" size="sm" onClick={exportCsv}>
            CSV
          </DashboardButton>
          <DashboardButton variant="primary" size="sm" onClick={onQuickAdd}>
            تازه
          </DashboardButton>
        </div>
      </div>

      <div className="flex flex-wrap gap-2">
        <Input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="جستجو: طرف، یادداشت، مبلغ…"
          className="h-10 min-w-[200px] flex-1"
        />
        <Input
          type="date"
          value={from}
          onChange={(e) => setFrom(e.target.value)}
          className="h-10 w-[140px]"
          dir="ltr"
        />
        <Input
          type="date"
          value={to}
          onChange={(e) => setTo(e.target.value)}
          className="h-10 w-[140px]"
          dir="ltr"
        />
      </div>

      <div className="flex flex-wrap gap-1.5">
        {['all', 'هزینه', 'درآمد', 'انتقال'].map((k) => (
          <Chip
            key={k}
            active={kind === k}
            onClick={() => setKind(k)}
            label={k === 'all' ? 'همه' : k}
          />
        ))}
        {accounts.slice(0, 8).map((a) => (
          <Chip
            key={a}
            active={account === a}
            onClick={() => setAccount(account === a ? '' : a)}
            label={a}
          />
        ))}
      </div>

      {filtered.length === 0 ? (
        <div className="sarv-panel px-4 py-12 text-center text-[13px] text-[var(--ink-muted)]">
          تراکنشی نیست — با Ctrl+N ثبت کنید
        </div>
      ) : (
        <div
          ref={parentRef}
          className="sarv-panel min-h-0 flex-1 overflow-auto"
        >
          <div
            className="sticky top-0 z-10 grid grid-cols-[96px_1fr_1fr_1fr_auto_auto] gap-2 border-b border-[var(--line)] bg-[var(--surface)] px-3 py-2 text-[11px] text-[var(--ink-muted)]"
          >
            <span>تاریخ</span>
            <span>طرف</span>
            <span>دسته</span>
            <span>حساب</span>
            <span>مبلغ</span>
            <span>مانده</span>
          </div>
          <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = filtered[item.index]!
              return (
                <button
                  key={row.id}
                  type="button"
                  className="absolute inset-x-0 grid grid-cols-[96px_1fr_1fr_1fr_auto_auto] items-center gap-2 border-b border-[var(--line)] px-3 text-right"
                  style={{
                    height: item.size,
                    transform: `translateY(${item.start}px)`,
                  }}
                  onClick={() => setSelected(row)}
                >
                  <span className="tabular-nums text-[12px] text-[var(--ink-muted)]">
                    {formatDate(row.date, dateStyle, persianDigits)}
                  </span>
                  <span className="truncate text-[13px] font-medium">
                    {row.payee || row.actor}
                  </span>
                  <span className="truncate text-[12px] text-[var(--ink-muted)]">
                    {row.category || '—'}
                  </span>
                  <span className="truncate text-[12px] text-[var(--ink-muted)]">
                    {row.account || row.accountSummary}
                  </span>
                  <span
                    className={`tabular-nums text-[13px] font-medium ${
                      row.signedAmountRial < 0
                        ? 'text-rose-700 dark:text-rose-400'
                        : 'text-emerald-700 dark:text-emerald-400'
                    }`}
                  >
                    {money(row.amountRial)}
                  </span>
                  <span className="tabular-nums text-[12px] text-[var(--ink-muted)]">
                    {money(row.runningBalanceRial)}
                  </span>
                </button>
              )
            })}
          </div>
        </div>
      )}

      <Sheet open={!!selected} onOpenChange={(o) => !o && setSelected(null)}>
        <SheetContent side="left" className="w-full sm:max-w-md">
          {selected ? (
            <>
              <SheetHeader>
                <SheetTitle>{selected.memo}</SheetTitle>
              </SheetHeader>
              <div className="mt-4 space-y-3 text-[13px]">
                <Row label="تاریخ">
                  {formatDate(selected.date, dateStyle, persianDigits)}
                </Row>
                <Row label="طرف">{selected.payee}</Row>
                <Row label="حساب">{selected.accountSummary}</Row>
                <Row label="مبلغ">{money(selected.amountRial)}</Row>
                <Row label="ماندهٔ جاری">
                  {money(selected.runningBalanceRial)}
                </Row>
                <Row label="وضعیت">{selected.status}</Row>
                {selected.attachmentName ? (
                  <Row label="پیوست">{selected.attachmentName}</Row>
                ) : null}
                {isBusiness ? (
                  <div className="sarv-panel p-3">
                    <p className="mb-2 text-[12px] font-semibold">خطوط دفتر</p>
                    {selected.splits.map((s, i) => (
                      <div
                        key={`${s.category}-${i}`}
                        className="flex justify-between gap-2 py-1 text-[12px]"
                      >
                        <span>{s.category || '—'}</span>
                        <span className="tabular-nums" dir="ltr">
                          Dr {s.debitRial} / Cr {s.creditRial}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="sarv-panel p-3">
                    <p className="mb-2 text-[12px] font-semibold">تفکیک</p>
                    {selected.splits.map((s, i) => (
                      <div
                        key={`${s.category}-${i}`}
                        className="flex justify-between gap-2 py-1 text-[12px]"
                      >
                        <span>{s.category || '—'}</span>
                        <span>{money(s.amountRial)}</span>
                      </div>
                    ))}
                  </div>
                )}
                <p className="text-[11px] text-[var(--ink-muted)]">
                  اصلاحات با برگشت و ثبت مجدد انجام می‌شود — ویرایش خاموش.
                </p>
              </div>
            </>
          ) : null}
        </SheetContent>
      </Sheet>
    </div>
  )
}

function Chip({
  label,
  active,
  onClick,
}: {
  label: string
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`min-h-9 rounded-[8px] px-3 text-[12px] ${
        active
          ? 'bg-[var(--cta)] text-[var(--primary-foreground)]'
          : 'bg-[var(--surface)] ring-1 ring-[var(--line)]'
      }`}
    >
      {label}
    </button>
  )
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <span className="text-[var(--ink-muted)]">{label}</span>
      <span className="text-left">{children}</span>
    </div>
  )
}

function csv(value: string) {
  const v = value.replaceAll('"', '""')
  return `"${v}"`
}
