import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
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
import { Textarea } from '@/components/ui/textarea'
import { formatDate, formatMoney } from '@/lib/format'
import { ipc, type Settings as SettingsModel } from '@/lib/ipc'
import { useSettings } from '@/lib/settings'

type Tab = 'profile' | 'money' | 'calendar' | 'appearance' | 'data'

export function SettingsScreen() {
  const qc = useQueryClient()
  const { settings, ready, applyLocal, refresh, unit, persianDigits, dateStyle } =
    useSettings()
  const [tab, setTab] = useState<Tab>('profile')
  const [form, setForm] = useState<SettingsModel | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  useEffect(() => {
    if (settings) setForm((prev) => prev ?? { ...settings })
  }, [settings])

  useEffect(() => {
    if (!form) return
    applyLocal(form)
  }, [
    form?.appearance,
    form?.density,
    form?.text_direction,
    form?.accent,
    form?.default_currency,
    form?.display_digits,
    form?.date_display,
    form?.app_mode,
    applyLocal,
    form,
  ])

  const save = useMutation({
    mutationFn: () => {
      if (!form) throw new Error('فرم خالی است')
      return ipc.updateSettings(form)
    },
    onSuccess: async (saved) => {
      setMessage('ذخیره شد')
      setForm({ ...saved })
      applyLocal(saved)
      await refresh()
      await qc.invalidateQueries({ queryKey: ['dashboard'] })
      await qc.invalidateQueries({ queryKey: ['ledger'] })
      await qc.invalidateQueries({ queryKey: ['accounts'] })
    },
    onError: (e: Error) => setMessage(e.message),
  })

  const reset = () => {
    if (settings) setForm({ ...settings })
    setMessage(null)
  }

  if (!ready || !form) {
    return <div className="sarv-page text-[var(--ink-muted)]">…</div>
  }

  const previewAmount = formatMoney(1_250_000, unit, persianDigits)
  const previewDate = formatDate(
    new Date().toISOString().slice(0, 10),
    dateStyle,
    persianDigits,
  )

  const tabs: { id: Tab; label: string }[] = [
    { id: 'profile', label: 'پروفایل' },
    { id: 'money', label: 'پول' },
    { id: 'calendar', label: 'تقویم' },
    { id: 'appearance', label: 'ظاهر' },
    { id: 'data', label: 'داده' },
  ]

  return (
    <div className="sarv-page mx-auto max-w-3xl space-y-4">
      <h1 className="text-[18px] font-semibold">تنظیمات و بومی‌سازی</h1>

      <div className="grid gap-4 md:grid-cols-[180px_1fr]">
        <nav className="sarv-panel flex flex-row overflow-auto p-1 md:flex-col">
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => setTab(t.id)}
              className={`min-h-11 rounded-[8px] px-3 text-start text-[13px] ${
                tab === t.id
                  ? 'bg-[var(--cta)] text-[var(--primary-foreground)]'
                  : 'text-[var(--ink-soft)]'
              }`}
            >
              {t.label}
            </button>
          ))}
        </nav>

        <div className="space-y-3">
          {tab === 'profile' ? (
            <section className="sarv-panel space-y-3 p-4">
              <Field label="نام (شخصی یا شرکت)">
                <Input
                  value={form.business_name}
                  onChange={(e) =>
                    setForm({ ...form, business_name: e.target.value })
                  }
                />
              </Field>
              <div className="grid grid-cols-2 gap-2">
                <Field label="تلفن">
                  <Input
                    dir="ltr"
                    value={form.business_phone}
                    onChange={(e) =>
                      setForm({ ...form, business_phone: e.target.value })
                    }
                  />
                </Field>
                <Field label="حالت">
                  <Select
                    value={form.app_mode}
                    onValueChange={(v) =>
                      setForm({
                        ...form,
                        app_mode: v as SettingsModel['app_mode'],
                      })
                    }
                  >
                    <SelectTrigger className="h-10">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Personal">شخصی</SelectItem>
                      <SelectItem value="Business">کسب‌وکار</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </div>
              <Field label="نشانی">
                <Input
                  value={form.business_address}
                  onChange={(e) =>
                    setForm({ ...form, business_address: e.target.value })
                  }
                />
              </Field>
              <Field label="سربرگ">
                <Input
                  value={form.header}
                  onChange={(e) => setForm({ ...form, header: e.target.value })}
                />
              </Field>
              <Field label="پاورقی">
                <Input
                  value={form.footer}
                  onChange={(e) => setForm({ ...form, footer: e.target.value })}
                />
              </Field>
              <Field label="شرایط پیش‌فرض">
                <Textarea
                  value={form.default_terms}
                  onChange={(e) =>
                    setForm({ ...form, default_terms: e.target.value })
                  }
                />
              </Field>
              <Field label="ماه شروع سال مالی (۱–۱۲)">
                <Input
                  type="number"
                  min={1}
                  max={12}
                  dir="ltr"
                  value={form.fiscal_year_start_month}
                  onChange={(e) =>
                    setForm({
                      ...form,
                      fiscal_year_start_month: Number(e.target.value) || 1,
                    })
                  }
                />
              </Field>
            </section>
          ) : null}

          {tab === 'money' ? (
            <section className="sarv-panel space-y-3 p-4">
              <Field label="واحد پایه">
                <Select
                  value={form.default_currency}
                  onValueChange={(v) =>
                    setForm({
                      ...form,
                      default_currency: v as SettingsModel['default_currency'],
                    })
                  }
                >
                  <SelectTrigger className="h-10">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Toman">تومان (۱ رقم اعشار)</SelectItem>
                    <SelectItem value="Rial">ریال (بدون اعشار)</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field label="نرخ مالیات (صدم درصد / basis points)">
                <Input
                  type="number"
                  dir="ltr"
                  value={form.tax_basis_points}
                  onChange={(e) =>
                    setForm({
                      ...form,
                      tax_basis_points: Number(e.target.value) || 0,
                    })
                  }
                />
              </Field>
              <Field label="پیشوند شماره سند">
                <Input
                  dir="ltr"
                  value={form.quote_prefix}
                  onChange={(e) =>
                    setForm({ ...form, quote_prefix: e.target.value })
                  }
                />
              </Field>
            </section>
          ) : null}

          {tab === 'calendar' ? (
            <section className="sarv-panel space-y-3 p-4">
              <Field label="نمایش تاریخ">
                <Select
                  value={form.date_display}
                  onValueChange={(v) =>
                    setForm({
                      ...form,
                      date_display: v as SettingsModel['date_display'],
                    })
                  }
                >
                  <SelectTrigger className="h-10">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Iso">میلادی ISO</SelectItem>
                    <SelectItem value="DayMonthYear">روز/ماه/سال</SelectItem>
                    <SelectItem value="Jalali">جلالی</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field label="ارقام">
                <Select
                  value={form.display_digits}
                  onValueChange={(v) =>
                    setForm({
                      ...form,
                      display_digits: v as SettingsModel['display_digits'],
                    })
                  }
                >
                  <SelectTrigger className="h-10">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Persian">فارسی</SelectItem>
                    <SelectItem value="Latin">لاتین</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field label="جهت متن">
                <Select
                  value={form.text_direction}
                  onValueChange={(v) =>
                    setForm({
                      ...form,
                      text_direction: v as SettingsModel['text_direction'],
                    })
                  }
                >
                  <SelectTrigger className="h-10">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Rtl">راست‌به‌چپ</SelectItem>
                    <SelectItem value="Ltr">چپ‌به‌راست</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </section>
          ) : null}

          {tab === 'appearance' ? (
            <section className="sarv-panel space-y-3 p-4">
              <Field label="پوسته">
                <Select
                  value={form.appearance}
                  onValueChange={(v) =>
                    setForm({
                      ...form,
                      appearance: v as SettingsModel['appearance'],
                    })
                  }
                >
                  <SelectTrigger className="h-10">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="System">سیستم</SelectItem>
                    <SelectItem value="Light">روشن</SelectItem>
                    <SelectItem value="Dark">تیره</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field label="تراکم">
                <Select
                  value={form.density}
                  onValueChange={(v) =>
                    setForm({
                      ...form,
                      density: v as SettingsModel['density'],
                    })
                  }
                >
                  <SelectTrigger className="h-10">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Comfortable">راحت</SelectItem>
                    <SelectItem value="Compact">فشرده</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field label="رنگ تأکید (CSS)">
                <Input
                  dir="ltr"
                  placeholder="#272523"
                  value={form.accent}
                  onChange={(e) => setForm({ ...form, accent: e.target.value })}
                />
              </Field>
            </section>
          ) : null}

          {tab === 'data' ? (
            <section className="sarv-panel space-y-2 p-4 text-[13px] text-[var(--ink-soft)]">
              <p>
                داده روی دستگاه شماست (SQLite). پشتیبان‌گیری و ورود از نرم‌افزارهای
                دیگر در فاز ۲ روی همین دفتر اضافه می‌شود.
              </p>
              <p>
                همهٔ ماژول‌های بعدی روی همان خطوط دفتر دو طرفه ثبت می‌شوند؛ کار فاز
                ۱ دور ریخته نمی‌شود.
              </p>
            </section>
          ) : null}

          <section className="sarv-panel space-y-2 p-4">
            <h2 className="text-[13px] font-semibold">پیش‌نمایش زنده</h2>
            <p className="text-[14px]">{previewAmount}</p>
            <p className="text-[13px] text-[var(--ink-muted)]">{previewDate}</p>
            <p className="text-[12px] text-[var(--ink-soft)]">
              {form.header || form.business_name || 'سربرگ نمونه'}
            </p>
          </section>

          <footer className="flex flex-wrap items-center gap-3">
            <DashboardButton
              variant="primary"
              size="md"
              onClick={() => save.mutate()}
              disabled={save.isPending}
            >
              ذخیره
            </DashboardButton>
            <DashboardButton variant="ghost" size="md" onClick={reset}>
              بازنشانی
            </DashboardButton>
            {message ? (
              <span className="text-[12px] text-[var(--ink-muted)]">{message}</span>
            ) : null}
          </footer>
        </div>
      </div>
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
