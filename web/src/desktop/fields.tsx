import {useId,useState} from 'react'
import {Input} from '@/components/ui/input'
import {Field,FieldLabel,FieldDescription} from '@/components/ui/field'
import {Select,SelectTrigger,SelectValue,SelectContent,SelectGroup,SelectItem} from '@/components/ui/select'
import {Popover,PopoverTrigger,PopoverContent} from '@/components/ui/popover'
import {Button} from '@/components/ui/button'
import {Calendar} from '@/components/ui/calendar'
import {CalendarDays} from 'lucide-react'
import {useWorkspace,useText} from './context'
export function TextField({label,value,onChange,required=false,description,inputMode}:{label:string;value:string;onChange:(s:string)=>void;required?:boolean;description?:string;inputMode?:'decimal'|'numeric'}){const id=useId();return <Field><FieldLabel htmlFor={id}>{label}</FieldLabel><Input className="min-h-11" id={id} value={value} onChange={e=>onChange(e.target.value)} required={required} inputMode={inputMode}/>{description&&<FieldDescription>{description}</FieldDescription>}</Field>}
export function ChoiceField({label,value,onChange,options}:{label:string;value:string;onChange:(s:string)=>void;options:{id:string;name:string}[]}){const id=useId();return <Field><FieldLabel htmlFor={id}>{label}</FieldLabel><Select value={value||null} onValueChange={v=>onChange(v||'')} items={options.map(o=>({value:o.id,label:o.name}))}><SelectTrigger id={id} className="w-full min-h-11"><SelectValue placeholder={options.find(o=>o.id==='')?.name||'—'}/></SelectTrigger><SelectContent><SelectGroup>{options.map(o=><SelectItem key={o.id} value={o.id}>{o.name}</SelectItem>)}</SelectGroup></SelectContent></Select></Field>}
export function DateField({label,value,onChange}:{label:string;value:string;onChange:(s:string)=>void}){
 const w=useWorkspace(),t=useText(),[open,setOpen]=useState(false),id=useId();const calendar=w.calendar==='Jalali'?'persian':'gregory';
 const choose=(d:Date|undefined)=>{if(!d)return;const parts=new Intl.DateTimeFormat(`en-US-u-ca-${calendar}`,{year:'numeric',month:'2-digit',day:'2-digit'}).formatToParts(d);const get=(k:string)=>parts.find(p=>p.type===k)?.value||'';onChange(`${get('year')}/${get('month')}/${get('day')}`);setOpen(false)}
 return <Field><FieldLabel htmlFor={id}>{label}</FieldLabel><div className="flex gap-2"><Input className="min-h-11" id={id} value={value} onChange={e=>onChange(e.target.value)} inputMode="numeric" required placeholder={w.calendar==='Jalali'?'۱۴۰۵/۰۱/۰۱':'2026/03/21'}/><Popover open={open} onOpenChange={setOpen}><PopoverTrigger render={<Button className="min-h-11" variant="outline" size="icon" aria-label={t('انتخاب تاریخ','Choose date')}/> }><CalendarDays/></PopoverTrigger><PopoverContent dir={w.language==='fa'?'rtl':'ltr'} className="w-auto p-0"><Calendar persian={w.calendar==='Jalali'} dir={w.language==='fa'?'rtl':'ltr'} mode="single" onSelect={choose}/></PopoverContent></Popover></div><FieldDescription>{t('ورودی و انتخاب تاریخ','Date input and picker')} · {w.calendar==='Jalali'?t('شمسی','Jalali'):t('میلادی','Gregorian')}</FieldDescription></Field>
}
