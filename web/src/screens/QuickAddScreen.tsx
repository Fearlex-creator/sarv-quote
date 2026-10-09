import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useMemo, useState } from 'react'
import { DashboardButton } from '@/components/block/dashboard-shell'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { parseAmount } from '@/lib/format'
import { ipc, type CashEntryInput, type CashSplit } from '@/lib/ipc'
import { useSettings } from '@/lib/settings'

type Kind = CashEntryInput['kind']

export function QuickAddScreen({
  onDone,
  embedded,
}: {
  onDone?: () => void
  embedded?: boolean
}) {
  const { unit, isBusiness } = useSettings()
  const qc = useQueryClient()
  const suggest = useQuery({
    queryKey: ['suggestions'],
    queryFn: () => ipc.getSuggestions(),
  })

  const [kind, setKind] = useState<Kind>('expense')
  const [amount, setAmount] = useState('')
  const [payee, setPayee] = useState('')
  const [category, setCategory] = useState('')
  const [account, setAccount] = useState('صندوق')
  const [toAccount, setToAccount] = useState('بانک')
  const [memo, setMemo] = useState('')
  const [date, setDate] = useState(() => new Date().toISOString().slice(0, 10))
  const [attachmentName, setAttachmentName] = useState('')
  const [splitOn, setSplitOn] = useState(false)
  const [split2Cat, setSplit2Cat] = useState('')
  const [split2Amt, setSplit2Amt] = useState('')
  const [status, setStatus] = useState<string | null>(null)
  const [addAnother, setAddAnother] = useState(false)

  useEffect(() => {
    const s = suggest.data
    if (!s) return
    if (s.lastKind === 'income' || s.lastKind === 'transfer' || s.lastKind === 'expense') {
      setKind(s.lastKind)
    }
    if (s.lastAccount) setAccount(s.lastAccount)
    if (s.lastToAccount) setToAccount(s.lastToAccount)
  }, [suggest.data])

  useEffect(() => {
    const map = suggest.data?.payeeCategory
    if (!map || !payee.trim()) return
    const hit = map[payee.trim()]
    if (hit) setCategory(hit)
  }, [payee, suggest.data?.payeeCategory])

  const categoryOptions = useMemo(
    () => suggest.data?.categories ?? [],
    [suggest.data],
  )

  const resetSoft = () => {
    setAmount('')
    setMemo('')
    setAttachmentName('')
    setSplitOn(false)
    setSplit2Cat('')
    setSplit2Amt('')
  }

  const save = useMutation({
    mutationFn: async () => {
      const amountRial = parseAmount(amount, unit)
      if (amountRial == null) throw new Error('مبلغ معتبر وارد کنید')
      let splits: CashSplit[] = []
      if (splitOn && kind !== 'transfer') {
        const part = parseAmount(split2Amt, unit)
        if (!part || !split2Cat.trim()) {
          throw new Error('تفکیک ناقص است')
        }
        if (part >= amountRial) throw new Error('مبلغ تفکیک باید کمتر از کل باشد')
        splits = [
          { category: category.trim() || 'سایر', amountRial: amountRial - part },
          { category: split2Cat.trim(), amountRial: part },
        ]
      }
      return ipc.addCashEntry({
        kind,
        amountRial,
        payee:
          payee.trim() ||
          (kind === 'transfer' ? `${account} → ${toAccount}` : 'نامشخص'),
        category: category.trim(),
        account,
        toAccount: kind === 'transfer' ? toAccount : '',
        memo: memo.trim(),
        date,
        splits,
        attachmentName,
      })
    },
    onSuccess: async () => {
      setStatus('ثبت شد')
      await Promise.all([
        qc.invalidateQueries({ queryKey: ['dashboard'] }),
        qc.invalidateQueries({ queryKey: ['ledger'] }),
        qc.invalidateQueries({ queryKey: ['accounts'] }),
        qc.invalidateQueries({ queryKey: ['contacts'] }),
        qc.invalidateQueries({ queryKey: ['suggestions'] }),
      ])
      if (addAnother) {
        resetSoft()
        setAddAnother(false)
      } else {
        resetSoft()
        onDone?.()
      }
    },
    onError: (e: Error) => setStatus(e.message),
  })

  const unitLabel = unit === 'rial' ? 'ریال' : 'تومان'

  return (
    <div className={embedded ? 'space-y-3' : 'sarv-page mx-auto max-w-md space-y-3'}>
      {!embedded ? <h1 className="text-[18px] font-semibold">ثبت سریع</h1> : null}

      <div className="flex gap-1 rounded-[10px] bg-[var(--muted)] p-1">
        {(
          [
            ['expense', 'هزینه'],
            ['income', 'درآمد'],
            ['transfer', 'انتقال'],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            onClick={() => setKind(id)}
            className={`min-h-10 flex-1 rounded-[8px] px-2 text-[13px] font-medium ${
              kind === id
                ? 'bg-[var(--surface)] text-[var(--ink)] shadow-sm'
                : 'text-[var(--ink-muted)]'
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <form
        className="sarv-panel space-y-3 p-4"
        onSubmit={(e) => {
          e.preventDefault()
          setStatus(null)
          setAddAnother(false)
          save.mutate()
        }}
      >
        <Field label={`مبلغ (${unitLabel})`}>
          <Input
            autoFocus
            dir="ltr"
            inputMode="decimal"
            className="h-11 text-[18px] font-semibold tabular-nums"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
          />
        </Field>

        <Field label="تاریخ">
          <Input
            type="date"
            dir="ltr"
            className="h-10"
            value={date}
            onChange={(e) => setDate(e.target.value)}
          />
        </Field>

        {kind !== 'transfer' ? (
          <Field label="طرف حساب">
            <Input
              list="sarv-payees"
              value={payee}
              onChange={(e) => setPayee(e.target.value)}
              autoComplete="off"
            />
            <datalist id="sarv-payees">
              {(suggest.data?.payees ?? []).map((p) => (
                <option key={p} value={p} />
              ))}
            </datalist>
          </Field>
        ) : null}

        {kind !== 'transfer' ? (
          <Field label="دسته‌بندی">
            <Input
              list="sarv-categories"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              autoComplete="off"
            />
            <datalist id="sarv-categories">
              {categoryOptions.map((c) => (
                <option key={c} value={c} />
              ))}
            </datalist>
          </Field>
        ) : null}

        <div className="grid grid-cols-2 gap-2">
          <Field label={kind === 'transfer' ? 'از حساب' : 'حساب'}>
            <Input
              list="sarv-accounts"
              value={account}
              onChange={(e) => setAccount(e.target.value)}
              autoComplete="off"
            />
          </Field>
          {kind === 'transfer' ? (
            <Field label="به حساب">
              <Input
                list="sarv-accounts"
                value={toAccount}
                onChange={(e) => setToAccount(e.target.value)}
                autoComplete="off"
              />
            </Field>
          ) : (
            <Field label="یادداشت">
              <Input value={memo} onChange={(e) => setMemo(e.target.value)} />
            </Field>
          )}
        </div>
        <datalist id="sarv-accounts">
          {(suggest.data?.accounts ?? []).map((a) => (
            <option key={a} value={a} />
          ))}
        </datalist>

        {kind === 'transfer' ? (
          <Field label="یادداشت">
            <Input value={memo} onChange={(e) => setMemo(e.target.value)} />
          </Field>
        ) : null}

        {kind !== 'transfer' ? (
          <label className="flex min-h-10 items-center gap-2 text-[13px]">
            <input
              type="checkbox"
              checked={splitOn}
              onChange={(e) => setSplitOn(e.target.checked)}
            />
            تفکیک بین دو دسته
          </label>
        ) : null}

        {splitOn && kind !== 'transfer' ? (
          <div className="grid grid-cols-2 gap-2">
            <Field label="دسته دوم">
              <Input value={split2Cat} onChange={(e) => setSplit2Cat(e.target.value)} />
            </Field>
            <Field label={`مبلغ دسته دوم (${unitLabel})`}>
              <Input
                dir="ltr"
                value={split2Amt}
                onChange={(e) => setSplit2Amt(e.target.value)}
              />
            </Field>
          </div>
        ) : null}

        <Field label="پیوست (نام فایل)">
          <Input
            value={attachmentName}
            onChange={(e) => setAttachmentName(e.target.value)}
            placeholder="receipt.jpg"
            dir="ltr"
          />
        </Field>

        {isBusiness ? (
          <p className="text-[11px] text-[var(--ink-muted)]">
            حالت کسب‌وکار: پس از ثبت، بدهکار/بستانکار در جزئیات دفتر دیده می‌شود.
          </p>
        ) : null}

        <div className="flex flex-col gap-2 sm:flex-row">
          <DashboardButton
            type="submit"
            variant="primary"
            size="md"
            className="min-h-11 flex-1"
            disabled={save.isPending}
          >
            {save.isPending ? '…' : 'ذخیره'}
          </DashboardButton>
          <DashboardButton
            type="button"
            variant="ghost"
            size="md"
            className="min-h-11 flex-1"
            disabled={save.isPending}
            onClick={() => {
              setStatus(null)
              setAddAnother(true)
              save.mutate()
            }}
          >
            ذخیره و بعدی
          </DashboardButton>
        </div>
        {status ? (
          <p className="text-center text-[12px] text-[var(--ink-muted)]">{status}</p>
        ) : null}
      </form>
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
