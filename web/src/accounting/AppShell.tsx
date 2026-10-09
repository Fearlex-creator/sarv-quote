import {useEffect,useRef,useState} from 'react'
import {LayoutDashboard,Rows3,Wallet,UsersRound,Settings2,Plus,Search,PanelLeftClose,PanelLeftOpen,ArrowRight} from 'lucide-react'
import {Dialog,DialogContent,DialogTitle,DialogDescription} from '@/components/ui/dialog'
import {Sheet,SheetContent,SheetHeader,SheetTitle,SheetDescription} from '@/components/ui/sheet'
import type {Filters,TransactionInput} from '@/lib/commands'
import {useAccounting} from './context'
import {Button,Input} from './common'
import {Dashboard} from './Dashboard'
import {TransactionLedger} from './TransactionLedger'
import {QuickAddTransaction} from './QuickAddTransaction'
import {AccountsWallets} from './AccountsWallets'
import {Contacts} from './Contacts'
import {Settings} from './Settings'

type Screen='dashboard'|'ledger'|'quick'|'accounts'|'contacts'|'settings'

export function AppShell(){
  const {data,t}=useAccounting();const s=data.settings
  const [active,setActive]=useState<Screen>('dashboard')
  const [quickOpen,setQuickOpen]=useState(false);const [palette,setPalette]=useState(false)
  const [paletteSearch,setPaletteSearch]=useState('');const [collapsed,setCollapsed]=useState(false)
  const [initial,setInitial]=useState<Partial<TransactionInput>>();const [quickKey,setQuickKey]=useState(0)
  const [ledgerFilters,setLedgerFilters]=useState<Filters>({});const mainRef=useRef<HTMLElement>(null)
  const shortcut=/Mac|iPhone|iPad/.test(navigator.userAgent)?'⌘':'Ctrl'
  const nav=[{id:'dashboard',label:'Overview',icon:LayoutDashboard},{id:'ledger',label:s.mode==='business'?'General ledger':'Transactions',icon:Rows3},{id:'quick',label:'New entry',icon:Plus},{id:'accounts',label:'Accounts',icon:Wallet},{id:'contacts',label:'Contacts',icon:UsersRound},{id:'settings',label:'Settings',icon:Settings2}] as const
  const openQuick=(input?:Partial<TransactionInput>)=>{setInitial(input);setQuickKey(key=>key+1);setQuickOpen(true)}
  const navigate=(screen:Screen)=>{setActive(screen);setPalette(false);mainRef.current?.scrollTo({top:0});mainRef.current?.focus()}
  const openLedger=(filters:Filters={})=>{setLedgerFilters(filters);navigate('ledger')}
  useEffect(()=>{
    const handle=(event:KeyboardEvent)=>{
      if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='k'){event.preventDefault();setPaletteSearch('');setPalette(p=>!p)}
      if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='n'){event.preventDefault();setInitial(undefined);setQuickKey(k=>k+1);setQuickOpen(true)}
    }
    window.addEventListener('keydown',handle);return()=>window.removeEventListener('keydown',handle)
  },[])
  const results=nav.filter(n=>t(n.label).toLowerCase().includes(paletteSearch.toLowerCase()))
  return <div className={`app-shell ${collapsed?'sidebar-collapsed':''}`}>
    <a className="skip-link" href="#accounting-main">{t('Skip to content')}</a>
    <aside className="app-sidebar">
      <a className="app-brand" href="#dashboard" onClick={e=>{e.preventDefault();navigate('dashboard')}}><img className="brand-mark" src="/sarv-mark.svg" alt=""/><span>{t('Sarv')}<small>{t('Accounting')}</small></span></a>
      <button className="workspace-switch" onClick={()=>navigate('settings')} aria-label={t('Workspace settings')}><span className="workspace-avatar" aria-hidden="true">{s.logo?<img src={s.logo} alt=""/>:s.profile[0]}</span><span><strong>{s.profile}</strong><small>{t(s.mode==='business'?'Business workspace':'Personal workspace')}</small></span></button>
      <div className="nav-label">{t('WORKSPACE')}</div>
      <nav className="app-nav" aria-label={t('Navigate to')}>{nav.filter(n=>n.id!=='settings'&&n.id!=='quick').map(n=><button key={n.id} aria-current={active===n.id?'page':undefined} title={t(n.label)} onClick={()=>navigate(n.id)}><n.icon aria-hidden="true"/><span>{t(n.label)}</span>{n.id==='ledger'&&<span className="nav-count">{data.transactionCount.toLocaleString(s.language==='fa'?'fa-IR':'en-US')}</span>}</button>)}</nav>
      <button className="sidebar-entry" aria-current={active==='quick'?'page':undefined} title={t('New entry')} onClick={()=>navigate('quick')}><Plus aria-hidden="true"/><span>{t('New entry')}</span><kbd>{shortcut} N</kbd></button>
      <div className="sidebar-bottom"><nav className="app-nav" aria-label={t('Settings')}><button aria-current={active==='settings'?'page':undefined} onClick={()=>navigate('settings')}><Settings2 aria-hidden="true"/><span>{t('Settings')}</span></button></nav><div className="local-workspace"><span className="status-dot" aria-hidden="true"/><div><strong>{t('Local data')}</strong><small>{t('Saved on this device')}</small></div></div></div>
    </aside>
    <div className="app-content">
      <header className="app-header"><div className="header-left"><Button variant="ghost" size="icon" aria-label={t(collapsed?'Expand sidebar':'Collapse sidebar')} onClick={()=>setCollapsed(v=>!v)}>{collapsed?<PanelLeftOpen/>:<PanelLeftClose/>}</Button><span className="breadcrumb">{t('Workspace')} <span>/</span> <strong>{t(nav.find(n=>n.id===active)!.label)}</strong></span></div><div className="header-right"><button className="global-search" aria-label={t('Open command palette')} onClick={()=>{setPaletteSearch('');setPalette(true)}}><Search aria-hidden="true"/><span>{t('Go to a screen…')}</span><kbd>{shortcut} K</kbd></button><Button onClick={()=>openQuick()} className="header-new"><Plus data-icon="inline-start"/><span>{t('New transaction')}</span></Button></div></header>
      <main ref={mainRef} tabIndex={-1} id="accounting-main" className="app-main">
        {active==='dashboard'&&<Dashboard onQuickAdd={()=>openQuick()} onLedger={openLedger} onAccounts={()=>navigate('accounts')} onContacts={()=>navigate('contacts')}/>}
        {active==='ledger'&&<TransactionLedger key={JSON.stringify(ledgerFilters)} initialFilters={ledgerFilters} onQuickAdd={()=>openQuick()} onCorrect={openQuick}/>}
        {active==='quick'&&<div className="screen"><QuickAddTransaction onDone={()=>openLedger()}/></div>}
        {active==='accounts'&&<AccountsWallets onLedger={openLedger} onTransfer={accountId=>openQuick({kind:'transfer',accountId,payee:t('Transfer')})}/>}
        {active==='contacts'&&<Contacts onLedger={openLedger}/>}
        {active==='settings'&&<Settings/>}
        <footer className="app-footer"><span>{t('Sarv')} · {t('Accounting')}</span><span>{t('Stored locally')}</span></footer>
      </main>
    </div>
    <Sheet open={quickOpen} onOpenChange={setQuickOpen}><SheetContent className="quick-sheet" side={s.direction==='rtl'?'left':'right'}><SheetHeader><SheetTitle>{t(initial?.correctionId?'Correct transaction':'New entry')}</SheetTitle><SheetDescription>{t('Record income, an expense, or an account transfer.')}</SheetDescription></SheetHeader><QuickAddTransaction key={quickKey} embedded initial={initial} onDone={()=>{setQuickOpen(false);openLedger()}}/></SheetContent></Sheet>
    <Dialog open={palette} onOpenChange={setPalette}><DialogContent className="command-dialog"><DialogTitle>{t('Command palette')}</DialogTitle><DialogDescription>{t('Navigate to a screen or add a transaction.')}</DialogDescription><div className="search-field"><Search aria-hidden="true"/><Input autoFocus aria-label={t('Go to a screen…')} placeholder={t('Go to a screen…')} value={paletteSearch} onChange={e=>setPaletteSearch(e.target.value)} onKeyDown={e=>{if(e.key==='ArrowDown'){e.preventDefault();e.currentTarget.closest('[data-slot="dialog-content"]')?.querySelector<HTMLButtonElement>('.command-list button')?.focus()}}}/></div><div className="command-list" onKeyDown={e=>{if(!['ArrowDown','ArrowUp'].includes(e.key))return;e.preventDefault();const buttons=Array.from(e.currentTarget.querySelectorAll('button'));const index=buttons.indexOf(document.activeElement as HTMLButtonElement);buttons[(index+(e.key==='ArrowDown'?1:buttons.length-1))%buttons.length]?.focus()}}>{results.map(n=><button key={n.id} onClick={()=>navigate(n.id)}><n.icon aria-hidden="true"/><span>{t(n.label)}</span><ArrowRight aria-hidden="true"/></button>)}{!results.length&&<p>{t('No results')}</p>}</div></DialogContent></Dialog>
  </div>
}
