export interface BusinessProfile {
  phone: string;
  address: string;
  registration_id: string;
  tax_id: string;
  invoice_prefix: string;
  terms: string;
  footer: string;
  bank_details: string;
}
export const emptyBusinessProfile: BusinessProfile = {
  phone: "",
  address: "",
  registration_id: "",
  tax_id: "",
  invoice_prefix: "",
  terms: "",
  footer: "",
  bank_details: "",
};
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
export interface PartyProfile {
  company_id: string;
  display_name: string;
  first_name: string;
  last_name: string;
  organization: string;
  phone: string;
  mobile: string;
  email: string;
  address: string;
  city: string;
  postal_code: string;
  national_id: string;
  tax_id: string;
  website: string;
  notes: string;
}
export const emptyPartyProfile: PartyProfile = {
  company_id: "",
  display_name: "",
  first_name: "",
  last_name: "",
  organization: "",
  phone: "",
  mobile: "",
  email: "",
  address: "",
  city: "",
  postal_code: "",
  national_id: "",
  tax_id: "",
  website: "",
  notes: "",
};
export interface ContactDetail {
  _generation: string;
  party: Party;
  profile: PartyProfile;
  as_of: string;
  stats: {
    orders: Integer;
    invoices: Integer;
    returns: Integer;
    gross_sales: Integer;
    returns_value: Integer;
    net_sales: Integer;
    gross_profit: Integer;
    outstanding: Integer;
    overdue: Integer;
    last_activity: string | null;
  };
  records: RecordEntry[];
  record_count: Integer;
  receivables: OpenItem[];
  payables: OpenItem[];
  monthly: { month: string; net: Integer }[];
  products: { id: string; name: string; quantity: Integer; revenue: Integer }[];
}
export interface BankingData {
  _generation: string;
  accounts: {
    account: Account;
    role: "Cash" | "Bank";
    opening: Integer;
    incoming: Integer;
    outgoing: Integer;
    balance: Integer;
  }[];
  movements: Journal[];
  movement_count: Integer;
  counterparts: Account[];
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
export interface QuoteIssue {
  series: string;
  number: string;
  issued: string;
  valid_until: string;
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
  quote_issue?: QuoteIssue | null;
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
  credit_id?: string;
  total?: Integer;
  cost?: Integer;
  applied?: Integer;
  advance_amount?: Integer;
  payment_id?: string;
  reference?: string;
  party?: Party;
  allocations?: { open_item_id: string; amount: Integer }[];
  lines?: FrozenLine[];
  customer?: Party;
  supplier?: Party;
  customer_id?: string;
  supplier_id?: string;
  order_id?: string;
  delivery_id?: string;
  invoice_id?: string;
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
  party_id?: string;
}
export interface RecordCommand {
  context: Context;
  reference?: string;
  party_id?: string;
  funds_role?: string;
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
export type ChequeStatus =
  | "in_hand"
  | "in_collection"
  | "cleared"
  | "bounced"
  | "spent"
  | "returned"
  | "issued"
  | "voided";
export type ChequeAction =
  "deposit" | "clear" | "bounce" | "spend" | "return" | "void";
export interface ChequeDetails {
  sayad_id: string;
  number: string;
  bank: string;
  branch: string;
  due_date: string;
  drawer_national_id: string;
  purpose: string;
  notes: string;
}
export interface ChequeRow {
  cheque: {
    id: string;
    company_id: string;
    direction: "Received" | "Issued";
    amount: Integer;
    party_id: string;
    details: ChequeDetails;
    opening_id: string | null;
  };
  status: ChequeStatus;
  /** Next steps the engine's state machine allows for this cheque right now. */
  allowed: ChequeAction[];
  events: { id: string; action: string; date: string; reason: string }[];
}
/** Full-validation safety net that runs in the background after an import. */
export type IntegrityStatus =
  | { status: "unchecked" }
  | { status: "running"; generation: Integer }
  | { status: "ok"; generation: Integer }
  | { status: "failed"; generation: Integer; message: string };
export interface Workspace {
  integrity?: IntegrityStatus;
  cheques?: ChequeRow[];
  product_profiles: Record<string, ProductProfile>;
  party_profiles: Record<string, PartyProfile>;
  business_profile: BusinessProfile;
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
  payments: PaymentRecordEntry[];
  payment_count: Integer;
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
export interface StatementRow {
  account_id: string;
  code: string;
  name: string;
  amount: Integer;
}
export interface BalanceSheet {
  as_of: string;
  assets: StatementRow[];
  liabilities: StatementRow[];
  equity: StatementRow[];
  current_earnings: Integer;
  total_assets: Integer;
  total_liabilities: Integer;
  total_equity: Integer;
  balanced: boolean;
}
export interface AgingRow {
  party_id: string;
  not_due: Integer;
  days_1_30: Integer;
  days_31_60: Integer;
  days_61_90: Integer;
  days_over_90: Integer;
  total: Integer;
}
export interface Aging {
  side: "receivable" | "payable";
  as_of: string;
  rows: AgingRow[];
  totals: AgingRow;
}
export interface Report {
  /** Present once the company has confirmed opening balances. */
  balance_sheet?: BalanceSheet | null;
  aging?: { receivable: Aging; payable: Aging } | null;
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
  | "payments"
  | "accounting"
  | "inventory"
  | "reports"
  | "customers"
  | "settings"
  | "banking";

export type PaymentSide = "receive" | "make";
export interface PaymentOpenItem {
  id: string;
  party_id: string;
  reference: string;
  document_date: string;
  due_date: string;
  remaining: Integer;
}
export interface PaymentAllocationInput {
  open_item_id: string;
  amount: Integer;
}
export interface PaymentRecordEntry {
  id: string;
  sequence: Integer;
  journal_id: string;
  command: Record<
    string,
    {
      context: Context;
      side?: "Receive" | "Make";
      party_id?: string;
      funds_role?: "Cash" | "Bank";
      reference?: string;
      payment_id?: string;
    }
  >;
  data: Record<
    string,
    {
      side?: "Receive" | "Make";
      payment_id?: string;
      party?: Party;
      reference?: string;
      allocations?: {
        open_item_id: string;
        party_id: string;
        reference: string;
        amount: Integer;
      }[];
      applied?: Integer;
      advance_amount?: Integer;
      total?: Integer;
    }
  >;
}
export interface PaymentOptions {
  items: PaymentOpenItem[];
}
export interface PaymentPreviewData {
  _generation?: string;
  data: Record<string, unknown>;
  journal_lines: {
    account_id: string;
    debit: Integer;
    credit: Integer;
    description: string;
  }[];
  total: Integer;
}

export interface ProductProfile {
  company_id: string;
  purchase_price: Integer | null;
  sale_price: Integer | null;
  barcode: string;
  category: string;
  brand: string;
  minimum_stock_thousandths: Integer;
  minimum_stock?: string;
  preferred_supplier_id: string;
  notes: string;
}
export const emptyProductProfile: ProductProfile = {
  company_id: "",
  purchase_price: null,
  sale_price: null,
  barcode: "",
  category: "",
  brand: "",
  minimum_stock_thousandths: "0",
  preferred_supplier_id: "",
  notes: "",
};
export interface ProductDetail {
  _generation: Integer;
  item: Item;
  profile: ProductProfile;
  as_of: string;
  stock: {
    item_id: string;
    warehouse_id: string;
    quantity_thousandths: Integer;
    value: Integer;
  }[];
  stats: {
    stock_quantity: Integer;
    stock_value: Integer;
    average_stock_cost: Integer | null;
    low_stock: boolean;
    sold_quantity: Integer;
    returned_quantity: Integer;
    net_sales: Integer;
    cost_of_goods_sold: Integer;
    gross_profit: Integer;
    returns_value: Integer;
    purchased_quantity: Integer;
    purchase_returns_quantity: Integer;
    net_purchase_value: Integer;
    last_purchase_price: Integer | null;
    last_sale_price: Integer | null;
    estimated_unit_profit: Integer | null;
    margin_basis_points: Integer | null;
    markup_basis_points: Integer | null;
  };
  monthly: {
    month: string;
    net_sales: Integer;
    cost: Integer;
    profit: Integer;
  }[];
  customers: {
    id: string;
    quantity: Integer;
    net_sales: Integer;
    profit: Integer;
  }[];
  suppliers: { id: string; quantity: Integer; value: Integer }[];
  records: RecordEntry[];
  record_count: Integer;
}
