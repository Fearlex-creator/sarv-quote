import {createContext,useContext,useEffect,useState,type ReactNode} from 'react'
import {useQuery,useQueryClient} from '@tanstack/react-query'
import {commands,type Workspace} from '@/lib/commands'
import {translate} from './i18n'

type Context={data:Workspace; t:(s:string)=>string; run:<T>(job:()=>Promise<T>,message?:string)=>Promise<T>; notice:string; busy:boolean}
const AccountingContext=createContext<Context|null>(null)
export function AccountingProvider({children}:{children:ReactNode}){
  const client=useQueryClient();const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false)
  const query=useQuery({queryKey:['workspace'],queryFn:()=>commands.getWorkspace(),staleTime:30_000})
  const s=query.data?.settings
  useEffect(()=>{
    if(!s)return
    const media=window.matchMedia('(prefers-color-scheme: dark)')
    const apply=()=>{document.documentElement.classList.toggle('dark',s.theme==='dark'||s.theme==='system'&&media.matches)}
    apply();media.addEventListener('change',apply)
    document.documentElement.dir=s.direction;document.documentElement.lang=s.language
    document.title=s.language==='fa'?'سرو — حسابداری':'Sarv — Accounting'
    document.documentElement.style.setProperty('--focus',s.accent)
    return()=>media.removeEventListener('change',apply)
  },[s])
  if(query.isPending)return <div className="startup"><img className="startup-mark" src="/favicon.svg" alt=""/><p>Loading your workspace…</p></div>
  if(query.isError||!query.data)return <div className="startup"><h1>Sarv</h1><p role="alert">{query.error?.message}</p><button onClick={()=>void query.refetch()}>Retry</button></div>
  const t=(text:string)=>translate(text,query.data.settings.language)
  async function run<T>(job:()=>Promise<T>,message='Saved'):Promise<T>{
    setBusy(true)
    try{const result=await job();await Promise.all([client.invalidateQueries({queryKey:['workspace']}),client.invalidateQueries({queryKey:['ledger']}),client.invalidateQueries({queryKey:['detail']}),client.invalidateQueries({queryKey:['dashboard-range']})]);setNotice(t(message));return result}
    catch(error){setNotice(error instanceof Error?error.message:String(error));throw error}
    finally{setBusy(false)}
  }
  return <AccountingContext.Provider value={{data:query.data,t,run,notice,busy}}>{children}<div className="notification" role="status" aria-live="polite">{notice&&<><span>{notice}</span><button onClick={()=>setNotice('')} aria-label={t('Close')}>×</button></>}</div></AccountingContext.Provider>
}
export function useAccounting(){const context=useContext(AccountingContext);if(!context)throw new Error('AccountingProvider missing');return context}
