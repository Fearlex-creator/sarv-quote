import {useState} from 'react'
import {motion,useReducedMotion} from 'motion/react'
import {ArrowLeft,ArrowRight,Check,Leaf,ShieldCheck,Upload,Building2,UserRound,Package} from 'lucide-react'
import {Card,CardHeader,CardTitle,CardDescription,CardContent,CardFooter} from '@/components/ui/card'
import {Button} from '@/components/ui/button'
import {FieldGroup,Field,FieldLabel} from '@/components/ui/field'
import {ToggleGroup,ToggleGroupItem} from '@/components/ui/toggle-group'
import {Progress} from '@/components/ui/progress'
import {Badge} from '@/components/ui/badge'
import {Alert,AlertTitle,AlertDescription} from '@/components/ui/alert'
import {Input} from '@/components/ui/input'
import {Separator} from '@/components/ui/separator'
import {useWorkspace,useText,useWrite} from './context'
import {ChoiceField,DateField,TextField} from './fields'
import {request,type Workspace} from './api'
export function Onboarding({onDone}:{onDone:()=>void}){
 const w=useWorkspace(),t=useText(),write=useWrite(),reduced=useReducedMotion()
 const [step,setStep]=useState(0),[path,setPath]=useState(w.existing?'existing':'new'),[profile,setProfile]=useState(w.profile),[name,setName]=useState(w.name),[companyId,setCompany]=useState(w.companyId),[start,setStart]=useState(w.fiscalStart),[end,setEnd]=useState(w.fiscalEnd),[content,setContent]=useState(''),[preview,setPreview]=useState<{name:string;companies:{id:string;name:string}[];products:number;journals:number;quotes:number;fingerprint:string}|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false)
 const titles=[t('از کجا شروع کنیم؟','Where shall we start?'),t('سرو، متناسب با شما','Make Sarv yours'),t('فضای کاری شما','Your workspace'),t('آماده برای شروع','Ready to begin')]
 async function importPreview(file:File|undefined){if(!file)return;setBusy(true);setError('');try{const c=await file.text();const p=await request<typeof preview>(false,'importPreview',{content:c});setContent(c);setPreview(p)}catch(e){setError(String(e))}finally{setBusy(false)}}
 async function finish(){setBusy(true);setError('');try{
   if(path==='import'){
     if(!preview)throw new Error(t('ابتدا فایل را بازبینی کنید','Preview a backup first'))
     if(w.existing)await request<Workspace>(true,'newWorkspace')
     const imported=await request<Workspace>(true,'import',{content,fingerprint:preview.fingerprint})
     if(!imported.companyId){throw new Error(t('فایل وارد شد؛ برای کتاب قدیمی یک فضای حسابداری بسازید','Backup imported. Set up an accounting workspace for these legacy records.'))}
     await write.mutateAsync({action:'onboard',payload:{path:'existing',profile,companyId:imported.companyId,language:w.language}})
   }else{
     if(path==='new'&&w.existing)await request<Workspace>(true,'newWorkspace')
     await write.mutateAsync({action:'onboard',payload:{path,profile,name,companyId,start,end,language:w.language}})
   }
 onDone();
 }catch(e){setError(String(e))}finally{setBusy(false)}}
 return <main id="main" className="flex min-h-svh flex-col items-center justify-center gap-8 px-4 py-8">
  <div className="flex w-full max-w-4xl items-center justify-between gap-4"><div className="flex items-center gap-2"><Leaf aria-hidden="true"/><strong>{t('سرو','Sarv')}</strong></div><Button className="min-h-11" variant="ghost" onClick={()=>write.mutate({action:'settings',payload:{language:w.language==='fa'?'en':'fa',calendar:w.calendar,theme:w.theme,profile:w.profile,unit:w.unit,name:w.name}})}>{w.language==='fa'?'English':'فارسی'}</Button></div>
  <div className="grid w-full max-w-4xl gap-6 md:grid-cols-[1fr_280px]">
   <Card><CardHeader><Badge variant="secondary" className="w-fit">{t('راه‌اندازی اولیه','First-time setup')} · {step+1}/4</Badge><CardTitle><h1>{titles[step]}</h1></CardTitle><CardDescription>{t('یک شروع ساده، با مسیر رشد برای کسب‌وکار شما.','A considered start, with room for your business to grow.')}</CardDescription></CardHeader>
   <CardContent><Progress value={(step+1)*25} aria-label={t('پیشرفت راه‌اندازی','Setup progress')}/><motion.div key={step} initial={reduced?false:{opacity:0,y:8}} animate={{opacity:1,y:0}} transition={{duration:.22}} className="mt-6"><FieldGroup>
   {step===0&&<><Field><FieldLabel>{t('مسیر شروع','Choose your starting point')}</FieldLabel><ToggleGroup value={[path]} onValueChange={v=>{if(v[0])setPath(v[0])}} orientation="vertical" variant="outline" className="w-full"><ToggleGroupItem value="existing" disabled={!w.existing} className="min-h-14 w-full justify-start">{t('ادامه کسب‌وکار موجود','Continue existing business')}</ToggleGroupItem><ToggleGroupItem value="new" className="min-h-14 w-full justify-start">{t('ساخت فضای کاری جدید','Create a new workspace')}</ToggleGroupItem><ToggleGroupItem value="import" className="min-h-14 w-full justify-start"><Upload data-icon="inline-start"/>{t('ورود نسخه پشتیبان سرو','Import a Sarv backup')}</ToggleGroupItem></ToggleGroup></Field>
    {path==='existing'&&<ChoiceField label={t('کسب‌وکار موجود','Existing business')} value={companyId} onChange={setCompany} options={w.companies}/>}
    {path==='import'&&<><Field><FieldLabel htmlFor="backup-file">{t('فایل JSON سرو','Sarv JSON backup')}</FieldLabel><Input id="backup-file" type="file" accept=".json,application/json" onChange={e=>void importPreview(e.target.files?.[0])}/></Field>{preview&&<Alert><ShieldCheck/><AlertTitle>{t('فایل معتبر است','Backup validated')}</AlertTitle><AlertDescription>{preview.name||preview.companies[0]?.name} · {preview.products} {t('کالا','products')} · {preview.journals} {t('سند','journals')} · {preview.quotes} {t('پیش‌فاکتور','quotations')}</AlertDescription></Alert> }</>}
   </>}
   {step===1&&<Field><FieldLabel>{t('چطور از سرو استفاده می‌کنید؟','How will you use Sarv?')}</FieldLabel><ToggleGroup orientation="vertical" variant="outline" className="w-full" value={[profile]} onValueChange={v=>{if(v[0])setProfile(v[0] as Workspace['profile'])}}><ToggleGroupItem className="min-h-14 w-full justify-start" value="individual"><UserRound/>{t('شخصی · درآمد، هزینه و کیف پول','Individual · income, expenses and wallets')}</ToggleGroupItem><ToggleGroupItem className="min-h-14 w-full justify-start" value="service"><Building2/>{t('خدماتی · مشتریان و حسابداری','Service business · contacts and accounting')}</ToggleGroupItem><ToggleGroupItem className="min-h-14 w-full justify-start" value="trading"><Package/>{t('بازرگانی · خرید، فروش و انبار','Trading business · purchases, sales and stock')}</ToggleGroupItem></ToggleGroup></Field>}
   {step===2&&(path==='new'?<><TextField label={t('نام شخص یا کسب‌وکار','Your name or business name')} value={name} onChange={setName} required/><div className="grid gap-4 sm:grid-cols-2"><DateField label={t('شروع سال مالی','Fiscal year begins')} value={start} onChange={setStart}/><DateField label={t('پایان سال مالی','Fiscal year ends')} value={end} onChange={setEnd}/></div><Alert><ShieldCheck/><AlertTitle>{t('ثبت دقیق ریالی','Exact Rial books')}</AlertTitle><AlertDescription>{t('مبالغ به ریال ذخیره و به تومان نمایش داده می‌شوند. افتتاحیه صفر است؛ مانده‌ها و ورود اطلاعات از چک‌لیست ادامه می‌یابند.','Amounts are stored in Rials and displayed in Tomans. Books start at zero; continue balances and data setup from the checklist.')}</AlertDescription></Alert></>:<Alert><ShieldCheck/><AlertTitle>{t('تاریخچه شما حفظ می‌شود','Your history stays intact')}</AlertTitle><AlertDescription>{t('داده موجود حذف یا جایگزین نمی‌شود. ورود نسخه پشتیبان در فضای مستقل انجام می‌شود.','Existing data is preserved. Backup imports use an independent workspace.')}</AlertDescription></Alert>)}
   {step===3&&<><Alert><Check/><AlertTitle>{name||w.name||preview?.name||t('فضای کاری سرو','Sarv workspace')}</AlertTitle><AlertDescription>{profile==='individual'?t('زبان ساده برای پول روزمره شما.','Simple language for everyday money.'):t('حسابداری، مخاطبان و جریان کالا روی دفتر مشترک.','Accounting, contacts and goods workflows on one ledger.')}</AlertDescription></Alert><p>{t('پس از ورود، چک‌لیست قابل ادامه برای کیف پول، مخاطبان، کالاها، انبارها و نسخه پشتیبان در دسترس است.','After setup, a resumable checklist guides wallets, contacts, products, warehouses and backups.')}</p></>}
   {error&&<Alert variant="destructive"><AlertTitle>{t('نیاز به بررسی','Please review')}</AlertTitle><AlertDescription>{error}</AlertDescription></Alert>}
   </FieldGroup></motion.div></CardContent>
   <CardFooter className="flex justify-between gap-3"><Button className="min-h-11" variant="ghost" disabled={step===0||busy} onClick={()=>setStep(step-1)}><ArrowRight data-icon="inline-start" className="ltr:rotate-180"/>{t('بازگشت','Back')}</Button>{step<3?<Button className="min-h-11" disabled={busy||(step===0&&path==='import'&&!preview)||(step===0&&path==='existing'&&!companyId)||(step===2&&path==='new'&&!name.trim())} onClick={()=>setStep(step+1)}>{t('ادامه','Continue')}<ArrowLeft data-icon="inline-end" className="ltr:rotate-180"/></Button>:<Button className="min-h-11" disabled={busy} onClick={()=>void finish()}>{busy?t('در حال راه‌اندازی…','Setting up…'):t('ورود به سرو','Open Sarv')}<Check data-icon="inline-end"/></Button>}</CardFooter></Card>
   <Card className="h-fit"><CardHeader><CardTitle>{t('همراه حساب‌های شما','A home for your books')}</CardTitle><CardDescription>{t('از زندگی روزمره تا رشد کسب‌وکار','From everyday life to a growing business')}</CardDescription></CardHeader><CardContent className="flex flex-col gap-6"><div className="flex gap-3"><ShieldCheck aria-hidden="true"/><p>{t('اطلاعات روی دستگاه شما؛ دفتر دقیق و تاریخچه قابل بررسی.','Your data stays on your device, with exact books and reviewable history.')}</p></div><Separator/><div className="flex gap-3"><Leaf aria-hidden="true"/><p>{t('فارسی، راست‌به‌چپ و تقویم شمسی از همان ابتدا.','Persian, right-to-left and Jalali from the first step.')}</p></div><Separator/><p className="text-muted-foreground">{t('همکاری تیمی و دسترسی‌های سازمانی در نسخه‌های بعدی اضافه می‌شوند.','Team access and enterprise permissions will follow in later releases.')}</p></CardContent></Card>
  </div><footer className="text-sm text-muted-foreground">{t('سرو · حسابداری محلی','Sarv · Local accounting')}</footer>
 </main>
}
