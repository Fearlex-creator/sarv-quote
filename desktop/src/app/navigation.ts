import { SquaresFourIcon as LayoutDashboard } from "@phosphor-icons/react/dist/csr/SquaresFour";
import { FileTextIcon as FileText } from "@phosphor-icons/react/dist/csr/FileText";
import { ShoppingBagIcon as ShoppingBag } from "@phosphor-icons/react/dist/csr/ShoppingBag";
import { BookOpenIcon as BookOpen } from "@phosphor-icons/react/dist/csr/BookOpen";
import { StackIcon as Boxes } from "@phosphor-icons/react/dist/csr/Stack";
import { ChartLineUpIcon as ChartNoAxesCombined } from "@phosphor-icons/react/dist/csr/ChartLineUp";
import { UsersIcon as Users } from "@phosphor-icons/react/dist/csr/Users";
import { GearIcon as Settings } from "@phosphor-icons/react/dist/csr/Gear";
import { BankIcon as Landmark } from "@phosphor-icons/react/dist/csr/Bank";
import { ArrowsLeftRightIcon as ArrowsLeftRight } from "@phosphor-icons/react/dist/csr/ArrowsLeftRight";
import { CashRegisterIcon as CashRegister } from "@phosphor-icons/react/dist/csr/CashRegister";
import { FileXlsIcon as FileXls } from "@phosphor-icons/react/dist/csr/FileXls";
import type { Page } from "../lib/types";
export type NavGroup = "daily" | "people" | "books" | "workspace";
export const navigationGroups: { id: NavGroup; label: string }[] = [
  { id: "daily", label: "Daily work" },
  { id: "people", label: "Stock & people" },
  { id: "books", label: "Books" },
  { id: "workspace", label: "Workspace" },
];
export const navigation: {
  page: Page;
  label: string;
  icon: typeof LayoutDashboard;
  group: NavGroup;
}[] = [
  { page: "overview", label: "Overview", icon: LayoutDashboard, group: "daily" },
  { page: "sales", label: "Sales", icon: FileText, group: "daily" },
  { page: "pos", label: "Point of sale", icon: CashRegister, group: "daily" },
  { page: "purchases", label: "Purchases", icon: ShoppingBag, group: "daily" },
  { page: "payments", label: "Payments", icon: ArrowsLeftRight, group: "daily" },
  { page: "banking", label: "Banking", icon: Landmark, group: "daily" },
  { page: "customers", label: "Contacts", icon: Users, group: "people" },
  { page: "inventory", label: "Inventory", icon: Boxes, group: "people" },
  { page: "accounting", label: "Accounting", icon: BookOpen, group: "books" },
  { page: "reports", label: "Reports", icon: ChartNoAxesCombined, group: "books" },
  { page: "settings", label: "Settings", icon: Settings, group: "workspace" },
  { page: "excel", label: "Excel", icon: FileXls, group: "workspace" },
];
export const persian: Record<string, string> = {
  Overview: "نمای کلی",
  Sales: "فروش",
  "Point of sale": "صندوق فروش",
  Excel: "اکسل",
  Purchases: "خریدها",
  Payments: "پرداخت‌ها",
  Accounting: "حسابداری",
  Inventory: "موجودی",
  Reports: "گزارش‌ها",
  Banking: "بانکداری",
  Customers: "مشتریان",
  Suppliers: "تأمین‌کنندگان",
  Settings: "تنظیمات",
  "Help & shortcuts": "راهنما و میانبرها",
  Workspace: "فضای کاری",
  "Search anything…": "جست‌وجو…",
  "Your business": "کسب‌وکار شما",
  "New journal": "سند جدید",
  "New order": "سفارش جدید",
  "Set up company": "راه‌اندازی کسب‌وکار",
  "Financial position from posted entries.":
    "وضعیت مالی بر پایهٔ اسناد ثبت‌شده.",
  "Cash and bank": "صندوق و بانک",
  Receivables: "حساب‌های دریافتنی",
  Payables: "حساب‌های پرداختنی",
  "Recent entries": "اسناد اخیر",
  "View all": "نمایش همه",
  "Start with reliable books": "با دفاتر قابل اعتماد شروع کنید",
  "Create a company and fiscal period": "کسب‌وکار و سال مالی را تعریف کنید",
  "Choose your chart of accounts": "کدینگ حساب‌ها را تعیین کنید",
  "Record your opening balances": "مانده‌های افتتاحیه را ثبت کنید",
  "Available workflows": "گردش‌کارهای موجود",
  "Goods sales": "فروش کالا",
  "Ledger and accounting": "دفتر حسابداری",
  "Banking and payments": "بانکداری و پرداخت‌ها",
  "Not yet supported.": "هنوز پشتیبانی نمی‌شود.",
  "No posted entries.": "سندی ثبت نشده است.",
  Date: "تاریخ",
  Reference: "مرجع",
  Description: "شرح",
  Debit: "بدهکار",
  Credit: "بستانکار",
  Amount: "مبلغ",
  Status: "وضعیت",
  Draft: "پیش‌نویس",
  Posted: "ثبت‌شده",
  Outstanding: "تسویه‌نشده",
  Orders: "سفارش‌ها",
  Invoices: "فاکتورها",
  Drafts: "پیش‌نویس‌ها",
  Bills: "صورتحساب‌ها",
  Vendors: "تأمین‌کنندگان",
  "Chart of accounts": "کدینگ حساب‌ها",
  Journals: "اسناد",
  "General ledger": "دفتر کل",
  "Fiscal periods": "سال‌های مالی",
  "Profit & Loss": "سود و زیان",
  "Trial balance": "تراز آزمایشی",
  "Control reconciliation": "تطبیق حساب‌های کنترلی",
  "Save draft": "ذخیرهٔ پیش‌نویس",
  "Review order": "بررسی سفارش",
  Cancel: "انصراف",
  Save: "ذخیره",
  Revenue: "درآمد",
  Expenses: "هزینه‌ها",
  "Net profit": "سود خالص",
  "Export CSV": "خروجی CSV",
  Products: "کالاها",
  Warehouses: "انبارها",
  Units: "واحدها",
  Stock: "موجودی",
  "New customer": "مشتری جدید",
  Name: "نام",
  Code: "کد",
  Customer: "مشتری",
  Vendor: "تأمین‌کننده",
  Quantity: "مقدار",
  "Unit price": "قیمت واحد",
  "Due date": "سررسید",
  Total: "جمع",
  "No records yet.": "هنوز رکوردی ثبت نشده است.",
};
