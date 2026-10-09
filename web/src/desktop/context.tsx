import {createContext,useContext,useEffect,type ReactNode} from 'react'
import {useQuery,useQueryClient,useMutation} from '@tanstack/react-query'
import {request,amount,type Workspace} from './api'
import {toast} from 'sonner'
const Context=createContext<Workspace|null>(null)
export function useWorkspace(){const w=useContext(Context);if(!w)throw new Error('Workspace provider missing');return w}
export function useText(){const w=useWorkspace();return (fa:string,en:string)=>w.language==='fa'?fa:en}
export function useMoney(){const w=useWorkspace();return (v:string)=>amount(v,w.unit,w.language)}
export function useWrite(){const client=useQueryClient();return useMutation({mutationFn:({action,payload}:{action:string;payload:unknown})=>request<Workspace>(true,action,payload),onSuccess:w=>{client.setQueryData(['sarv','workspace'],w);void client.invalidateQueries({queryKey:['sarv','rows']});void client.invalidateQueries({queryKey:['sarv','detail']});toast.success(w.language==='fa'?'ذخیره شد':'Saved')},onError:e=>toast.error(e.message)})}
export function WorkspaceProvider({children}:{children:ReactNode}){
 const query=useQuery({queryKey:['sarv','workspace'],queryFn:()=>request<Workspace>(false,'workspace'),staleTime:15_000})
 useEffect(()=>{if(!query.data)return;const w=query.data;document.documentElement.lang=w.language;document.documentElement.dir=w.language==='fa'?'rtl':'ltr';document.title=w.language==='fa'?'سرو | حسابداری':'Sarv | Accounting';document.documentElement.classList.toggle('dark',w.theme==='dark'||(w.theme==='system'&&matchMedia('(prefers-color-scheme: dark)').matches))},[query.data])
 if(query.isPending)return <div role="status" className="p-8">در حال باز کردن سرو…</div>
 if(query.isError)return <div role="alert" className="p-8">{query.error.message}</div>
 return <Context.Provider value={query.data}>{children}</Context.Provider>
}
