# Persian accounting UI terminology

Research retrieved 2026-10-09 (Asia/Tehran). Scope: visible Persian text and bounded form/settings presentation. This is terminology research, not accountant approval, tax guidance, competitor parity or a change to Sarv's accounting policy. Research agent inspected the actual shell, settings, quotation exports, sales, procurement, accounting and sales export source; root owns implementation.

## Official sources and observed terms

| Source | Exact short terms observed | Supported distinction / limit |
| --- | --- | --- |
| [Sepidar customer and sales](https://www.sepidarsystem.com/products/sepidar/customer-and-sales-system/) | «پیش‌فاکتور»، «فاکتور فروش»، «فاکتور برگشت از فروش» | Separate commercial documents; additional reservations, tax, messaging and other advertised capabilities are not Sarv claims. |
| [Sepidar sales instruction](https://www.sepidarsystem.com/blog/sepidar-sales-system-instruction/) | «اعلامیه قیمت»، «صورت حساب طرف مقابل» | Price lists and customer statements have distinct purposes from invoices. |
| [Sepidar purchase instruction](https://www.sepidarsystem.com/blog/purchase-invoice/) | «فاکتور خرید»، «رسید انبار»، «تأمین‌کننده»، «فی»، «خالص» | Its article distinguishes a purchase document from the warehouse receipt. Its described financial timing differs from Sarv; do not copy its policy into Sarv. |
| [Hesabfa current help index](https://www.hesabfa.com/help/topics) | «فاکتورهای فروش»، «فاکتورهای خرید»، «برگشت از فروش»، «برگشت از خرید»، «تنظیمات مالی» | Current guide has separate sales, purchases, warehouse, accounting, reports and settings operations. |
| [Hesabfa sales form](https://www.hesabfa.com/help/topics/فروش-درآمد/فروش-جدید) | «مشتری»، «مبلغ واحد»، «تاریخچه»، «حواله خروج از انبار» | Searchable items/prices, quantity, discount, due date and invoice history are ordinary workflow vocabulary. Its editable/deletable invoice and manual paid-state behavior are not recommendations for Sarv. |
| [Hesabfa purchase form](https://www.hesabfa.com/help/topics/خرید-هزینه/خرید-جدید) | «تأمین‌کننده»، «مبلغ واحد»، «رسید ورود به انبار» | Supplier bill, goods receipt and payment are distinct operations. |
| [Hesabfa financial preferences](https://www.hesabfa.com/help/topics/تنظیمات/تنظیمات-مالی) | «نرخ مالیات بر ارزش افزوده» | A rate field is understandable to users; this does not establish a statutory rate or authorize a compliance claim. |
| [Holoo official business product](https://holoo.co.ir/product/ساده-کد-21/) | «پیش‌فاکتور»، «سفارش»، «فاکتور»، «برگشت از خرید»، «برگشت از فروش»، «حواله»، «رسید» | Official product page separately describes quotations/orders, conversion, commercial invoices, warehouse documents and accounts. We did not authenticate into a demo or adopt its posting/edit/delete behavior. |
| [Zoho Books quotes](https://www.zoho.com/us/books/help/quote/) and [sales orders](https://www.zoho.com/us/books/help/sales-order/) | “Quotes”, “Sales Order”, “Invoice” | Accepted quote can become an order or invoice. Sarv's own quotation approval is local version approval, not proof of customer acceptance or sending. |

Access note: `https://www.holoo.co.ir` initially returned an internal browser error. The canonical non-www official site, its business product page and its linked help landing page were readable. Hesabfa's initial narrow search queries returned no results; direct official navigation yielded the current guide and actual form pages. No reseller or third-party page is used as primary evidence. GrandTotal is unnecessary to determine the Persian names in this bounded change.

## Recommended Sarv vocabulary (application-specific synthesis)

| Current object / behavior | Recommended visible name | Must preserve |
| --- | --- | --- |
| Existing quotation / revision archive | پیش‌فاکتور / بایگانی پیش‌فاکتورها | No ledger posting, stock commitment, customer acceptance or send claim. Approved revisions retain their original number/history. |
| Sales order / saved order draft | سفارش فروش / پیش‌نویس سفارش فروش | An order is not an invoice; saved drafts are not posted sales. |
| Sales fulfillment's financial document | فاکتور فروش | Actual AR/revenue/COGS and inventory behavior; separate invoice ID. |
| Same fulfillment's warehouse document | حواله خروج | Separate delivery ID; do not imply standalone delivery-only action exists. |
| Fulfillment action | صدور فاکتور فروش و حواله خروج | Brief adjacent help says both are recorded together; partial quantity remains possible. |
| Supplier order | سفارش خرید | No payable or stock receipt on order alone. |
| Purchase receipt | رسید انبار | Receipt quantity/value and unbilled-goods accounting remain exact. |
| Supplier bill matching a receipt | فاکتور خرید / تطبیق با رسید انبار | Creates payable and matches existing stock receipt; no second inventory receipt. |
| Actual goods-return action | برگشت از فروش / برگشت از خرید | Linked original document, original warehouse/cost and separate credit IDs. |
| Receipt-only supplier return | برگشت رسید انبار | The receipt has no purchase invoice; do not claim a supplier credit exists. |
| Credit state on customer return | بستانکار مشتری ثبت شد | Decreases receivable; not cash refunded or payment settled. |
| Credit state on billed supplier return | بستانکار خرید ثبت شد | Decreases payable; no cash-refund claim. |
| Manual ledger journal | سند حسابداری | Posted record is immutable; correction by سند معکوس with reason. |
| Audit history | سوابق تغییرات | Local history, not external tamper-proof certification. |
| Chart / period / opening | کدینگ حساب‌ها / سال مالی / مانده‌های افتتاحیه | Do not imply assumed account mappings or year-end closing exists. |
| Inventory movements and balances | گردش کالا / موجودی و ارزش کالا | Quantity, unit and explicit rial/toman visible. |
| Price source catalog | فهرست قیمت | Preserve searchable product/price selection; a price list is not a sales invoice. |

Normal navigation: فروش، خرید، انبار، حسابداری، پیش‌فاکتور، بایگانی پیش‌فاکتورها، فهرست قیمت، مشتریان، تنظیمات، پشتیبان و بازیابی. Do not introduce cash/bank/cheque/tax navigation unless those workflows exist.

## Concise UI copy and settings

- App title: **سرو | حسابداری**. Page headings name the task; remove developer milestones, pilot descriptions and architectural nouns from ordinary page titles.
- Prefer **جدید**, **افزودن ردیف**, **ذخیرهٔ پیش‌نویس**, **پیش‌نمایش**, **ثبت**, **انصراف**, **بستن**, **ویرایش**, **صفحهٔ قبل / صفحهٔ بعد**. A final button names its actual document: ثبت سفارش، ثبت رسید انبار، ثبت فاکتور خرید.
- Prefer **قیمت واحد**, **مقدار**, **تخفیف**, **جمع ردیف**, **جمع کل**, **شماره / مرجع**, **تاریخ سند**, **تاریخ ثبت حسابداری**, **سررسید**. Distinguish entered independent/check total as **جمع اعلام‌شده**; do not remove the independently validated total just to simplify text.
- Concise statuses: **پیش‌نویس**, **تأییدشده**, **ثبت‌شده**, **در حال ذخیره…**, **ذخیره شد**, **تغییرات ذخیره‌نشده**, **در حال بارگذاری…**, **موردی یافت نشد**. Keep fields' precise errors and failed-save states. Quotation approval should say پیش‌فاکتور تأیید و ذخیره شد, not مشتری پذیرفت.
- Remove noisy routine-success diagnostics such as آماده‌سازی جست‌وجوی… پایان یافت، منبع فروش و جریان کار آماده‌اند، درخواست ثابت، هسته، سیاست نسخه…, and implementation event codes from top-level daily workflow captions. Put historical IDs/source/policy detail in an existing selectable details disclosure if needed; do not lose traceability.
- Compress long policy paragraphs into short task-relevant help. Keep real consequences visible before posting: stock/payable/receivable effect, original-cost returns, immutable posted records, simultaneous invoice/warehouse posting, no payment/refund, unsaved-data loss and backup replacement.
- Correct stale accounting help: current trial balance includes connected purchase/sales entries; it must not describe the ledger as دستی only or claim all subledgers remain unconfigured. Keep unimplemented capabilities plainly disclosed where relevant.
- Settings groups: **اطلاعات کسب‌وکار**, **پیش‌فرض‌های پیش‌فاکتور**, **شماره‌گذاری پیش‌فاکتور**, **سربرگ و شرایط پیش‌فاکتور**, **نمایش**, **پشتیبان و بازیابی**. Scope quotation-only preferences explicitly; do not imply invoice-wide defaults.
- Settings rows: **واحد مبلغ**, **مدت اعتبار (روز)**, **تخفیف پیش‌فرض**, **پیشوند شماره**, **شمارهٔ بعدی**, **شمارهٔ پیش‌فاکتور بعدی**, **تم برنامه** (سیستم / روشن / تیره), **تراکم جدول** (معمولی / فشرده), **نمایش ارقام**, **قالب تاریخ**, **متن سربرگ**, **متن پایانی**, **شرایط پیش‌فاکتور**.
- Rate row: **نرخ افزودهٔ پیش‌فاکتور (%)**, with **نرخ دلخواه؛ فقط برای محاسبهٔ پیش‌فاکتور** as compact scope help. Normal percentage display is preferable, but conversion must preserve integer basis-point storage exactly (0.01% steps, range 0…100%, no truncated fractional basis point). If that bounded conversion is not implemented, retain the explicit ۰٫۰۱٪ unit rather than mislabelling the stored integer as percent. No default statutory rate, tax-law change, or tax-compliance claim.
- Keep **۱ تومان = ۱۰ ریال** once in unit help. State whether a form converts an existing typed amount when units change; settings discount currently converts, while manual journal input intentionally does not. Do not make one generic claim for both.
- Historical frozen quote text/header/footer remains frozen. Improve default/new documents and visible export labels; do not migrate old customer-authored free text or mutate posted history.

## Acceptance checklist for root implementation / review

- [x] Generic quotation editor, archive, conversion dialog, settings and sample PDF consistently say پیش‌فاکتور; frozen/custom user text is preserved.
- [x] Posted sales/purchase documents say فاکتور فروش / فاکتور خرید; order/warehouse/return states remain distinct.
- [x] Navigation and normal forms/settings remove material developer jargon and needless paragraphs. The detailed GRNI wording refinement noted below remains nonblocking.
- [x] Quantity, units, money, links, exact checks, audit/reversal restrictions and unavailable payment/tax scope remain truthful.
- [x] Settings native rejection/discard/save/restart passes; exact percentage round trips and quotation numbering/revisions are verified.
- [x] Internal storage/event/widget identifiers remain stable; matching automation labels are updated to the visible copy.
- [x] Paired six-page images, native light/dark/narrow interactions and actual quotation/invoice/bill renderings reviewed.
- [x] Final 240 ordinary tests plus five opt-ins pass; independent review assesses actual semantics and raw State preservation.

Final bounded review: [independent source, visual and persistence evidence](assets/ui-copy/review.md), same final4 source/binary, sidebar 40 checks and product workflow 28 checks. No material blocker remains. A legacy compact `فاکتور‌نشده` phrase remains in a GRNI account label/error text and detailed procurement exports; normal receipt captions are clarified. No new performance or compliance claim follows from this copy pass.

## Independent implementation review — initial source pass

Reviewed the working implementation on `codex/persian-accounting-copy` on 2026-10-09. This pass was read-only for production files; the integration lead was still editing, so it is not an immutable final approval or native interaction result.

The new percentage parser preserves integer basis-point storage. It accepts localized digits and a decimal separator, validates at most two decimal places, checks 0…100%, rejects overprecision rather than truncating, and cannot overflow its scaled integer after the whole-percent bound. Its formatter represents all existing 0…10000 basis-point values exactly. The settings entry participates in dirty tracking and the save handler writes the validated integer into the existing settings field. Native save/discard/restart evidence remains required.

The reviewed accounting/domain diffs change visible/error vocabulary rather than posting equations, stored enum/event identifiers or schema. Quotations remain quotations; the actual sales fulfillment and supplier bill are named sales/purchase invoices. The trial balance's stale manual-only description is corrected.

Initial findings sent to the integration lead: fix mechanically generated Persian grammar (`مشتریانی…`, `سوابق تغییراتٔ`), remove the remaining quotation empty-state wording, replace `فاکتور‌نشده` with a natural receipt-without-invoice phrase, remove normal-form `کالای هسته` and order-policy jargon, give sales returns their ordinary name, align delivery/receipt export names with the UI, remove exposed milestone codes in report descriptions, and distinguish receipt-only purchase returns from billed purchase returns. These are pending correction verification; no financial parser or posting regression was identified in this source pass.

The follow-up source review found the material terminology corrections applied. The residual help phrase `واحد هسته` can be simplified to `واحد کالا`; it does not affect accounting semantics. Before-image sales review confirms the old quotation/invoice names and implementation paragraphs were visible, not hypothetical search findings.

Keyboard observer assessment: the Alt-page handler, ListBox selection callback, `show_workspace_content()` and F9 focus-restore handler are unchanged from the baseline. Selecting a current page does not promise moving keyboard focus from a previously focused sidebar row. The observed retained Accounting focus after Alt+9 is therefore not a copy-change regression. Establishing the current Inventory row with F9 hide/show, asserting its actual focus, then testing arrow/Enter navigation is a valid supported interaction check. It must replace, rather than retain, the old unsupported claim that Tab necessarily enters at the selected Inventory row. Native after-image and persisted settings review remains pending.

The integration lead subsequently chose an explicit navigation improvement instead of changing that observer expectation. Final4's visible-child callback captures whether the currently focused widget lies beneath either navigation ListBox, then focuses the new current row only in that case. The independent source review approves this bounded conditional behavior: body/form focus cannot satisfy the predicate, no financial writes occur, and the narrow row-activation path still hides navigation and transfers focus into content. The former unchanged-source observation above describes the pre-improvement state. Native checks must establish the new row-focus guarantee, unchanged content-focus behavior and narrow navigation return.
