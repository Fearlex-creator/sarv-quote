import {cloneElement,isValidElement,useId,type ReactNode,type ReactElement,type ComponentProps,type SelectHTMLAttributes} from 'react'
import {Input} from '@/components/ui/input'
import {Button} from '@/components/ui/button'
import {useAccounting} from './context'
import {Inbox} from 'lucide-react'
import {Field as ShadcnField,FieldGroup,FieldLabel,FieldDescription} from '@/components/ui/field'

export {Input,Button}
export function Field({label,hint,children}:{label:string;hint?:string;children:ReactNode}){
  const id=useId()
  const child=isValidElement(children)?cloneElement(children as ReactElement<{id?:string;'aria-describedby'?:string}>,{id,'aria-describedby':hint?`${id}-hint`:undefined}):children
  return <ShadcnField className="form-field"><FieldLabel htmlFor={id}>{label}</FieldLabel>{child}{hint&&<FieldDescription id={`${id}-hint`}>{hint}</FieldDescription>}</ShadcnField>
}
export function Form({children,...props}:ComponentProps<'form'>){return <form {...props}><FieldGroup className="form-stack">{children}</FieldGroup></form>}
export function Select({children,...props}:SelectHTMLAttributes<HTMLSelectElement>){return <select className="native-select" {...props}>{children}</select>}
export function PageHeading({title,description,actions}:{title:string;description?:string;actions?:ReactNode}){
  return <div className="page-heading"><div><h1>{title}</h1>{description&&<p>{description}</p>}</div><div className="actions">{actions}</div></div>
}
export function Panel({title,actions,children,className=''}:{title?:string;actions?:ReactNode;children:ReactNode;className?:string}){
  return <section className={`panel ${className}`}>{title&&<header className="panel-heading"><h2>{title}</h2><div className="actions">{actions}</div></header>}{children}</section>
}
export function Empty({title,description,action}:{title:string;description?:string;action?:ReactNode}){
  return <div className="empty-state"><Inbox aria-hidden="true"/><h2>{title}</h2>{description&&<p>{description}</p>}{action}</div>
}
export function ErrorMessage({error}:{error:unknown}){return error?<p className="form-error" role="alert">{error instanceof Error?error.message:String(error)}</p>:null}
export function Future({title}:{title:string}){
  const {t}=useAccounting();return <div className="future"><h3>{t(title)}</h3><p>{t('Available in Phase 2')}</p></div>
}
export async function readReceipt(file:File){
  if(file.size>5*1024*1024)throw new Error('Attachments must be under 5 MB')
  return {name:file.name,mime:file.type||'application/octet-stream',bytes:Array.from(new Uint8Array(await file.arrayBuffer()))}
}
