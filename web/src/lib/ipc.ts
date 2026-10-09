import { invoke } from '@tauri-apps/api/core'

export type PartyBalance = { name: string; amountRial: number }
export type AgingBucket = { label: string; amountRial: number }

export type SplitView = {
  category: string
  amountRial: number
  debitRial: number
  creditRial: number
}

export type DashboardView = {
  businessName: string
  modeLabel: string
  appMode: string
  cashOnHandRial: number
  incomeRial: number
  expenseRial: number
  profitRial: number
  receivableCount: number
  payableHint: string
  receivables: PartyBalance[]
  payables: PartyBalance[]
  accountCount: number
  journalCount: number
  contactCount: number
  catalogCount: number
  quoteCount: number
  companyReady: boolean
  currencyLabel: string
  recent: LedgerRow[]
}

export type LedgerRow = {
  id: string
  date: string
  memo: string
  kind: string
  actor: string
  payee: string
  category: string
  account: string
  amountRial: number
  signedAmountRial: number
  runningBalanceRial: number
  debitRial: number
  creditRial: number
  accountSummary: string
  status: string
  attachmentName: string
  splits: SplitView[]
}

export type AccountRow = {
  id: string
  code: string
  name: string
  classification: string
  accountType: string
  group: string
  normalBalance: string
  postable: boolean
  archived: boolean
  openingBalanceRial: number
  balanceRial: number
  currency: string
  sortOrder: number
}

export type ContactRow = {
  id: string
  name: string
  phone: string
  email: string
  address: string
  contactName: string
  archived: boolean
  roles: string[]
  isCustomer: boolean
  isSupplier: boolean
  taxId: string
  notes: string
  balanceRial: number
  aging: AgingBucket[]
}

export type ContactInput = {
  id?: string
  name: string
  phone?: string
  email?: string
  address?: string
  contactName?: string
  archived?: boolean
  isCustomer?: boolean
  isSupplier?: boolean
  taxId?: string
  notes?: string
}

export type WalletInput = {
  id?: string
  name: string
  kind?: 'cash' | 'bank' | 'card' | 'wallet' | 'savings' | 'loan'
  group?: string
  openingBalanceRial?: number
  archived?: boolean
  sortOrder?: number
  currency?: string
}

export type SuggestView = {
  payees: string[]
  categories: string[]
  accounts: string[]
  payeeCategory: Record<string, string>
  lastAccount: string
  lastToAccount: string
  lastKind: string
}

export type CashSplit = { category: string; amountRial: number }

export type CashEntryInput = {
  kind: 'expense' | 'income' | 'transfer'
  amountRial: number
  payee: string
  category?: string
  account?: string
  toAccount?: string
  memo?: string
  date?: string
  splits?: CashSplit[]
  attachmentName?: string
}

export type Settings = {
  business_name: string
  business_phone: string
  business_address: string
  header: string
  footer: string
  default_terms: string
  default_currency: 'Toman' | 'Rial'
  quote_prefix: string
  next_number: number
  default_validity_days: number
  default_discount_rial: number
  tax_basis_points: number
  appearance: 'System' | 'Light' | 'Dark'
  density: 'Comfortable' | 'Compact'
  display_digits: 'Persian' | 'Latin'
  date_display: 'Iso' | 'DayMonthYear' | 'Jalali'
  app_mode: 'Personal' | 'Business'
  fiscal_year_start_month: number
  text_direction: 'Rtl' | 'Ltr'
  accent: string
}

export type SettingsView = {
  settings: Settings
  currency: string
  dateDisplay: string
  appearance: string
  digits: string
  appMode: string
  textDirection: string
}

export const ipc = {
  getDashboard: () => invoke<DashboardView>('get_dashboard'),
  listLedger: () => invoke<LedgerRow[]>('list_ledger'),
  listAccounts: () => invoke<AccountRow[]>('list_accounts'),
  saveWallet: (wallet: WalletInput) => invoke('save_wallet', { wallet }),
  listContacts: () => invoke<ContactRow[]>('list_contacts'),
  saveContact: (contact: ContactInput) =>
    invoke<ContactRow>('save_contact', { contact }),
  getSettingsView: () => invoke<SettingsView>('get_settings_view'),
  updateSettings: (settings: Settings) =>
    invoke<Settings>('update_settings', { settings }),
  getSuggestions: () => invoke<SuggestView>('get_suggestions'),
  addCashEntry: (entry: CashEntryInput) => invoke('add_cash_entry', { entry }),
}
