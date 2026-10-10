export type Integer = string;
export interface Company {
  id: string;
  name: string;
}
export interface Period {
  id: string;
  company_id: string;
  name: string;
  start_date: string;
  end_date: string;
  locked: boolean;
}
export interface Account {
  id: string;
  company_id: string;
  code: string;
  name: string;
  classification: "Asset" | "Liability" | "Equity" | "Revenue" | "Expense";
  normal_balance: "Debit" | "Credit";
  postable: boolean;
  control: boolean;
  archived: boolean;
}
export interface JournalLine {
  account: Account;
  debit: Integer;
  credit: Integer;
  description: string;
}
export interface Journal {
  total_debit: Integer;
  total_credit: Integer;
  _generation?: string;
  id: string;
  company_id: string;
  period_id: string;
  date: string;
  source_id: string;
  actor: string;
  reason: string;
  kind: string | { Reversal: { journal_id: string } };
  lines: JournalLine[];
  recorded_at: string;
}
export interface Party {
  id: string;
  company_id: string;
  name: string;
  customer: boolean;
  supplier: boolean;
}
export interface Unit {
  id: string;
  company_id: string;
  name: string;
}
export interface Item {
  id: string;
  company_id: string;
  sku: string;
  name: string;
  base_unit_id: string;
  indivisible: boolean;
}
export interface Resources {
  parties: Party[];
  units: Unit[];
  items: Item[];
  warehouses: Unit[];
}
export interface Context {
  company_id: string;
  period_id: string;
  setup_id: string;
  source_id: string;
  source_revision: Integer;
  date: string;
  request_key: string;
  actor: string;
  reason: string;
}
export interface InputLine {
  id: string;
  item_id: string;
  warehouse_id: string;
  unit_id: string;
  quantity: string;
  unit_price: Integer;
  line_discount: Integer;
  price_reference: string;
  price_date: string;
  override_reason: string;
}
export interface Draft {
  id: string;
  revision: Integer;
  company_id: string;
  period_id: string;
  setup_id: string;
  customer_id: string;
  reference: string;
  document_date: string;
  financial_date: string;
  date_display: "Iso" | "Jalali";
  display_unit: "Rial" | "Toman";
  document_discount: string;
  supplied_total: string;
  reason: string;
  quotation: null;
  lines: InputLine[];
}
export interface FrozenLine {
  input: {
    id: string;
    item_id: string;
    warehouse_id: string;
    unit_id: string;
    quantity_thousandths: Integer;
    unit_price: Integer;
    line_discount: Integer;
  };
  item: Item;
  base_quantity_thousandths: Integer;
  gross: Integer;
  document_discount: Integer;
  net: Integer;
}
export interface Data {
  total?: Integer;
  cost?: Integer;
  lines?: FrozenLine[];
  customer?: Party;
  supplier?: Party;
  customer_id?: string;
  supplier_id?: string;
  order_id?: string;
  delivery_id?: string;
  invoice_id?: string;
}
export interface RecordCommand {
  context: Context;
  reference?: string;
  invoice_reference?: string;
  return_reference?: string;
  delivery_reference?: string;
  document_date?: string;
  due_date?: string;
  order_id?: string;
  lines?: unknown[];
}
export interface RecordEntry {
  id: string;
  sequence: Integer;
  journal_id: string | null;
  command: Record<string, RecordCommand>;
  data: Record<string, Data>;
}
export interface Settings {
  business_name: string;
  business_phone: string;
  business_address: string;
  header: string;
  footer: string;
  default_terms: string;
  default_currency: "Rial" | "Toman";
  quote_prefix: string;
  next_number: Integer;
  default_validity_days: Integer;
  default_discount_rial: Integer;
  tax_basis_points: Integer;
  appearance: "System" | "Light" | "Dark";
  density: "Comfortable" | "Compact";
  display_digits: "Persian" | "Latin";
  date_display?: "Iso" | "DayMonthYear" | "Jalali";
}
export interface Setup {
  id: string;
  command: {
    company_id: string;
    period_id: string;
    policy: { effective_date: string };
    mappings: { role: string; account_id: string }[];
  };
}
export interface Workspace {
  companies: Company[];
  periods: Period[];
  accounts: Account[];
  setups: Setup[];
  initialized: boolean;
  resources: Resources | null;
  journals: Journal[];
  journal_count: Integer;
  sales: RecordEntry[];
  sales_count: Integer;
  purchases: RecordEntry[];
  purchase_count: Integer;
  drafts: Draft[];
  audit: {
    id: string;
    actor: string;
    operation: string;
    reason: string;
    recorded_at: string;
  }[];
  settings: Settings;
  capabilities: Record<string, boolean>;
}
export interface TrialRow {
  account_id: string;
  code: string;
  name: string;
  debit: Integer;
  credit: Integer;
}
export interface OpenItem {
  id: string;
  reference: string;
  party_id?: string;
  supplier_id?: string;
  due_date: string;
  original: Integer;
  remaining: Integer;
  source_id: string;
}
export interface Report {
  party_balances: Record<string, Integer>;
  company_id: string;
  from: string;
  to: string;
  trial: { rows: TrialRow[]; total_debit: Integer; total_credit: Integer };
  position: { cash: Integer; receivables: Integer; payables: Integer };
  profit_loss: {
    rows: {
      account_id: string;
      code: string;
      name: string;
      classification: string;
      amount: Integer;
    }[];
    revenue: Integer;
    expenses: Integer;
    profit: Integer;
  };
  reconciliation: null | {
    reconciled: boolean;
    controls: {
      role: string;
      account_id: string;
      ledger: Integer;
      detail: Integer;
      difference: Integer;
    }[];
    receivables: OpenItem[];
    payables: OpenItem[];
    stock: {
      item_id: string;
      warehouse_id: string;
      quantity_thousandths: Integer;
      value: Integer;
    }[];
  };
}
export interface Preview {
  _generation?: string;
  total: Integer;
  cost?: Integer;
  journal_lines: {
    account_id: string;
    debit: Integer;
    credit: Integer;
    description: string;
  }[];
  nonfinancial: boolean;
}
export type Page =
  | "overview"
  | "sales"
  | "purchases"
  | "accounting"
  | "inventory"
  | "reports"
  | "customers"
  | "settings"
  | "banking";
