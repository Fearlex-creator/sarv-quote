import { invoke, isTauri } from '@tauri-apps/api/core'

/** All monetary values are exact minor-unit strings. Never coerce to Number. */
export type Amount = { currency: string; minor: string }
export type Settings = {
  profile: string; phone: string; address: string; taxId: string; logo: string;
  fiscalMonth: number; currency: string; calendar: 'gregorian'|'jalali';
  language: 'en'|'fa'; direction: 'ltr'|'rtl'; digits: 'latin'|'persian';
  dateFormat: 'medium'|'short'|'iso'; theme: 'light'|'dark'|'system'; accent: string;
  mode: 'personal'|'business'; taxFormat: 'exclusive'|'inclusive'; closedThrough: string;
  widgets: string[]; hiddenWidgets: string[]; lastAccount: string;
}
export type Account = {
  id: string; name: string; kind: string; currency: string; group: string;
  archived: number; sortOrder: number; reconciledThrough: string|null;
  balanceMinor: string; openingMinor: string;
}
export type ContactInput = {
  id: string; name: string; customer: boolean; supplier: boolean; phone: string;
  email: string; address: string; taxId: string; notes: string;
}
export type Contact = Omit<ContactInput, 'customer'|'supplier'> & {
  customer: number; supplier: number;
  balances: {currency: string; receivableMinor: string; payableMinor: string; receivableAging: string[]; payableAging: string[]}[];
}
export type Category = {id: string; name: string; kind: string}
export type Filters = {search?:string; from?:string; to?:string; account?:string; category?:string; contact?:string; status?:string; currency?:string; offset?:number; limit?:number}
export type LedgerRow = {
  id: string; date: string; kind: string; payee: string; memo: string; tags: string;
  category: string; account: string; currency: string; amountMinor: string;
  movementMinor: string; paidMinor: string; balanceMinor: string; status: string; recordedAt: string;
}
export type Page = {rows:LedgerRow[]; total:number; offset:number; nextOffset:number|null; balanceCurrency:string}
export type Receipt = {name:string; mime:string; bytes:number[]}
export type TransactionInput = {
  date:string; kind:string; amount:string; paidAmount:string; accountId:string; toAccountId:string;
  payee:string; contactId:string; categoryId:string; memo:string; tags:string; dueDate:string;
  splits:{categoryId:string; amount:string}[]; receipt?:Receipt|null; correctionId?:string; correctionDate?:string;
}
export type AccountInput = {id?:string; name:string; kind:string; currency:string; group:string; openingBalance?:string; openingDate?:string; archived?:boolean; sortOrder?:number}
export type Detail = {
  id:string; input:TransactionInput; currency:string; amountMinor:string; paidMinor:string;
  recordedAt:string; reverses:string|null; replaces:string|null; reversedBy:string|null;
  lines:{account:string;kind:string;category:string|null;debitMinor:string;creditMinor:string}[];
  attachments:{id:string;name:string;mime:string;size:number}[];
  history:{id:number;at:string;action:string;entityId:string;detail:string}[];
  balanceHistory:{date:string;minor:string}[];
}
export type Workspace = {
  settings:Settings; currencies:{code:string;decimals:number}[]; accounts:Account[];
  contacts:Contact[]; categories:Category[]; totals:Amount[]; transactionCount:number;
  savedViews:{id:string;name:string;filters:Filters}[];
  memories:{payee:string;categoryId:string;accountId:string}[];
  audit:{id:number;at:string;action:string;entityId:string;detail:string}[];
  dashboard:{currency:string;from:string;to:string;cashMinor:string;incomeMinor:string;expenseMinor:string;profitMinor:string;
    incomeChange:string;expenseChange:string;profitChange:string;previousIncomeMinor:string;previousExpenseMinor:string;
    trend:{month:string;incomeMinor:string;expenseMinor:string;incomeHeight:number;expenseHeight:number}[];
    receivables:{id:string;name:string;minor:string;currency:string}[];
    payables:{id:string;name:string;minor:string;currency:string}[];recent:LedgerRow[]};
}
export type Export = {name:string;mime:string;content:string;bytes?:number[]}

async function request<T>(mutation:boolean,action:string,payload:unknown={}):Promise<T> {
  if (isTauri()) {
    try {return await invoke<T>(mutation?'accounting_mutate':'accounting_view',{action,payload})}
    catch(error){throw new Error(String(error))}
  }
  const token=import.meta.env.VITE_ACCOUNTING_PREVIEW_TOKEN as string|undefined
  if (!import.meta.env.DEV || !token) throw new Error('Open the desktop app with pnpm tauri dev. Browser development requires the Rust preview service (see README).')
  const response=await fetch('/api/accounting',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({mutation,action,payload,token})})
  const body=await response.json() as {data:T;error?:string}
  if (!response.ok || body.error) throw new Error(body.error??'The local ledger could not be reached')
  return body.data
}

export const commands = {
  getWorkspace:(range:Record<string,unknown>={})=>request<Workspace>(false,'workspace',range),
  listTransactions:(filters:Filters)=>request<Page>(false,'ledger',filters),
  getTransaction:(id:string)=>request<Detail>(false,'detail',{id}),
  getAccount:(id:string)=>request<{currency:string;history:{date:string;minor:string}[];recent:Page}>(false,'account',{id}),
  previewTransaction:(input:TransactionInput)=>request<Detail>(false,'preview',input),
  createTransaction:(input:TransactionInput)=>request<{id:string}>(true,'transaction',input),
  saveAccount:(input:AccountInput)=>request<{id:string}>(true,'account',input),
  saveContact:(input:ContactInput)=>request<{id:string}>(true,'contact',input),
  updateSettings:(input:Settings)=>request<Settings>(true,'settings',input),
  mutate:(action:string,payload:unknown)=>request<Record<string,unknown>>(true,action,payload),
  exportData:(scope:string,filters?:Filters)=>request<Export>(false,'export',{scope,filters}),
  getAttachment:(id:string)=>request<Receipt>(false,'attachment',{id}),
}

export function downloadFile(file:Export) {
  const blob=new Blob([file.bytes?new Uint8Array(file.bytes):file.content],{type:file.mime})
  const url=URL.createObjectURL(blob);const anchor=document.createElement('a');anchor.href=url;anchor.download=file.name;anchor.click()
  window.setTimeout(()=>URL.revokeObjectURL(url),30_000)
}
