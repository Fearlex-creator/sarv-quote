import {useEffect,useMemo,useRef,useState} from 'react'
import {useQueries,useQuery} from '@tanstack/react-query'
import {useVirtualizer} from '@tanstack/react-virtual'
import {Download,Plus,Search,SlidersHorizontal,FileDown,Bookmark,ArrowUpRight,Pencil} from 'lucide-react'
import {Sheet,SheetContent,SheetHeader,SheetTitle,SheetDescription} from '@/components/ui/sheet'
import {Dialog,DialogContent,DialogTitle,DialogDescription} from '@/components/ui/dialog'
import {commands,downloadFile,type Filters,type LedgerRow,type TransactionInput} from '@/lib/commands'
import {date,money,today} from '@/lib/exact-format'
import {useAccounting} from './context'
import {Form,Button,Input,Select,Field,PageHeading,Empty,ErrorMessage,Future} from './common'

const SIZE=150
export function TransactionLedger({onQuickAdd,onCorrect,initialFilters={}}:{onQuickAdd:()=>void;onCorrect:(input:TransactionInput)=>void;initialFilters?:Filters}){
  const {data,t,run,busy}=useAccounting();const s=data.settings
  const [filters,setFilters]=useState<Filters>({...initialFilters,currency:initialFilters.currency||s.currency})
  const [search,setSearch]=useState('');const [selected,setSelected]=useState<string[]>([]);const [detail,setDetail]=useState<string|null>(null)
  const [viewName,setViewName]=useState('');const [viewOpen,setViewOpen]=useState(false);const [bulkOpen,setBulkOpen]=useState(false);const [bulkCategory,setBulkCategory]=useState('');const [error,setError]=useState<unknown>()
  const [inline,setInline]=useState<{id:string;categoryId:string}|null>(null)
  const scroller=useRef<HTMLDivElement>(null)
  useEffect(()=>{const timer=setTimeout(()=>setFilters(f=>({...f,search})),250);return()=>clearTimeout(timer)},[search])
  useEffect(()=>{setFilters(f=>({...f,currency:s.currency}));setSelected([])},[s.currency])
  const first=useQuery({queryKey:['ledger',filters,0],queryFn:()=>commands.listTransactions({...filters,offset:0,limit:SIZE}),staleTime:30_000})
  const total=first.data?.total??0
  // eslint-disable-next-line react/incompatible-library -- TanStack Virtual is intentionally outside React Compiler; this project uses ordinary Vite React.
  const virtual=useVirtualizer({count:total,getScrollElement:()=>scroller.current,estimateSize:()=>58,overscan:8})
  const items=virtual.getVirtualItems()
  const offsets=Array.from(new Set(items.map(v=>Math.floor(v.index/SIZE)*SIZE))).filter(n=>n!==0)
  const pageQueries=useQueries({queries:offsets.map(offset=>({queryKey:['ledger',filters,offset],queryFn:()=>commands.listTransactions({...filters,offset,limit:SIZE}),staleTime:30_000}))})
  const loaded=new Map<number,LedgerRow>()
  first.data?.rows.forEach((r,i)=>loaded.set(i,r));pageQueries.forEach(q=>q.data?.rows.forEach((r,i)=>loaded.set(q.data!.offset+i,r)))
  const setFilter=(key:keyof Filters,value:string)=>{setFilters(f=>({...f,[key]:value}));setSelected([]);scroller.current?.scrollTo({top:0})}
  const exportData=async(scope:string)=>{setError(undefined);try{downloadFile(await commands.exportData(scope,filters))}catch(err){setError(err)}}
  const categorize=async(ids:string[],categoryId:string)=>{
    try{await run(()=>commands.mutate('categorize',{ids,categoryId,date:today()}));setSelected([]);setBulkOpen(false);setInline(null);setError(undefined)}catch(err){setError(err)}
  }
  const activeFilters=Object.entries(filters).filter(([key,value])=>value&&key!=='currency'&&key!=='search')
  return <div className="screen ledger-screen">
    <PageHeading title={t(s.mode==='business'?'General ledger':'Transactions')} description={t('Search, review, and manage your financial entries.')} actions={<><Button variant="outline" onClick={()=>void exportData('ledger')}><Download data-icon="inline-start"/>{t('Export CSV')}</Button><Button onClick={onQuickAdd}><Plus data-icon="inline-start"/>{t('New transaction')}</Button></>}/>
    <section className="panel ledger-panel" aria-label={t('All transactions')}>
      <div className="ledger-tools"><div className="search-field"><Search aria-hidden="true"/><Input aria-label={t('Search payee, memo, amount, or tag…')} placeholder={t('Search payee, memo, amount, or tag…')} value={search} onChange={e=>setSearch(e.target.value)}/></div>
        <label className="date-filter"><span>{t('From')}</span><Input aria-label={t('From')} type="date" value={filters.from||''} onChange={e=>setFilter('from',e.target.value)}/></label><label className="date-filter"><span>{t('To')}</span><Input aria-label={t('To')} type="date" value={filters.to||''} onChange={e=>setFilter('to',e.target.value)}/></label>
        <Button variant="ghost" onClick={()=>setViewOpen(true)}><Bookmark data-icon="inline-start"/>{t('Save view')}</Button>
      </div>
      <div className="filter-bar"><SlidersHorizontal aria-hidden="true"/>
        <Select aria-label={t('Account')} value={filters.account||''} onChange={e=>setFilter('account',e.target.value)}><option value="">{t('All accounts')}</option>{data.accounts.map(a=><option key={a.id} value={a.id}>{a.name}</option>)}</Select>
        <Select aria-label={t('Category')} value={filters.category||''} onChange={e=>setFilter('category',e.target.value)}><option value="">{t('All categories')}</option>{data.categories.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select>
        <Select aria-label={t('Contact')} value={filters.contact||''} onChange={e=>setFilter('contact',e.target.value)}><option value="">{t('All contacts')}</option>{data.contacts.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select>
        <Select aria-label={t('All statuses')} value={filters.status||''} onChange={e=>setFilter('status',e.target.value)}><option value="">{t('All statuses')}</option>{['posted','reversed','reversal'].map(status=><option key={status} value={status}>{t(status==='posted'?'Posted':status==='reversed'?'Reversed':'Reversal')}</option>)}</Select>
        <Select aria-label={t('Currency')} value={filters.currency||s.currency} onChange={e=>setFilter('currency',e.target.value)}>{data.currencies.map(c=><option key={c.code} value={c.code}>{c.code}</option>)}</Select>
        <Select aria-label={t('Saved views')} value="" onChange={e=>{const view=data.savedViews.find(v=>v.id===e.target.value);if(view){setFilters(view.filters);setSearch(view.filters.search||'');setSelected([])}}}><option value="">{t('Saved views')}</option>{data.savedViews.map(v=><option key={v.id} value={v.id}>{v.name}</option>)}</Select>
        <Button variant="ghost" size="icon" aria-label={t('Export PDF')} onClick={()=>void exportData('ledger_pdf')}><FileDown/></Button>
      </div>
      {activeFilters.length>0&&<div className="filter-chips">{activeFilters.map(([key,value])=><Button key={key} variant="secondary" onClick={()=>setFilter(key as keyof Filters,'')}>{t(key==='account'?'Account':key==='contact'?'Contact':key==='category'?'Category':key)}: {data.accounts.find(a=>a.id===value)?.name||data.contacts.find(c=>c.id===value)?.name||data.categories.find(c=>c.id===value)?.name||String(value)} <span aria-hidden="true">×</span></Button>)}<Button variant="ghost" onClick={()=>{setFilters({currency:s.currency});setSearch('')}}>{t('Clear filters')}</Button></div>}
      {selected.length>0&&<div className="bulk-bar"><strong>{selected.length} {t('selected')}</strong><Button variant="outline" onClick={()=>setBulkOpen(true)}>{t('Bulk categorize')}</Button><Button variant="outline" disabled={selected.length<2||busy} onClick={async()=>{try{await run(()=>commands.mutate('merge',{ids:selected,date:today()}));setSelected([]);setError(undefined)}catch(err){setError(err)}}}>{t('Merge duplicates')}</Button><Button variant="ghost" onClick={()=>setSelected([])}>{t('Cancel')}</Button></div>}
      <ErrorMessage error={error||first.error||pageQueries.find(q=>q.error)?.error}/>
      {first.isPending?<div className="loading-state">{t('Working…')}</div>:total===0?<Empty title={t('No transactions match your filters.')} action={<Button onClick={onQuickAdd}>{t('New transaction')}</Button>}/>:<div ref={scroller} className="ledger-scroll" tabIndex={0} aria-label={t('All transactions')}>
        <table className="ledger-table" aria-rowcount={total+1}><thead><tr><th><input type="checkbox" aria-label={t('Select all loaded')} checked={selected.length>0&&Array.from(loaded.values()).filter(r=>r.status==='posted'&&['expense','income','transfer','payment'].includes(r.kind)).every(r=>selected.includes(r.id))} onChange={e=>setSelected(e.target.checked?Array.from(loaded.values()).filter(r=>r.status==='posted'&&['expense','income','transfer','payment'].includes(r.kind)).slice(0,200).map(r=>r.id):[])}/></th>{['Date','Payee','Category','Account','Amount','Running balance'].map(label=><th key={label} scope="col">{t(label)}</th>)}</tr></thead>
          <tbody style={{height:virtual.getTotalSize()}}>{items.map(item=>{
            const row=loaded.get(item.index)
            if(!row)return <tr key={item.index} aria-rowindex={item.index+2} className="virtual-row" style={{transform:`translateY(${item.start}px)`}}><td colSpan={7}>{t('Working…')}</td></tr>
            const editable=row.status==='posted'&&['income','expense','transfer','payment'].includes(row.kind)
            return <tr key={row.id} className={`virtual-row ${row.status!=='posted'?'row-reversed':''}`} aria-rowindex={item.index+2} style={{transform:`translateY(${item.start}px)`}}>
              <td><input type="checkbox" disabled={!editable} aria-label={`${t('Select transaction')} ${row.payee}`} checked={selected.includes(row.id)} onChange={e=>setSelected(v=>e.target.checked?[...v,row.id]:v.filter(id=>id!==row.id))}/></td>
              <td className="table-date">{date(row.date,s)}</td><td><button className="payee-link" onClick={()=>setDetail(row.id)}>{row.payee}<span className="row-subtitle">{row.memo||t(row.status==='reversed'?'Reversed':row.kind==='reversal'?'Reversal':'Posted')}</span></button></td>
              <td>{inline?.id===row.id?<Select autoFocus aria-label={t('Category')} value={inline.categoryId} onChange={e=>void categorize([row.id],e.target.value)}><option value="">{t('Choose a category')}</option>{data.categories.filter(c=>c.kind===row.kind).map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select>:<button className="inline-category" disabled={!editable||!['income','expense'].includes(row.kind)||row.category.includes(',')} onClick={()=>setInline({id:row.id,categoryId:''})}>{row.category}{editable&&['income','expense'].includes(row.kind)&&<Pencil aria-hidden="true"/>}</button>}</td>
              <td><span className="account-tag">{row.account}</span></td><td className={`numeric ${row.amountMinor.startsWith('-')?'negative':'positive'}`}>{money(row.amountMinor,row.currency,s,true)}</td><td className="numeric">{money(row.balanceMinor,row.currency,s)}</td>
            </tr>
          })}</tbody>
        </table>
      </div>}
      <footer className="table-footer"><span>{total.toLocaleString(s.language==='fa'?'fa-IR':'en-US')} {t('transactions')} · {filters.currency||s.currency}</span><span>{t('Running balances include all entries, even when filtered.')}</span></footer>
    </section>
    <div className="ledger-connections">{['Invoices','Bills','Cheques','Reconciliation'].map(label=><span key={label}>{t(label)} <ArrowUpRight aria-hidden="true"/> <small>{t('Available in Phase 2')}</small></span>)}</div>
    <Dialog open={viewOpen} onOpenChange={setViewOpen}><DialogContent><DialogTitle>{t('Save this view')}</DialogTitle><DialogDescription>{t('Keep these filters for next time.')}</DialogDescription><Form className="form-stack" onSubmit={async e=>{e.preventDefault();try{await run(()=>commands.mutate('saved_view',{name:viewName,filters}));setViewOpen(false);setViewName('')}catch(err){setError(err)}}}><Field label={t('View name')}><Input required maxLength={100} value={viewName} onChange={e=>setViewName(e.target.value)}/></Field><Button type="submit" disabled={busy}>{t('Save')}</Button><ErrorMessage error={error}/></Form></DialogContent></Dialog>
    <Dialog open={bulkOpen} onOpenChange={setBulkOpen}><DialogContent><DialogTitle>{t('Bulk categorize')}</DialogTitle><DialogDescription>{t('Corrections reverse the original and post a new entry.')}</DialogDescription><Select aria-label={t('Category')} value={bulkCategory} onChange={e=>setBulkCategory(e.target.value)}><option value="">{t('Choose a category')}</option>{data.categories.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</Select><Button disabled={!bulkCategory||busy} onClick={()=>void categorize(selected,bulkCategory)}>{t('Save')}</Button><ErrorMessage error={error}/></DialogContent></Dialog>
    <TransactionDetail key={detail||'closed'} id={detail} onClose={()=>setDetail(null)} onCorrect={input=>{setDetail(null);onCorrect(input)}}/>
  </div>
}

export function TransactionDetail({id,onClose,onCorrect}:{id:string|null;onClose:()=>void;onCorrect:(input:TransactionInput)=>void}){
  const {data,t,run,busy}=useAccounting();const s=data.settings;const [tab,setTab]=useState('details');const [reason,setReason]=useState('');const [reverseOpen,setReverseOpen]=useState(false);const [error,setError]=useState<unknown>()
  const query=useQuery({queryKey:['detail',id],queryFn:()=>commands.getTransaction(id!),enabled:!!id})
  const d=query.data
  const canEdit=d&&!d.reversedBy&&['income','expense','transfer'].includes(d.input.kind)
  const title=d?.input.payee||t('Transaction details')
  const formattedLines=useMemo(()=>d?.lines.map(l=>({...l,label:l.category||t(l.kind==='payable'?'Payables':l.kind==='receivable'?'Receivables':l.account)})),[d,t])
  return <Sheet open={!!id} onOpenChange={open=>{if(!open)onClose()}}><SheetContent className="detail-sheet" side={s.direction==='rtl'?'left':'right'}><SheetHeader><SheetTitle>{title}</SheetTitle><SheetDescription>{d?date(d.input.date,s):t('Working…')}</SheetDescription></SheetHeader>
    <div className="drawer-body"><ErrorMessage error={query.error||error}/>{d&&<>
      <div className="detail-amount">{money(d.amountMinor,d.currency,s)}<small>{t(d.input.kind==='income'?'Income':d.input.kind==='expense'?'Expense':d.input.kind==='reversal'?'Reversal':'Transfer')}</small></div>
      <div className="tab-bar" role="tablist" aria-label={t('Transaction details')}>{['details','attachments','history'].map(name=><button key={name} role="tab" id={`detail-tab-${name}`} aria-controls="detail-panel" aria-selected={tab===name} onClick={()=>setTab(name)}>{t(name==='details'?'Details':name==='attachments'?'Attachments':'History')}</button>)}</div>
      <div role="tabpanel" id="detail-panel" aria-labelledby={`detail-tab-${tab}`}>
        {tab==='details'&&<div className="form-stack"><dl className="detail-list"><div><dt>{t('Paid now')}</dt><dd>{money(d.paidMinor,d.currency,s)}</dd></div><div><dt>{t('Notes')}</dt><dd>{d.input.memo||t('No notes')}</dd></div><div><dt>{t('Tags')}</dt><dd>{d.input.tags||'—'}</dd></div></dl>
          <section className="journal-preview"><h3>{t(s.mode==='business'?'Journal lines':'Split categories')}</h3>{formattedLines?.filter(l=>s.mode==='business'||l.category).map((l,i)=><div key={i}><span>{l.label}</span><span>{s.mode==='business'&&`${t(l.debitMinor!=='0'?'Debit':'Credit')} `}{money(l.debitMinor!=='0'?l.debitMinor:l.creditMinor,d.currency,s)}</span></div>)}</section>
          <Future title="Invoices"/>
          {canEdit&&<><Button onClick={()=>onCorrect({...d.input,date:today(),correctionId:d.id,correctionDate:today()})}>{t('Correct transaction')}</Button><Button variant="outline" onClick={()=>setReverseOpen(v=>!v)}>{t('Reverse transaction')}</Button></>}
          {reverseOpen&&<Form className="form-stack" onSubmit={async e=>{e.preventDefault();try{await run(()=>commands.mutate('reverse',{id:d.id,date:today(),reason}),'Entry reversed');setReverseOpen(false)}catch(err){setError(err)}}}><Field label={t('Reason')}><Input required value={reason} onChange={e=>setReason(e.target.value)}/></Field><p className="muted">{t('Corrections reverse the original and post a new entry.')}</p><Button type="submit" disabled={busy}>{t('Reverse entry')}</Button></Form>}
          {d.reversedBy&&<p className="info-note">{t('Reversed')} · <button className="text-button" onClick={()=>void commands.getTransaction(d.reversedBy!).then(r=>setError(`Reversal recorded ${r.recordedAt}`))}>{d.reversedBy.slice(0,8)}</button></p>}
        </div>}
        {tab==='attachments'&&<div className="form-stack">{d.attachments.length?d.attachments.map(a=><Button key={a.id} variant="outline" onClick={async()=>{try{const file=await commands.getAttachment(a.id);downloadFile({name:file.name,mime:file.mime,bytes:file.bytes,content:''})}catch(err){setError(err)}}}><Download data-icon="inline-start"/>{a.name}</Button>):<p className="muted">{t('No attachment')}</p>}</div>}
        {tab==='history'&&<ol className="audit-timeline">{d.history.map(event=><li key={event.id}><strong>{event.action.replaceAll('.',' · ')}</strong><time dateTime={event.at}>{date(event.at,s)} · {new Date(event.at).toLocaleTimeString(s.language)}</time><code>{event.entityId.slice(0,8)}</code></li>)}</ol>}
      </div>
    </>}</div>
  </SheetContent></Sheet>
}
