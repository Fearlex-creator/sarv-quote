import { useEffect, useMemo, useState } from 'react'
import { SCREENS, type ScreenId } from '@/lib/navigation'

export function CommandPalette({
  open,
  onOpenChange,
  onNavigate,
  onQuickAdd,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  onNavigate: (id: ScreenId) => void
  onQuickAdd: () => void
}) {
  const [q, setQ] = useState('')

  useEffect(() => {
    if (!open) setQ('')
  }, [open])

  const items = useMemo(() => {
    const actions = [
      {
        id: 'action-quick-add',
        label: 'ثبت سریع هزینه / درآمد / انتقال',
        hint: 'Ctrl+N',
        run: onQuickAdd,
      },
      ...SCREENS.map((s) => ({
        id: s.id,
        label: s.title,
        hint: s.description,
        run: () => onNavigate(s.id),
      })),
    ]
    const needle = q.trim().toLowerCase()
    if (!needle) return actions
    return actions.filter(
      (a) =>
        a.label.toLowerCase().includes(needle) ||
        a.hint.toLowerCase().includes(needle),
    )
  }, [q, onNavigate, onQuickAdd])

  if (!open) return null

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/35 p-4 pt-[12vh]"
      role="dialog"
      aria-modal="true"
      aria-label="پالت فرمان"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onOpenChange(false)
      }}
    >
      <div className="w-full max-w-lg overflow-hidden rounded-[12px] border border-[var(--line)] bg-[var(--surface)] shadow-xl">
        <input
          autoFocus
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') onOpenChange(false)
            if (e.key === 'Enter' && items[0]) {
              items[0].run()
              onOpenChange(false)
            }
          }}
          placeholder="جستجو یا فرمان…"
          className="h-12 w-full border-b border-[var(--line)] bg-transparent px-4 text-[14px] outline-none"
        />
        <ul className="max-h-72 overflow-auto p-1">
          {items.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                className="flex w-full items-center justify-between gap-3 rounded-[8px] px-3 py-2.5 text-right hover:bg-[var(--canvas)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--focus)]"
                onClick={() => {
                  item.run()
                  onOpenChange(false)
                }}
              >
                <span className="text-[13px] font-medium">{item.label}</span>
                <span className="text-[11px] text-[var(--ink-muted)]">
                  {item.hint}
                </span>
              </button>
            </li>
          ))}
          {items.length === 0 ? (
            <li className="px-3 py-6 text-center text-[12px] text-[var(--ink-muted)]">
              نتیجه‌ای نیست
            </li>
          ) : null}
        </ul>
      </div>
    </div>
  )
}
