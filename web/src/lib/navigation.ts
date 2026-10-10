import type { LucideIcon } from 'lucide-react'
import {
  Archive,
  ArrowLeftRight,
  Banknote,
  BookOpen,
  Boxes,
  Calculator,
  CalendarClock,
  Camera,
  CheckSquare,
  ClipboardList,
  FileSpreadsheet,
  FileText,
  Globe2,
  Landmark,
  LayoutDashboard,
  ListTree,
  Package,
  Percent,
  Receipt,
  RefreshCw,
  Search,
  Settings,
  Shield,
  ShoppingBag,
  ShoppingCart,
  Tags,
  Users,
  Wallet,
  Warehouse,
  Workflow,
} from 'lucide-react'

export type ScreenId =
  | 'dashboard'
  | 'ledger'
  | 'quick-add'
  | 'accounts'
  | 'contacts'
  | 'settings'
  | 'invoices'
  | 'bills'
  | 'payments'
  | 'cheques'
  | 'products'
  | 'stock'
  | 'reports'
  | 'pos'
  | 'bank-import'
  | 'backup'
  | 'onboarding'
  | 'stock-adjust'
  | 'purchase-orders'
  | 'chart'
  | 'taxes'
  | 'recurring'
  | 'currencies'
  | 'budgets'
  | 'audit'
  | 'command'
  | 'users'
  | 'warehouses'
  | 'batches'
  | 'receipt-scan'
  | 'sync'

export type NavModule =
  | 'core'
  | 'sales'
  | 'purchases'
  | 'inventory'
  | 'payments'
  | 'reports'
  | 'accounting'
  | 'system'

export type ScreenMeta = {
  id: ScreenId
  title: string
  description: string
  phase: 1 | 2 | 3 | 4
  weight: number
  icon: LucideIcon
  module: NavModule
}

