import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { DashboardButton } from '@/components/block/dashboard-shell'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { formatMoney, parseAmount } from '@/lib/format'
import { ipc, type AccountRow, type WalletInput } from '@/lib/ipc'
import { useSettings } from '@/lib/settings'

const KINDS = [
  ['cash', 'نقد'],
  ['bank', 'بانک'],
  ['card', 'کارت'],
  ['wallet', 'کیف پول'],
  ['savings', 'پس‌انداز'],
  ['loan', 'وام'],
] as const

export function AccountsScreen({
  onOpenLedger,
}: {
  onOpenLedger?: (account: string) => void
}) {
  const { unit, persianDigits } = useSettings()
  const qc = useQueryClient()
  const [showArchived, setShowArchived] = useState(false)
  const [open, setOpen] = useState(false)
  const [form, setForm] = useState<WalletInput>({
    name: '',
    kind: 'cash',
    group: 'اصلی',
    openingBalanceRial: 0,
  })
  const [openingDisplay, setOpeningDisplay] = useState('')
  const [error, setError] = useState<string | null>(null)

  const { data = [], isLoading, isError, error: loadError } = useQuery({
    queryKey: ['accounts'],
    queryFn: () => ipc.listAccounts(),
  })

  const wallets = useMemo(
    () =>
      data.filter(
        (a) =>
          (showArchived || !a.archived) &&
          !a.id.startsWith('acc-') &&
          a.postable,
      ),
    [data, showArchived],
  )

  const grouped = useMemo(() => {
    const map = new Map<string, AccountRow[]>()
    for (const a of wallets) {
      const g = a.group || 'اصلی'
      if (!map.has(g)) map.set(g, [])
      map.get(g)!.push(a)
    }
    return [...map.entries()]
  }, [wallets])

  const net = wallets
    .filter((a) => !a.archived)
    .reduce((s, a) => s + a.balanceRial, 0)

  const save = useMutation({
    mutationFn: () => {
      const opening = openingDisplay
        ? parseAmount(openingDisplay, unit) ?? 0
        : (form.openingBalanceRial ?? 0)
      return ipc.saveWallet({ ...form, openingBalanceRial: opening })
    },
    onSuccess: async () => {
      setOpen(false)
      setError(null)
      await qc.invalidateQueries({ queryKey: ['accounts'] })
      await qc.invalidateQueries({ queryKey: ['dashboard'] })
      await qc.invalidateQueries({ queryKey: ['suggestions'] })
    },
    onError: (e: Error) => setError(e.message),
  })

  if (isLoading) {
    return <div className="sarv-page text-[var(--ink-muted)]">…</div>
  }
  if (isError) {
    return (
      <div className="sarv-page text-[var(--destructive)]">
        {(loadError as Error).message}
      </div>
    )
  }

  const money = (n: number) => formatMoney(n, unit, persianDigits)

  return (
    <div className="sarv-page space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h1 className="text-[18px] font-semibold">حساب‌ها و کیف‌ها</h1>
          <p className="text-[13px] text-[var(--ink-muted)]">
            خالص {money(net)}
          </p>
        </div>
        <div className="flex gap-2">
          <label className="flex min-h-10 items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              checked={showArchived}
              onChange={(e) => setShowArchived(e.target.checked)}
            />
            بایگانی
          </label>
          <DashboardButton
            variant="primary"
            size="sm"
            onClick={() => {
              setForm({
                name: '',
                kind: 'cash',
                group: 'اصلی',
                openingBalanceRial: 0,
              })
              setOpeningDisplay('')
              setOpen(true)
            }}
          >
            <Plus className="size-3.5" />
            حساب
          </DashboardButton>
        </div>
      </div>

      {grouped.map(([group, rows]) => (
        <section key={group} className="sarv-panel overflow-hidden">
          <header className="px-4 py-2.5 text-[12px] font-semibold text-[var(--ink-muted)]">
            {group}
          </header>
          {rows.map((a) => (
            <button
              key={a.id}
              type="button"
              className="sarv-row grid-cols-[1fr_auto] w-full text-right"
              onClick={() => {
                setForm({
                  id: a.id,
                  name: a.name,
                  kind: kindFromFa(a.accountType),
                  group: a.group,
                  openingBalanceRial: a.openingBalanceRial,
                  archived: a.archived,
                  sortOrder: a.sortOrder,
                  currency: a.currency,
                })
                setOpeningDisplay(
                  unit === 'toman'
                    ? String(Math.trunc(a.openingBalanceRial / 10))
                    : String(a.openingBalanceRial),
                )
                setOpen(true)
              }}
            >
              <div className="min-w-0">
                <p className="truncate text-[14px] font-medium">
                  {a.name}
                  {a.archived ? ' (بایگانی)' : ''}
                </p>
                <p className="text-[12px] text-[var(--ink-muted)]">
                  {a.accountType}
                  {onOpenLedger ? ' · کلیک برای دفتر' : ''}
                </p>
              </div>
              <span className="tabular-nums text-[14px] font-semibold">
                {money(a.balanceRial)}
              </span>
            </button>
          ))}
        </section>
      ))}

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent side="left" className="w-full sm:max-w-md">
          <SheetHeader>
            <SheetTitle>{form.id ? 'ویرایش حساب' : 'حساب تازه'}</SheetTitle>
          </SheetHeader>
          <form
            className="mt-4 space-y-3"
            onSubmit={(e) => {
              e.preventDefault()
              save.mutate()
            }}
          >
            <Field label="نام">
              <Input
                required
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
              />
            </Field>
            <Field label="نوع">
              <Select
                value={form.kind ?? 'cash'}
                onValueChange={(v) =>
                  setForm({ ...form, kind: v as WalletInput['kind'] })
                }
              >
                <SelectTrigger className="h-10">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {KINDS.map(([id, label]) => (
                    <SelectItem key={id} value={id}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="گروه">
              <Input
                value={form.group ?? ''}
                onChange={(e) => setForm({ ...form, group: e.target.value })}
              />
            </Field>
            <Field label={`مانده افتتاحیه (${unit === 'rial' ? 'ریال' : 'تومان'})`}>
              <Input
                dir="ltr"
                value={openingDisplay}
                onChange={(e) => setOpeningDisplay(e.target.value)}
              />
            </Field>
            {form.id ? (
              <label className="flex min-h-10 items-center gap-2 text-[13px]">
                <input
                  type="checkbox"
                  checked={!!form.archived}
                  onChange={(e) =>
                    setForm({ ...form, archived: e.target.checked })
                  }
                />
                بایگانی (حذف سخت نیست)
              </label>
            ) : null}
            {error ? (
              <p className="text-[12px] text-[var(--destructive)]">{error}</p>
            ) : null}
            <div className="flex gap-2">
              <DashboardButton
                type="submit"
                variant="primary"
                size="md"
                className="flex-1"
                disabled={save.isPending}
              >
                ذخیره
              </DashboardButton>
              {form.name && onOpenLedger ? (
                <DashboardButton
                  type="button"
                  variant="ghost"
                  size="md"
                  onClick={() => {
                    setOpen(false)
                    onOpenLedger(form.name)
                  }}
                >
                  دفتر
                </DashboardButton>
              ) : null}
            </div>
          </form>
        </SheetContent>
      </Sheet>
    </div>
  )
}

function kindFromFa(label: string): WalletInput['kind'] {
  const hit = KINDS.find(([, fa]) => fa === label)
  return hit?.[0] ?? 'cash'
}

function Field({
  label,
  children,
}: {
  label: string
  children: React.ReactNode
}) {
  return (
    <div className="space-y-1">
      <Label className="text-[12px] text-[var(--ink-muted)]">{label}</Label>
      {children}
    </div>
  )
}
