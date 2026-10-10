import {invoke,isTauri} from '@tauri-apps/api/core'
export type Resource={id:string;name:string;customer?:boolean;supplier?:boolean;sku?:string;base_unit_id?:string}
export type Account={id:string;code:string;name:string;class:string;control:boolean;archived:boolean;wallet:boolean;balance:string}
export type Workspace={workspaceId:string;workspaceChoices:Resource[];generation:number;ready:boolean;existing:boolean;profile:'individual'|'service'|'trading';language:'fa'|'en';calendar:'Jalali'|'Gregorian';theme:string;unit:'Rial'|'Toman';name:string;companyId:string;companies:Resource[];today:string;fiscalStart:string;fiscalEnd:string;accounts:Account[];resources:{parties:Resource[];items:Resource[];units:Resource[];warehouses:Resource[]}|null;cash:string;income:string;expense:string;profit:string;trend:{month:string;income:string;expense:string;incomePlot:number;expensePlot:number}[];periods:{id:string;name:string;start_date:string;end_date:string;locked:boolean}[];settings:{business_phone:string;business_address:string};counts:{products:number;contacts:number;quotes:number;journals:number}}
export type Row={id:string;kind:string;name:string;reference:string;date?:string;detail?:string;amount?:string;balance?:string|null;status:string;payable?:string;aging?:string[]}
export type Page={total:number;rows:Row[]}
export type DocumentDetail={data:unknown;reversible?:boolean;lines?:{name:string;debit:string;credit:string;description:string}[];returnable?:{id?:string;bill_line_id?:string;item?:Resource;item_id?:string;quantity_thousandths?:string;remaining_quantity_thousandths?:string}[];history?:unknown[]}
export async function request<T>(mutation:boolean,action:string,payload:unknown={}):Promise<T>{
 if(isTauri())return invoke<T>(mutation?'desktop_mutate':'desktop_view',{action,payload}).catch(e=>{throw new Error(String(e))})
 const token=import.meta.env.VITE_ACCOUNTING_PREVIEW_TOKEN
 if(!import.meta.env.DEV||!token)throw new Error('Open Sarv with pnpm tauri dev; the browser requires pnpm preview:dev.')
 const response=await fetch('/api/accounting',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({mutation,action,payload,token})})
 const result=await response.json() as {data:T;error?:string};if(!response.ok||result.error)throw new Error(result.error||response.statusText);return result.data
}
export function amount(rials:string,unit:Workspace['unit'],language:string){
 const n=BigInt(rials||'0'),negative=n<0n,v=negative?-n:n;const divisor=unit==='Toman'?10n:1n
 const whole=new Intl.NumberFormat(language==='fa'?'fa-IR':'en-US').format(v/divisor)
 const fraction=unit==='Toman'&&v%10n!==0n?`${language==='fa'?'٫':'.'}${new Intl.NumberFormat(language==='fa'?'fa-IR':'en-US').format(v%10n)}`:''
 return `${negative?'−':''}${whole}${fraction} ${language==='fa'?(unit==='Toman'?'تومان':'ریال'):(unit==='Toman'?'Toman':'IRR')}`
}
export function shownDate(value:string|undefined,w:Pick<Workspace,'language'|'calendar'>){if(!value)return '—';const d=new Date(`${value.slice(0,10)}T12:00:00`);if(Number.isNaN(d.getTime()))return value;return new Intl.DateTimeFormat(w.language==='fa'?'fa-IR':'en-US',{calendar:w.calendar==='Jalali'?'persian':'gregory',year:'numeric',month:'short',day:'numeric'}).format(d)}
export async function download(module:string,format='csv') {const file=await request<{name:string;mime:string;content:string}>(false,'export',{module,format});const url=URL.createObjectURL(new Blob([file.content],{type:file.mime}));const a=document.createElement('a');a.href=url;a.download=file.name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)}