export const SCREENS: ScreenMeta[] = [
  { id: 'dashboard', title: 'داشبورد', description: 'موجودی، طلب و بدهی', phase: 1, weight: 9, icon: LayoutDashboard, module: 'core' },
  { id: 'ledger', title: 'دفتر تراکنش‌ها', description: 'همهٔ حرکت‌های پولی', phase: 1, weight: 10, icon: ListTree, module: 'core' },
  { id: 'quick-add', title: 'ثبت سریع', description: 'هزینه، درآمد یا انتقال', phase: 1, weight: 10, icon: Banknote, module: 'core' },
  { id: 'accounts', title: 'حساب‌ها و کیف‌ها', description: 'نقد، بانک، کارت و وام', phase: 1, weight: 10, icon: Wallet, module: 'core' },
  { id: 'contacts', title: 'مخاطبین', description: 'مشتری و تأمین‌کننده', phase: 1, weight: 9, icon: Users, module: 'core' },
  { id: 'settings', title: 'تنظیمات', description: 'واحد پول، تقویم و ظاهر', phase: 1, weight: 9, icon: Settings, module: 'system' },
  { id: 'invoices', title: 'فاکتورها', description: 'فروش و مطالبات', phase: 2, weight: 8, icon: FileText, module: 'sales' },
  { id: 'pos', title: 'فروش سریع', description: 'صندوق فروشگاهی', phase: 2, weight: 7, icon: ShoppingBag, module: 'sales' },
  { id: 'bills', title: 'صورتحساب و هزینه', description: 'خرید و مخارج', phase: 2, weight: 8, icon: Receipt, module: 'purchases' },
  { id: 'purchase-orders', title: 'سفارش خرید', description: 'سفارش و رسید کالا', phase: 3, weight: 6, icon: ShoppingCart, module: 'purchases' },
  { id: 'payments', title: 'پرداخت‌ها', description: 'دریافت و پرداخت', phase: 2, weight: 8, icon: ArrowLeftRight, module: 'payments' },
  { id: 'cheques', title: 'چک‌ها', description: 'دریافتی و پرداختی', phase: 2, weight: 8, icon: CheckSquare, module: 'payments' },
  { id: 'products', title: 'کالا و خدمات', description: 'فهرست قیمت و موجودی', phase: 2, weight: 8, icon: Tags, module: 'inventory' },
  { id: 'stock', title: 'موجودی', description: 'مقدار و حرکت کالا', phase: 2, weight: 8, icon: Package, module: 'inventory' },
  { id: 'stock-adjust', title: 'انبارگردانی', description: 'تعدیل موجودی', phase: 2, weight: 7, icon: ClipboardList, module: 'inventory' },
  { id: 'warehouses', title: 'انبارها', description: 'چند مکان', phase: 4, weight: 4, icon: Warehouse, module: 'inventory' },
  { id: 'batches', title: 'سری ساخت', description: 'انقضا و سریال', phase: 4, weight: 4, icon: Boxes, module: 'inventory' },
  { id: 'reports', title: 'گزارش‌ها', description: 'سود و زیان تا تراز', phase: 2, weight: 8, icon: FileSpreadsheet, module: 'reports' },
  { id: 'chart', title: 'دفتر کل', description: 'حساب‌ها و سند دستی', phase: 3, weight: 6, icon: BookOpen, module: 'accounting' },
  { id: 'taxes', title: 'مالیات', description: 'نرخ و خلاصه دوره', phase: 3, weight: 6, icon: Percent, module: 'accounting' },
  { id: 'currencies', title: 'ارزها', description: 'نرخ تبدیل', phase: 3, weight: 6, icon: Globe2, module: 'accounting' },
  { id: 'audit', title: 'ممیزی و بستن دوره', description: 'تاریخچه تغییرات', phase: 3, weight: 5, icon: Shield, module: 'accounting' },
  { id: 'bank-import', title: 'صورت‌حساب بانک', description: 'ورود و تطبیق', phase: 2, weight: 7, icon: Landmark, module: 'system' },
  { id: 'backup', title: 'پشتیبان', description: 'ورود و خروج داده', phase: 2, weight: 7, icon: Archive, module: 'system' },
  { id: 'onboarding', title: 'شروع و حالت', description: 'شخصی یا کسب‌وکار', phase: 2, weight: 7, icon: Workflow, module: 'system' },
  { id: 'recurring', title: 'تکرار و یادآور', description: 'اسناد دوره‌ای', phase: 3, weight: 6, icon: CalendarClock, module: 'system' },
  { id: 'budgets', title: 'بودجه و هدف', description: 'برنامه و پیگیری', phase: 3, weight: 5, icon: Calculator, module: 'system' },
  { id: 'command', title: 'جستجوی سراسری', description: 'Ctrl+K', phase: 3, weight: 5, icon: Search, module: 'system' },
  { id: 'users', title: 'کاربران', description: 'نقش و دسترسی', phase: 3, weight: 5, icon: Users, module: 'system' },
  { id: 'receipt-scan', title: 'اسکن رسید', description: 'پیوست و OCR', phase: 4, weight: 3, icon: Camera, module: 'system' },
  { id: 'sync', title: 'همگام‌سازی', description: 'چند دستگاه', phase: 4, weight: 3, icon: RefreshCw, module: 'system' },
]

export const NAV_MODULES: { id: NavModule; label: string }[] = [
  { id: 'core', label: 'اصلی' },
  { id: 'sales', label: 'فروش' },
  { id: 'purchases', label: 'خرید' },
  { id: 'payments', label: 'پرداخت' },
  { id: 'inventory', label: 'انبار' },
  { id: 'reports', label: 'گزارش' },
  { id: 'accounting', label: 'حسابداری' },
  { id: 'system', label: 'سیستم' },
]

export function screenById(id: string): ScreenMeta | undefined {
  return SCREENS.find((s) => s.id === id)
}

export const PHASE1_NAV: ScreenId[] = [
  'dashboard',
  'ledger',
  'accounts',
  'contacts',
]

export const PRIMARY_NAV: ScreenId[] = [
  'dashboard',
  'ledger',
  'accounts',
  'contacts',
  'invoices',
  'bills',
  'payments',
  'products',
  'reports',
]

export const SECONDARY_NAV: ScreenId[] = [
  'settings',
  'pos',
  'cheques',
  'stock',
  'backup',
  'onboarding',
  'chart',
  'taxes',
  'audit',
  'users',
]
