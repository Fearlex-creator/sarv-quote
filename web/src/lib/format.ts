const persianDigits = '۰۱۲۳۴۵۶۷۸۹'
const latinDigits = '0123456789'

export type MoneyUnit = 'rial' | 'toman'
export type DateStyle = 'iso' | 'dmy' | 'jalali'

export function faDigits(value: string | number, persian = true): string {
  const s = String(value)
  if (!persian) return s
  return s.replace(/\d/g, (d) => persianDigits[Number(d)] ?? d)
}

export function toLatinDigits(value: string): string {
  return value.replace(/[۰-۹]/g, (d) => latinDigits[persianDigits.indexOf(d)] ?? d)
}

/** Approximate Jalali from Gregorian ISO date (YYYY-MM-DD). */
export function toJalali(iso: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso)
  if (!m) return iso
  let gy = Number(m[1])
  const gm = Number(m[2])
  const gd = Number(m[3])
  const g_d_m = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]
  let jy: number
  if (gy > 1600) {
    jy = 979
    gy -= 1600
  } else {
    jy = 0
    gy -= 621
  }
  const gy2 = gm > 2 ? gy + 1 : gy
  let days =
    365 * gy +
    Math.floor((gy2 + 3) / 4) -
    Math.floor((gy2 + 99) / 100) +
    Math.floor((gy2 + 399) / 400) -
    80 +
    gd +
    g_d_m[gm - 1]!
  jy += 33 * Math.floor(days / 12053)
  days %= 12053
  jy += 4 * Math.floor(days / 1461)
  days %= 1461
  if (days > 365) {
    jy += Math.floor((days - 1) / 365)
    days = (days - 1) % 365
  }
  const jm =
    days < 186 ? 1 + Math.floor(days / 31) : 7 + Math.floor((days - 186) / 30)
  const jd = 1 + (days < 186 ? days % 31 : (days - 186) % 30)
  return `${String(jy).padStart(4, '0')}/${String(jm).padStart(2, '0')}/${String(jd).padStart(2, '0')}`
}

export function formatDate(
  iso: string,
  style: DateStyle = 'iso',
  persian = true,
): string {
  const prefix = iso.slice(0, 10)
  let shown = prefix
  if (style === 'jalali') shown = toJalali(prefix)
  else if (style === 'dmy') {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(prefix)
    shown = m ? `${m[3]}/${m[2]}/${m[1]}` : prefix
  }
  return faDigits(shown, persian)
}

export function formatMoney(
  rial: number,
  unit: MoneyUnit = 'toman',
  persian = true,
): string {
  const amount = unit === 'toman' ? Math.trunc(rial / 10) : rial
  const grouped = Math.abs(amount)
    .toString()
    .replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  const sign = amount < 0 ? '−' : ''
  const label = unit === 'toman' ? 'تومان' : 'ریال'
  return `${sign}${faDigits(grouped, persian)} ${label}`
}

/** Parse user amount in display unit → rial storage. */
export function parseAmount(
  raw: string,
  unit: MoneyUnit = 'toman',
): number | null {
  const normalized = toLatinDigits(raw)
    .replace(/[٬,]/g, '')
    .replace(/[^\d.]/g, '')
    .trim()
  if (!normalized) return null
  const n = Number(normalized)
  if (!Number.isFinite(n) || n <= 0) return null
  return unit === 'toman' ? Math.round(n * 10) : Math.round(n)
}

export function cnMoneyTone(rial: number): string {
  if (rial > 0) return 'text-emerald-700 dark:text-emerald-400'
  if (rial < 0) return 'text-rose-700 dark:text-rose-400'
  return 'text-[var(--ink-muted)]'
}
