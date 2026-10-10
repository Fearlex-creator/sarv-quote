import {useEffect,useRef,useState} from 'react'
import {useQuery} from '@tanstack/react-query'
import {Plus,Paperclip,ArrowRightLeft,ArrowDownLeft,ArrowUpRight,Trash2} from 'lucide-react'
import {Textarea} from '@/components/ui/textarea'
import {commands,type TransactionInput} from '@/lib/commands'
import {date,money,today} from '@/lib/exact-format'
import {useAccounting} from './context'
import {Form,Button,Field,Input,Select,ErrorMessage,readReceipt} from './common'

export function QuickAddTransaction({initial,onDone,embedded=false}:{initial?:Partial<TransactionInput>;onDone?:()=>void;embedded?:boolean}){
  const {data,t,run,busy}=useAccounting();const s=data.settings
  const defaults=():TransactionInput=>({date:today(),kind:'expense',amount:'',paidAmount:'',accountId:s.lastAccount||data.accounts.find(a=>!a.archived)?.id||'',toAccountId:'',payee:'',contactId:'',categoryId:'',memo:'',tags:'',dueDate:'',splits:[],...initial})
  const [form,setForm]=useState(defaults);const [partial,setPartial]=useState(!!initial?.paidAmount);const [error,setError]=useState<unknown>();const [draft,setDraft]=useState(form)
  const receiptRef=useRef<HTMLInputElement>(null);const formRef=useRef<HTMLFormElement>(null)
  useEffect(()=>{const timer=setTimeout(()=>setDraft(form),350);return()=>clearTimeout(timer)},[form])
  const preview=useQuery({queryKey:['preview',draft],queryFn:()=>commands.previewTransaction(draft),enabled:!!draft.amount&&!!draft.accountId&&!!draft.payee&&(!!draft.categoryId||draft.kind==='transfer'||draft.splits.length>0),retry:false,staleTime:60_000})
  const update=<K extends keyof TransactionInput>(key:K,value:TransactionInput[K])=>setForm(current=>({...current,[key]:value}))
  const categories=data.categories.filter(c=>c.kind===form.kind)
  const account=data.accounts.find(a=>a.id===form.accountId);const currency=account?.currency||s.currency
  const save=async(another:boolean)=>{
    if(!formRef.current?.reportValidity())return
    setError(undefined)
    try{await run(()=>commands.createTransaction({...form,paidAmount:partial?form.paidAmount:'',correctionDate:form.correctionId?form.date:undefined}),'Transaction posted');if(another){setForm({...defaults(),accountId:form.accountId,kind:form.kind});setPartial(false);formRef.current?.querySelector<HTMLInputElement>('input[name="amount"]')?.focus()}else onDone?.()}
    catch(err){setError(err)}
  }
  return <div className={embedded?'quick-form':'standalone-quick'}>
    {!embedded&&<div className="page-heading"><div><h1>{t('New entry')}</h1><p>{t('Record income, an expense, or an account transfer.')}</p></div></div>}
    <Form ref={formRef} onSubmit={e=>{e.preventDefault();void save(false)}} className="form-stack">
      {form.correctionId&&<div className="info-note">{t('Corrections reverse the original and post a new entry.')}</div>}
      <fieldset className="transaction-kinds"><legend className="sr-only">{t('Type')}</legend>{[['expense',ArrowUpRight,'Expense'],['income',ArrowDownLeft,'Income'],['transfer',ArrowRightLeft,'Transfer']].map(([kind,Icon,label])=>{
        const KindIcon=Icon as typeof ArrowUpRight
        return <label key={String(kind)} data-selected={form.kind===kind}><input type="radio" name="kind" value={String(kind)} checked={form.kind===kind} onChange={()=>setForm(current=>({...current,kind:String(kind),categoryId:'',splits:[],paidAmount:'',dueDate:''}))}/><KindIcon aria-hidden="true"/><span>{t(String(label))}</span></label>
      })}</fieldset>
      <Field label={t(form.kind==='transfer'?'Amount':'Total amount')} hint={`${currency} · ${data.currencies.find(c=>c.code===currency)?.decimals??0} ${t('Decimal places')}`}>
        <Input name="amount" required autoFocus inputMode="decimal" dir="ltr" placeholder="0.00" value={form.amount} onChange={e=>update('amount',e.target.value)} className="amount-input"/>
      </Field>
      <div className="form-grid">
        <Field label={t('Account')}><Select required value={form.accountId} onChange={e=>update('accountId',e.target.value)}><option value="">{t('Choose an account')}</option>{data.accounts.filter(a=>!a.archived).map(a=><option key={a.id} value={a.id}>{a.name} · {a.currency}</option>)}</Select></Field>
        <Field label={t(form.correctionId?'Correction date':'Date')}><Input type="date" required value={form.date} onChange={e=>update('date',e.target.value)}/></Field>
      </div>
      <small className="muted">{date(form.date,s)} · {t('Use Gregorian ISO dates for input. Display follows your calendar.')}</small>
      {form.kind==='transfer'?<Field label={t('Destination account')}><Select required value={form.toAccountId} onChange={e=>update('toAccountId',e.target.value)}><option value="">{t('Choose destination')}</option>{data.accounts.filter(a=>!a.archived&&a.id!==form.accountId&&a.currency===currency).map(a=><option key={a.id} value={a.id}>{a.name}</option>)}</Select></Field>:null}
      <Field label={t('Payee')}><Input required maxLength={160} list="remembered-payees" value={form.payee} onChange={e=>{
        const remembered=data.memories.find(m=>m.payee.toLowerCase()===e.target.value.toLowerCase());const valid=data.categories.some(c=>c.id===remembered?.categoryId&&c.kind===form.kind)
        setForm({...form,payee:e.target.value,categoryId:valid?remembered!.categoryId:form.categoryId})
      }}/></Field>
      <datalist id="remembered-payees">{data.memories.map(m=><option key={m.payee} value={m.payee}/>)}</datalist>
      {form.kind!=='transfer'&&<>
        {!form.splits.length&&<Field label={t('Category')}><Select required value={form.categoryId} onChange={e=>update('categoryId',e.target.value)}><option value="">{t('Choose a category')}</option>{categories.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select></Field>}
        <div className="split-header"><span>{t('Split categories')}</span><Button variant="ghost" type="button" onClick={()=>update('splits',[...form.splits,{categoryId:form.categoryId,amount:''}])}><Plus data-icon="inline-start"/>{t('Add split')}</Button></div>
        {form.splits.map((split,index)=><div className="split-row" key={index}><Select aria-label={`${t('Category')} ${index+1}`} required value={split.categoryId} onChange={e=>update('splits',form.splits.map((v,i)=>i===index?{...v,categoryId:e.target.value}:v))}><option value="">{t('Choose a category')}</option>{categories.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select><Input required aria-label={`${t('Amount')} ${index+1}`} inputMode="decimal" dir="ltr" value={split.amount} onChange={e=>update('splits',form.splits.map((v,i)=>i===index?{...v,amount:e.target.value}:v))}/><Button variant="ghost" size="icon" type="button" aria-label={t('Remove split')} onClick={()=>update('splits',form.splits.filter((_,i)=>i!==index))}><Trash2/></Button></div>)}
        <label className="checkbox-label"><input type="checkbox" checked={partial} onChange={e=>{setPartial(e.target.checked);if(!e.target.checked)update('paidAmount','')}}/>{t('Partial payment')}</label>
        {partial&&<div className="form-grid"><Field label={t('Paid now')}><Input required inputMode="decimal" dir="ltr" value={form.paidAmount} onChange={e=>update('paidAmount',e.target.value)}/></Field><Field label={t('Due date')}><Input type="date" value={form.dueDate} onChange={e=>update('dueDate',e.target.value)}/></Field></div>}
      </>}
      <Field label={t('Contact')}><Select required={partial&&form.kind!=='transfer'} value={form.contactId} onChange={e=>{const contact=data.contacts.find(c=>c.id===e.target.value);setForm({...form,contactId:e.target.value,payee:form.payee||contact?.name||''})}}><option value="">{t('No contact')}</option>{data.contacts.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select></Field>
      <Field label={t('Memo')}><Textarea value={form.memo} maxLength={4000} onChange={e=>update('memo',e.target.value)}/></Field>
      <Field label={t('Tags')}><Input value={form.tags} maxLength={500} placeholder="work, travel" onChange={e=>update('tags',e.target.value)}/></Field>
      {s.mode==='business'&&preview.data&&<section className="journal-preview"><h3>{t('Journal lines')}</h3>{preview.data.lines.map((l,i)=><div key={i}><span>{l.category||t(l.kind==='receivable'?'Receivables':l.kind==='payable'?'Payables':l.account)}</span><span>{t(l.debitMinor!=='0'?'Debit':'Credit')} {money(l.debitMinor!=='0'?l.debitMinor:l.creditMinor,currency,s)}</span></div>)}</section>}
      <ErrorMessage error={error||preview.error}/>
      <footer className="quick-footer"><input ref={receiptRef} type="file" hidden tabIndex={-1} aria-label={t('Attach receipt')} onChange={async e=>{const file=e.target.files?.[0];if(!file)return;try{update('receipt',await readReceipt(file));setError(undefined)}catch(err){setError(err)}}}/><Button type="button" variant="outline" onClick={()=>receiptRef.current?.click()}><Paperclip data-icon="inline-start"/>{form.receipt?.name||t('Attach receipt')}</Button>
        {form.receipt&&<Button type="button" variant="ghost" aria-label={t('Remove split')} onClick={()=>update('receipt',null)}><Trash2/></Button>}
        <div className="actions"><Button type="button" variant="outline" disabled={busy} onClick={()=>void save(true)}>{t('Save & add another')}</Button><Button type="submit" disabled={busy}>{t(busy?'Working…':'Save transaction')}</Button></div>
      </footer>
    </Form>
  </div>
}
