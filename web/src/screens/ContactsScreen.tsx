import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { DashboardButton } from '@/components/block/dashboard-shell'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { formatMoney } from '@/lib/format'
import { ipc, type ContactInput, type ContactRow } from '@/lib/ipc'
import { useSettings } from '@/lib/settings'

const emptyForm: ContactInput = {
  name: '',
  phone: '',
  email: '',
  address: '',
  contactName: '',
  isCustomer: true,
  isSupplier: false,
  taxId: '',
  notes: '',
  archived: false,
}

export function ContactsScreen({
  onOpenHistory,
}: {
  onOpenHistory?: (payee: string) => void
}) {
  const { unit, persianDigits } = useSettings()
  const qc = useQueryClient()
  const [open, setOpen] = useState(false)
  const [q, setQ] = useState('')
  const [role, setRole] = useState<'all' | 'customer' | 'supplier'>('all')
  const [form, setForm] = useState<ContactInput>(emptyForm)
  const [selected, setSelected] = useState<ContactRow | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [importText, setImportText] = useState('')

  const { data = [], isLoading, isError, error: loadError } = useQuery({
    queryKey: ['contacts'],
    queryFn: () => ipc.listContacts(),
  })

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase()
    return data.filter((c) => {
      if (c.archived) return false
      if (role === 'customer' && !c.isCustomer) return false
      if (role === 'supplier' && !c.isSupplier) return false
      if (!needle) return true
      return [c.name, c.phone, c.email, c.contactName, c.taxId, ...c.roles]
        .join(' ')
        .toLowerCase()
        .includes(needle)
    })
  }, [data, q, role])

  const save = useMutation({
    mutationFn: () => ipc.saveContact(form),
    onSuccess: async (row) => {
      setOpen(false)
      setSelected(row)
      setError(null)
      await qc.invalidateQueries({ queryKey: ['contacts'] })
      await qc.invalidateQueries({ queryKey: ['dashboard'] })
      await qc.invalidateQueries({ queryKey: ['suggestions'] })
    },
    onError: (e: Error) => setError(e.message),
  })

  const importCsv = useMutation({
    mutationFn: async () => {
      const lines = importText
        .split(/\r?\n/)
        .map((l) => l.trim())
        .filter(Boolean)
      for (const line of lines) {
        if (line.toLowerCase().startsWith('name')) continue
        const [name, phone = '', email = ''] = line
          .split(',')
          .map((p) => p.replace(/^"|"$/g, '').trim())
        if (!name) continue
        await ipc.saveContact({
          name,
          phone,
          email,
          isCustomer: true,
          isSupplier: false,
        })
      }
    },
    onSuccess: async () => {
      setImportText('')
      await qc.invalidateQueries({ queryKey: ['contacts'] })
      await qc.invalidateQueries({ queryKey: ['dashboard'] })
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
    <div className="flex h-full min-h-0 flex-col gap-3 p-3 md:p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-[18px] font-semibold">مخاطبین</h1>
        <DashboardButton
          variant="primary"
          size="sm"
          onClick={() => {
            setForm(emptyForm)
            setOpen(true)
          }}
        >
          <Plus className="size-3.5" />
          تازه
        </DashboardButton>
      </div>

      <div className="flex flex-wrap gap-2">
        <Input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="جستجو…"
          className="h-10 flex-1"
        />
        {(
          [
            ['all', 'همه'],
            ['customer', 'مشتری'],
            ['supplier', 'تأمین‌کننده'],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            onClick={() => setRole(id)}
            className={`min-h-10 rounded-[8px] px-3 text-[12px] ${
              role === id
                ? 'bg-[var(--cta)] text-[var(--primary-foreground)]'
                : 'bg-[var(--surface)] ring-1 ring-[var(--line)]'
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="grid min-h-0 flex-1 gap-3 lg:grid-cols-[1fr_320px]">
        <div className="sarv-panel min-h-0 overflow-auto">
          {filtered.length === 0 ? (
            <p className="px-4 py-10 text-center text-[13px] text-[var(--ink-muted)]">
              مخاطبی نیست
            </p>
          ) : (
            filtered.map((c) => (
              <button
                key={c.id}
                type="button"
                className="sarv-row grid-cols-[1fr_auto] w-full text-right"
                onClick={() => setSelected(c)}
              >
                <div className="min-w-0">
                  <p className="truncate text-[14px] font-medium">{c.name}</p>
                  <p className="truncate text-[12px] text-[var(--ink-muted)]">
                    {c.roles.join(' · ')}
                    {c.phone ? ` · ${c.phone}` : ''}
                  </p>
                </div>
                <span className="tabular-nums text-[13px] font-medium">
                  {money(c.balanceRial)}
                </span>
              </button>
            ))
          )}
        </div>

        <aside className="sarv-panel space-y-3 overflow-auto p-4">
          {selected ? (
            <>
              <div>
                <h2 className="text-[16px] font-semibold">{selected.name}</h2>
                <p className="text-[12px] text-[var(--ink-muted)]">
                  {selected.roles.join(' · ')}
                </p>
              </div>
              <p className="text-[14px] font-medium">
                مانده: {money(selected.balanceRial)}
              </p>
              <div className="grid grid-cols-2 gap-2">
                {selected.aging.map((b) => (
                  <div
                    key={b.label}
                    className="rounded-[8px] bg-[var(--canvas)] p-2 text-[12px]"
                  >
                    <div className="text-[var(--ink-muted)]">{b.label}</div>
                    <div className="font-medium tabular-nums">
                      {money(b.amountRial)}
                    </div>
                  </div>
                ))}
              </div>
              <dl className="space-y-1 text-[12px]">
                <div className="flex justify-between gap-2">
                  <dt className="text-[var(--ink-muted)]">تلفن</dt>
                  <dd dir="ltr">{selected.phone || '—'}</dd>
                </div>
                <div className="flex justify-between gap-2">
                  <dt className="text-[var(--ink-muted)]">ایمیل</dt>
                  <dd dir="ltr">{selected.email || '—'}</dd>
                </div>
                <div className="flex justify-between gap-2">
                  <dt className="text-[var(--ink-muted)]">کد اقتصادی</dt>
                  <dd dir="ltr">{selected.taxId || '—'}</dd>
                </div>
              </dl>
              {selected.notes ? (
                <p className="text-[12px] text-[var(--ink-soft)]">{selected.notes}</p>
              ) : null}
              <div className="flex flex-col gap-2">
                <DashboardButton
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setForm({
                      id: selected.id,
                      name: selected.name,
                      phone: selected.phone,
                      email: selected.email,
                      address: selected.address,
                      contactName: selected.contactName,
                      archived: selected.archived,
                      isCustomer: selected.isCustomer,
                      isSupplier: selected.isSupplier,
                      taxId: selected.taxId,
                      notes: selected.notes,
                    })
                    setOpen(true)
                  }}
                >
                  ویرایش
                </DashboardButton>
                <DashboardButton
                  variant="ghost"
                  size="sm"
                  onClick={() => onOpenHistory?.(selected.name)}
                >
                  تاریخچه در دفتر
                </DashboardButton>
                <p className="text-[11px] text-[var(--ink-muted)]">
                  صورت‌حساب PDF و اسناد فاز ۲ در همین پروفایل جای می‌گیرند.
                </p>
              </div>
            </>
          ) : (
            <p className="text-[13px] text-[var(--ink-muted)]">
              یک مخاطب را انتخاب کنید
            </p>
          )}

          <div className="border-t border-[var(--line)] pt-3">
            <p className="mb-2 text-[12px] font-semibold">ورود CSV</p>
            <Textarea
              value={importText}
              onChange={(e) => setImportText(e.target.value)}
              placeholder={'name,phone,email\nعلی,0912...,a@b.c'}
              className="min-h-24 font-mono text-[12px]"
              dir="ltr"
            />
            <DashboardButton
              className="mt-2 w-full"
              size="sm"
              variant="ghost"
              disabled={!importText.trim() || importCsv.isPending}
              onClick={() => importCsv.mutate()}
            >
              وارد کردن
            </DashboardButton>
          </div>
        </aside>
      </div>

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent side="left" className="w-full sm:max-w-md">
          <SheetHeader>
            <SheetTitle>{form.id ? 'ویرایش مخاطب' : 'مخاطب تازه'}</SheetTitle>
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
                autoFocus
                value={form.name}
                onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
              />
            </Field>
            <div className="flex gap-4 text-[13px]">
              <label className="flex min-h-10 items-center gap-2">
                <input
                  type="checkbox"
                  checked={!!form.isCustomer}
                  onChange={(e) =>
                    setForm((f) => ({ ...f, isCustomer: e.target.checked }))
                  }
                />
                مشتری
              </label>
              <label className="flex min-h-10 items-center gap-2">
                <input
                  type="checkbox"
                  checked={!!form.isSupplier}
                  onChange={(e) =>
                    setForm((f) => ({ ...f, isSupplier: e.target.checked }))
                  }
                />
                تأمین‌کننده
              </label>
            </div>
            <Field label="تلفن">
              <Input
                dir="ltr"
                value={form.phone ?? ''}
                onChange={(e) => setForm((f) => ({ ...f, phone: e.target.value }))}
              />
            </Field>
            <Field label="ایمیل">
              <Input
                dir="ltr"
                value={form.email ?? ''}
                onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))}
              />
            </Field>
            <Field label="کد اقتصادی / مالیاتی">
              <Input
                dir="ltr"
                value={form.taxId ?? ''}
                onChange={(e) => setForm((f) => ({ ...f, taxId: e.target.value }))}
              />
            </Field>
            <Field label="نشانی">
              <Input
                value={form.address ?? ''}
                onChange={(e) =>
                  setForm((f) => ({ ...f, address: e.target.value }))
                }
              />
            </Field>
            <Field label="یادداشت">
              <Textarea
                value={form.notes ?? ''}
                onChange={(e) => setForm((f) => ({ ...f, notes: e.target.value }))}
              />
            </Field>
            {form.id ? (
              <label className="flex min-h-10 items-center gap-2 text-[13px]">
                <input
                  type="checkbox"
                  checked={!!form.archived}
                  onChange={(e) =>
                    setForm((f) => ({ ...f, archived: e.target.checked }))
                  }
                />
                بایگانی
              </label>
            ) : null}
            {error ? (
              <p className="text-[12px] text-[var(--destructive)]">{error}</p>
            ) : null}
            <DashboardButton
              type="submit"
              variant="primary"
              size="md"
              className="w-full"
              disabled={save.isPending}
            >
              ذخیره
            </DashboardButton>
          </form>
        </SheetContent>
      </Sheet>
    </div>
  )
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
