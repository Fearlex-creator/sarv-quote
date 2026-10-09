import type {Settings} from './commands'

const places:Record<string,number>={USD:2,EUR:2,GBP:2,IRR:0,JPY:0,AED:2,KWD:3}
const symbols:Record<string,string>={USD:'$',EUR:'€',GBP:'£',IRR:'IRR',JPY:'¥',AED:'AED',KWD:'KWD'}
const persian='۰۱۲۳۴۵۶۷۸۹'
export const digits=(text:string,settings:Pick<Settings,'digits'>)=>settings.digits==='persian'?text.replace(/\d/g,n=>persian[Number(n)]):text

/** Display only: split exact strings. No arithmetic, rounding, or float conversion. */
export function decimalInput(minor:string,currency:string):string {
  const negative=minor.startsWith('-');const precision=places[currency]??0;
  const raw=minor.replace(/^-/,'').padStart(precision+1,'0')
  return `${negative?'-':''}${precision?`${raw.slice(0,-precision)}.${raw.slice(-precision)}`:raw}`
}
export function money(minor:string,currency:string,settings:Pick<Settings,'digits'>,signed=false):string {
  const decimal=decimalInput(minor,currency);const negative=decimal.startsWith('-');const [whole,fraction]=decimal.replace(/^-/,'').split('.')
  const grouped=whole.replace(/\B(?=(\d{3})+(?!\d))/g,',')
  const value=`${negative?'−':signed&&minor!=='0'?'+':''}${symbols[currency]??currency}${currency==='IRR'||currency==='AED'||currency==='KWD'?' ':''}${grouped}${fraction!==undefined?`.${fraction}`:''}`
  return digits(settings.digits==='persian'?value.replaceAll(',','٬').replaceAll('.','٫'):value,settings)
}
export function date(iso:string,settings:Pick<Settings,'calendar'|'digits'|'language'|'dateFormat'>):string {
  if (!iso) return '—'
  const value=new Date(`${iso.slice(0,10)}T12:00:00Z`)
  if (Number.isNaN(value.getTime())) return iso
  if(settings.calendar==='gregorian'&&settings.dateFormat==='iso')return digits(iso.slice(0,10),settings)
  return new Intl.DateTimeFormat(settings.language==='fa'?'fa-IR':'en-US',{
    calendar:settings.calendar==='jalali'?'persian':'gregory', numberingSystem:settings.digits==='persian'?'arabext':'latn',
    timeZone:'UTC',year:'numeric',month:settings.dateFormat==='short'?'2-digit':'short',day:'2-digit',
  }).format(value)
}
export function today():string {
  const d=new Date();return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`
}

export function monthLabel(iso:string,settings:Pick<Settings,'calendar'|'digits'|'language'>):string {
  return new Intl.DateTimeFormat(settings.language==='fa'?'fa-IR':'en-US',{
    calendar:settings.calendar==='jalali'?'persian':'gregory',numberingSystem:settings.digits==='persian'?'arabext':'latn',
    timeZone:'UTC',year:'numeric',month:'short',
  }).format(new Date(`${iso.slice(0,10)}T12:00:00Z`))
}
