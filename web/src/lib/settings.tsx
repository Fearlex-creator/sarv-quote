import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import type { DateStyle, MoneyUnit } from '@/lib/format'
import { ipc, type Settings, type SettingsView } from '@/lib/ipc'

type SettingsCtx = {
  ready: boolean
  error: string | null
  view: SettingsView | null
  settings: Settings | null
  unit: MoneyUnit
  persianDigits: boolean
  dateStyle: DateStyle
  appMode: 'personal' | 'business'
  isBusiness: boolean
  refresh: () => Promise<void>
  applyLocal: (next: Settings) => void
}

const Ctx = createContext<SettingsCtx | null>(null)

function applyDocument(settings: Settings) {
  const dark =
    settings.appearance === 'Dark' ||
    (settings.appearance === 'System' &&
      window.matchMedia('(prefers-color-scheme: dark)').matches)
  document.documentElement.classList.toggle('dark', dark)
  document.documentElement.dataset.density = settings.density.toLowerCase()
  document.documentElement.dir =
    settings.text_direction === 'Ltr' ? 'ltr' : 'rtl'
  if (settings.accent.trim()) {
    document.documentElement.style.setProperty('--cta', settings.accent.trim())
  }
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<SettingsView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [ready, setReady] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const next = await ipc.getSettingsView()
      setView(next)
      applyDocument(next.settings)
      setError(null)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setReady(true)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const applyLocal = useCallback((next: Settings) => {
    setView((prev) =>
      prev
        ? {
            ...prev,
            settings: next,
            currency: next.default_currency === 'Rial' ? 'rial' : 'toman',
            dateDisplay:
              next.date_display === 'Jalali'
                ? 'jalali'
                : next.date_display === 'DayMonthYear'
                  ? 'dmy'
                  : 'iso',
            appearance: next.appearance.toLowerCase(),
            digits: next.display_digits === 'Latin' ? 'latin' : 'persian',
            appMode: next.app_mode === 'Business' ? 'business' : 'personal',
            textDirection: next.text_direction === 'Ltr' ? 'ltr' : 'rtl',
          }
        : prev,
    )
    applyDocument(next)
  }, [])

  const settings = view?.settings ?? null
  const value = useMemo<SettingsCtx>(
    () => ({
      ready,
      error,
      view,
      settings,
      unit: settings?.default_currency === 'Rial' ? 'rial' : 'toman',
      persianDigits: settings?.display_digits !== 'Latin',
      dateStyle:
        settings?.date_display === 'Jalali'
          ? 'jalali'
          : settings?.date_display === 'DayMonthYear'
            ? 'dmy'
            : 'iso',
      appMode: settings?.app_mode === 'Business' ? 'business' : 'personal',
      isBusiness: settings?.app_mode === 'Business',
      refresh,
      applyLocal,
    }),
    [ready, error, view, settings, refresh, applyLocal],
  )

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useSettings() {
  const ctx = useContext(Ctx)
  if (!ctx) throw new Error('useSettings outside SettingsProvider')
  return ctx
}
