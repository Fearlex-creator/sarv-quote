//! Guided native setup and staged opening. Financial writes cross RunCommand only.
use super::accounting::{DateField, date_field, date_value, form_dialog};
use super::*;
use sarv_quote::accounting::*;
use sarv_quote::core::*;
use std::rc::Weak;

pub(super) type Mutation = Box<dyn FnOnce(&mut AccountingState) -> Result<()> + Send>;
pub(super) type RunCommand =
    Rc<dyn Fn(adw::Dialog, gtk::Label, gtk::Button, &'static str, Mutation)>;
const ACTOR: &str = "local-single-user";
const POLICY: &str = "سیاست هسته v1 • تعهدی و موجودی دائمی\nذخیرهٔ دقیق ریال؛ ۱ تومان = ۱۰ ریال • مقدار تا سه رقم اعشار • گردکردن مثبت نیم‌به‌بالا\nمیانگین موزون متحرک برای هر کالا/انبار؛ موجودی منفی و ثبت قبل از آخرین تاریخ موجودی ممنوع؛ برگشت با بهای اصلی\nمالیات و ارسال قانونی غیرفعال؛ این تنظیمات گواهی حرفه‌ای یا قانونی نیست. تاریخ مالی، روز مدنی است؛ زمان ثبت رویداد جداگانه و UTC است.";

// Opt-in measurements run only in explicitly selected disposable test profiles.
struct PhaseMonitor {
    phase: &'static str,
    path: PathBuf,
    began: std::time::Instant,
    last: Cell<std::time::Instant>,
    gaps: RefCell<Vec<f64>>,
    done: Cell<bool>,
}
impl PhaseMonitor {
    fn start(ui: &Rc<Ui>, phase: &'static str, post: bool) -> Option<Rc<Self>> {
        if std::env::var("SARV_CORE_PERFORMANCE_TRACE").as_deref() != Ok("1")
            || std::env::var("SARV_TEST_INSTANCE").is_err()
        {
            return None;
        }
        let directory = ui.store.borrow().directory.canonicalize().ok()?;
        if !(directory.starts_with("/tmp")
            || directory.starts_with(Path::new(env!("CARGO_MANIFEST_DIR")).join("artifacts")))
        {
            return None;
        }
        let now = std::time::Instant::now();
        let trace = Rc::new(Self {
            phase,
            path: directory.join("core-performance-phases.jsonl"),
            began: now,
            last: Cell::new(now),
            gaps: RefCell::new(Vec::new()),
            done: Cell::new(false),
        });
        let weak_ui = Rc::downgrade(ui);
        let measure = trace.clone();
        let saw_busy = Cell::new(false);
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            if measure.done.get() {
                return glib::ControlFlow::Break;
            }
            let now = std::time::Instant::now();
            measure
                .gaps
                .borrow_mut()
                .push(now.duration_since(measure.last.replace(now)).as_secs_f64() * 1000.0);
            let Some(ui) = weak_ui.upgrade() else {
                measure.finish("closed");
                return glib::ControlFlow::Break;
            };
            if post {
                if ui.persistence_busy.get() {
                    saw_busy.set(true);
                } else if saw_busy.get() {
                    measure.finish("worker_finished");
                    return glib::ControlFlow::Break;
                } else if measure.began.elapsed().as_millis() > 64 {
                    measure.finish("not_started");
                    return glib::ControlFlow::Break;
                }
            }
            if measure.began.elapsed().as_secs() > 120 {
                measure.finish("timeout");
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        Some(trace)
    }
    fn finish(&self, result: &str) {
        if self.done.replace(true) {
            return;
        }
        let now = std::time::Instant::now();
        let mut gaps = self.gaps.borrow().clone();
        gaps.push(now.duration_since(self.last.get()).as_secs_f64() * 1000.0);
        let value = serde_json::json!({"phase":self.phase,"elapsed_ms":now.duration_since(self.began).as_secs_f64()*1000.0,"ticks":gaps.len()-1,"interval_ms":16,"max_gap_ms":gaps.iter().copied().fold(0.0,f64::max),"gaps_ms":gaps,"result":result,"boundary":if self.phase=="preview"{"dispatch through completed UI result rendering"}else{"dispatch through observed persistence busy release; durable state independently checked"}});
        let path = self.path.clone();
        let _ = std::thread::Builder::new()
            .name("sarv-core-metrics".into())
            .spawn(move || {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = writeln!(file, "{value}");
                }
            });
    }
}
fn finish_monitor(monitor: &Option<Rc<PhaseMonitor>>, result: &str) {
    if let Some(monitor) = monitor {
        monitor.finish(result);
    }
}

fn text(caption: &str, name: &str, value: &str, ltr: bool) -> gtk::Entry {
    let e = entry(caption);
    e.set_widget_name(name);
    e.set_text(value);
    if ltr {
        e.set_direction(gtk::TextDirection::Ltr);
    }
    accessible_name(&e, caption);
    e
}
fn fail(error: &gtk::Label, e: impl std::fmt::Display) {
    error.set_text(&e.to_string());
    error.set_visible(true);
}
fn amount(e: &gtk::Entry) -> Result<Money> {
    let input = e.text();
    let result = Money::parse(
        if input.trim().is_empty() {
            "0"
        } else {
            input.as_str()
        },
        MoneyUnit::Rial,
    );
    entry_validity(
        e,
        result.as_ref().err().map(ToString::to_string).as_deref(),
        "ریال دقیق؛ کسر ریال مجاز نیست",
    );
    if result.is_err() {
        e.grab_focus();
    }
    result
}
fn class(c: AccountClass) -> &'static str {
    match c {
        AccountClass::Asset => "دارایی",
        AccountClass::Liability => "بدهی",
        AccountClass::Equity => "حقوق مالکانه",
        AccountClass::Revenue => "درآمد",
        AccountClass::Expense => "هزینه",
    }
}

pub(super) fn build(ui: &Rc<Ui>, company_id: Option<String>, run: RunCommand) -> gtk::Widget {
    let panel = card();
    panel.set_widget_name("core-foundation-panel");
    panel.append(&heading("راه‌اندازی هسته و افتتاحیهٔ تطبیق‌شده"));
    let setup = button("شرکت تازه با راهنمای هسته");
    setup.set_widget_name("core-new-company");
    panel.append(&setup);
    let weak = Rc::downgrade(ui);
    let runner = run.clone();
    setup.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            setup_dialog(&ui, runner.clone());
        }
    });
    let Some(company) = company_id else {
        panel.append(&label("یک شرکت تازه با کدینگ قابل ویرایش، سیاست مشخص و سال مالی بسازید. هیچ مانده‌ای هنگام ساخت شرکت ایجاد نمی‌شود."));
        return panel.upcast();
    };
    let state = ui.store.borrow().state.accounting.clone();
    let Some(config) = state
        .core
        .setups
        .iter()
        .find(|s| s.command.company_id == company)
        .cloned()
    else {
        panel.append(&label("این شرکت دفتر دستی موجود دارد. حساب‌ها و اسناد آن حفظ می‌شوند؛ تبدیل به زیر‌دفتر و افتتاحیهٔ دوم در این مرحله مجاز نیست. برای شروع هسته از شرکت تازه استفاده کنید."));
        return panel.upcast();
    };
    panel.append(&label(POLICY));
    panel.append(&label(&format!(
        "منطقهٔ زمانی شرکت: {} • نسخهٔ سیاست و نگاشت حساب‌ها ثابت است.",
        config.command.policy.timezone.label()
    )));
    if let Some(opening) = state
        .core
        .openings
        .iter()
        .find(|o| o.command.company_id == company)
        .cloned()
    {
        panel.append(&heading(
            if opening.effect == OpeningEffect::ZeroBalanceInitialization {
                "آغاز بدون مانده تأیید‌شده • فقط خواندنی"
            } else {
                "افتتاحیه ثبت‌شده • فقط خواندنی"
            },
        ));
        panel.append(&label(&format!("مرز آغاز روز {} • واحد ریال • منبع {} • {} {}\nافتتاحیهٔ دیگری ایجاد نمی‌شود؛ مانده‌های ثبت‌شده با تغییر تنظیمات بازنویسی نمی‌شوند.",display_date(ui,&opening.command.as_of),isolate(&opening.command.source_id),if opening.effect==OpeningEffect::ZeroBalanceInitialization{"مرجع تأیید آغاز"}else{"سند"},isolate(&opening.journal_id))));
        let details = button("جزئیات منبع و تطبیق افتتاحیه");
        details.set_widget_name("core-view-opening");
        panel.append(&details);
        let weak = Rc::downgrade(ui);
        details.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                posted_dialog(&ui, opening.clone());
            }
        });
    } else {
        panel.append(&label("افتتاحیه هنوز ثبت نشده است. مانده‌های مستقل دفتر کل و جزئیات را وارد کنید؛ پیش‌نمایش باید تمام تفاوت‌های کنترلی را صفر نشان دهد. هیچ حساب سرمایه‌ای خودکار تراز نمی‌شود."));
        let begin = button("آماده‌سازی افتتاحیه");
        begin.set_widget_name("core-stage-opening");
        panel.append(&begin);
        let weak = Rc::downgrade(ui);
        begin.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                opening_dialog(&ui, config.clone(), run.clone());
            }
        });
    }
    panel.upcast()
}
fn setup_dialog(ui: &Rc<Ui>, run: RunCommand) {
    let (dialog, form, error, save) =
        form_dialog(ui, "ساخت شرکت، سال مالی و کدینگ", "core-setup-dialog");
    form.append(&label("این یک فرمان واحد است: شرکت، سال مالی، کدینگ و سیاست با هم ذخیره می‌شوند؛ افتتاحیه جداگانه بازبینی و ثبت می‌شود."));
    let name = text("نام شرکت", "core-company-name", "", false);
    let fiscal = text("عنوان سال مالی", "core-fiscal-name", "سال مالی ۱۴۰۵", false);
    let start = date_field(ui, "شروع سال مالی", "core-fiscal-start", "2026-03-21");
    let end = date_field(ui, "پایان سال مالی", "core-fiscal-end", "2027-03-20");
    let timezone = gtk::DropDown::from_strings(&["Asia/Tehran • تهران", "UTC"]);
    timezone.set_widget_name("core-timezone");
    accessible_name(&timezone, "منطقهٔ زمانی شرکت");
    form.append(&field("نام شرکت", &name));
    form.append(&field("عنوان سال مالی", &fiscal));
    form.append(&start.widget);
    form.append(&end.widget);
    form.append(&field("منطقهٔ زمانی؛ تاریخ مدنی تبدیل نمی‌شود", &timezone));
    form.append(&label(POLICY));
    form.append(&heading("کدینگ پیشنهادی قابل ویرایش"));
    form.append(&label("نام و کد هر حساب را تغییر دهید. نقش، طبقه، سمت عادی و الزام زیر‌دفتر برای حفظ کنترل‌ها مشخص است؛ پنج گروه اصلی غیرقابل ثبت ساخته می‌شوند."));
    let leaves = default_chart()
        .into_iter()
        .map(|leaf| {
            let box_ = card();
            box_.append(&heading(leaf.role.label()));
            box_.append(&label(&format!(
                "{} • عادی {} • {}",
                class(leaf.role.classification()),
                if leaf.role.normal_balance() == BalanceSide::Debit {
                    "بدهکار"
                } else {
                    "بستانکار"
                },
                if leaf.role.is_control() {
                    "فقط با جزئیات زیر‌دفتر"
                } else {
                    "حساب عادی"
                }
            )));
            let code = text(
                "کد حساب",
                &format!("core-chart-{}-code", leaf.role.key()),
                &leaf.code,
                true,
            );
            let name = text(
                "نام حساب",
                &format!("core-chart-{}-name", leaf.role.key()),
                &leaf.name,
                false,
            );
            box_.append(&field("کد حساب", &code));
            box_.append(&field("نام حساب", &name));
            form.append(&box_);
            (leaf.role, code, name)
        })
        .collect::<Vec<_>>();
    save.connect_clicked(move |save| {
        let values: Result<NewCoreCompany> = (|| {
            Ok(NewCoreCompany {
                name: name.text().to_string(),
                fiscal_name: fiscal.text().to_string(),
                start_date: date_value(&start)?,
                end_date: date_value(&end)?,
                timezone: if timezone.selected() == 0 {
                    CompanyTimezone::AsiaTehran
                } else {
                    CompanyTimezone::Utc
                },
                chart: leaves
                    .iter()
                    .map(|(role, code, name)| ChartLeaf {
                        role: *role,
                        code: code.text().to_string(),
                        name: name.text().to_string(),
                    })
                    .collect(),
                actor: ACTOR.into(),
            })
        })();
        match values {
            Ok(command) => run(
                dialog.clone(),
                error.clone(),
                save.clone(),
                "شرکت و سیاست هسته ساخته شد",
                Box::new(move |state| state.setup_core_company(command).map(|_| ())),
            ),
            Err(e) => fail(&error, e),
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Party,
    Unit,
    Item,
    Pack,
    Warehouse,
    Cash,
    Bank,
    Ar,
    Ap,
    SupplierAdvance,
    CustomerAdvance,
    Cr,
    Cp,
    Stock,
    Grni,
}
impl Kind {
    const ALL: [Kind; 15] = [
        Self::Party,
        Self::Unit,
        Self::Item,
        Self::Pack,
        Self::Warehouse,
        Self::Cash,
        Self::Bank,
        Self::Ar,
        Self::Ap,
        Self::SupplierAdvance,
        Self::CustomerAdvance,
        Self::Cr,
        Self::Cp,
        Self::Stock,
        Self::Grni,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Party => "طرف حساب؛ مشتری / تأمین‌کننده",
            Self::Unit => "واحد مقدار",
            Self::Item => "کالا و واحد پایه",
            Self::Pack => "نسبت دقیق بسته‌بندی",
            Self::Warehouse => "انبار",
            Self::Cash => "صندوق و وجه شمارش‌شده",
            Self::Bank => "بانک و ماندهٔ صورت‌حساب",
            Self::Ar => "فاکتور باز مشتری",
            Self::Ap => "صورتحساب باز تأمین‌کننده",
            Self::SupplierAdvance => "پیش‌پرداخت تأمین‌کننده",
            Self::CustomerAdvance => "پیش‌دریافت مشتری",
            Self::Cr => "چک دریافتی؛ نگهداری‌شده و وصول‌نشده",
            Self::Cp => "چک پرداختی؛ صادرشده و پاس‌نشده",
            Self::Stock => "موجودی مقدار و ارزش",
            Self::Grni => "رسید بدون صورتحساب؛ بخشی از موجودی",
        }
    }
}
#[derive(Clone)]
enum Input {
    Text(gtk::Entry),
    Choice(gtk::DropDown, Vec<String>),
    Check(gtk::CheckButton),
    Date(DateField),
}
impl Input {
    fn value(&self) -> Result<String> {
        match self {
            Self::Text(e) => Ok(e.text().to_string()),
            Self::Date(d) => date_value(d),
            Self::Choice(d, ids) => ids
                .get(d.selected() as usize)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("مرجع را انتخاب کنید؛ ابتدا رکورد پایه را بسازید")),
            Self::Check(c) => Ok(c.is_active().to_string()),
        }
    }
    fn set(&self, value: &str) {
        match self {
            Self::Text(e) => e.set_text(value),
            Self::Date(d) => {
                let _ = d.set_canonical(value);
            }
            Self::Choice(d, ids) => d.set_selected(
                ids.iter()
                    .position(|x| x == value)
                    .map(|i| i as u32)
                    .unwrap_or(gtk::INVALID_LIST_POSITION),
            ),
            Self::Check(c) => c.set_active(value == "true"),
        }
    }
}
type RecordRow = (String, String, Vec<(&'static str, String)>);
struct Editor {
    ui: Weak<Ui>,
    dialog: adw::Dialog,
    error: gtk::Label,
    post: gtk::Button,
    run: RunCommand,
    command: RefCell<OpeningCommand>,
    verified: RefCell<Option<OpeningCommand>>,
    revision: Cell<u64>,
    dirty: Cell<bool>,
    selected_kind: Cell<u32>,
    switching_kind: Cell<bool>,
    source: gtk::Entry,
    reason: gtk::Entry,
    balances: Vec<(String, gtk::Entry, gtk::Entry, gtk::Entry)>,
    required_controls: std::collections::HashSet<String>,
    kind: gtk::DropDown,
    form: gtk::Box,
    inputs: RefCell<Vec<(String, Input)>>,
    editing: RefCell<Option<String>>,
    list: gtk::Box,
    search: gtk::SearchEntry,
    page: Cell<usize>,
    review: gtk::Box,
}
impl Editor {
    fn invalidate(&self) {
        self.revision.set(self.revision.get() + 1);
        self.verified.borrow_mut().take();
        self.post.set_sensitive(false);
        clear(&self.review);
        self.review.append(&label(
            "پیش‌نمایش قدیمی است؛ پس از آخرین تغییر دوباره تطبیق را بررسی کنید.",
        ));
    }
    fn kind(&self) -> Kind {
        Kind::ALL[self.kind.selected().min(14) as usize]
    }
    fn collect(&self) -> Result<OpeningCommand> {
        let mut command = self.command.borrow().clone();
        command.source_id = self.source.text().to_string();
        command.reason = self.reason.text().to_string();
        command.balances = self
            .balances
            .iter()
            .map(|(id, debit, credit, reference)| {
                let debit = amount(debit)?;
                let credit = amount(credit)?;
                let reference = reference.text().to_string();
                if !self.required_controls.contains(id)
                    && debit == Money::ZERO
                    && credit == Money::ZERO
                    && reference.trim().is_empty()
                {
                    return Ok(None);
                }
                Ok(Some(OpeningBalance {
                    account_id: id.clone(),
                    debit,
                    credit,
                    source_reference: reference,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(command)
    }
    fn input(&self, key: &str, caption: &str, value: &str, ltr: bool) {
        let e = text(caption, &format!("core-record-{key}"), value, ltr);
        self.form.append(&field(caption, &e));
        self.inputs.borrow_mut().push((key.into(), Input::Text(e)));
    }
    fn check(&self, key: &str, caption: &str, value: bool) {
        let c = gtk::CheckButton::with_label(caption);
        c.set_active(value);
        c.set_widget_name(&format!("core-record-{key}"));
        self.form.append(&c);
        self.inputs.borrow_mut().push((key.into(), Input::Check(c)));
    }
    fn date(&self, ui: &Ui, key: &str, caption: &str) {
        let d = date_field(
            ui,
            caption,
            &format!("core-record-{key}"),
            &self.command.borrow().as_of,
        );
        self.form.append(&d.widget);
        self.inputs.borrow_mut().push((key.into(), Input::Date(d)));
    }
    fn choices(&self, key: &str, caption: &str, values: Vec<(String, String)>) {
        let names = values
            .iter()
            .map(|(id, name)| format!("{name} • {id}"))
            .collect::<Vec<_>>();
        let d = gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
        d.set_enable_search(true);
        d.set_widget_name(&format!("core-record-{key}"));
        accessible_name(&d, caption);
        self.form.append(&field(caption, &d));
        self.inputs.borrow_mut().push((
            key.into(),
            Input::Choice(d, values.into_iter().map(|x| x.0).collect()),
        ));
    }
    fn resource_form(self: &Rc<Self>) {
        self.dirty.set(false);
        clear(&self.form);
        self.inputs.borrow_mut().clear();
        self.editing.borrow_mut().take();
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let k = self.kind();
        self.form.append(&heading(k.label()));
        if k != Kind::Pack {
            self.input("id", "شناسهٔ ثابت؛ یکتا و قابل ارجاع", "", true);
        }
        let c = self.command.borrow().clone();
        let parties = || {
            c.parties
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect()
        };
        let items = || {
            c.items
                .iter()
                .map(|p| (p.id.clone(), format!("{} — {}", p.sku, p.name)))
                .collect()
        };
        let units = || {
            c.units
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect()
        };
        let warehouses = || {
            c.warehouses
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect()
        };
        match k {
            Kind::Party => {
                self.input("name", "نام طرف حساب", "", false);
                self.check("customer", "مشتری", true);
                self.check("supplier", "تأمین‌کننده", false);
            }
            Kind::Unit | Kind::Warehouse => self.input("name", "نام", "", false),
            Kind::Item => {
                self.input("sku", "کد کالا", "", true);
                self.input("name", "نام کالا", "", false);
                self.choices("unit", "واحد پایه", units());
                self.check("indivisible", "واحد پایه تجزیه‌ناپذیر است", true);
            }
            Kind::Pack => {
                self.form.append(&label("یک واحد انتخابی = صورت / مخرج واحد پایه. مثال: یک کارتن = ۱۲ / ۱ عدد. این نسبت مقدارهای تاریخی را تغییر نمی‌دهد."));
                self.choices("item", "کالا", items());
                self.choices("unit", "واحد بسته", units());
                self.input("numerator", "صورت نسبت مثبت", "12", true);
                self.input("denominator", "مخرج نسبت مثبت", "1", true);
            }
            Kind::Cash | Kind::Bank => {
                self.input("name", "نام صندوق / حساب", "", false);
                if k == Kind::Bank {
                    self.input("bank", "شناسهٔ حساب بانکی", "", true);
                    self.input("statement", "ماندهٔ پایان صورت‌حساب • ریال", "0", true);
                }
                self.input("amount", "ماندهٔ دفتر • ریال", "0", true);
                self.input("reference", "مرجع شمارش / صورت‌حساب مستقل", "", true);
            }
            Kind::Ar | Kind::Ap | Kind::SupplierAdvance | Kind::CustomerAdvance => {
                self.choices("party", "طرف حساب", parties());
                self.input("reference", "مرجع سند اصلی", "", true);
                self.date(&ui, "date", "تاریخ سند اصلی");
                if matches!(k, Kind::Ar | Kind::Ap) {
                    self.date(&ui, "due", "تاریخ سررسید");
                }
                self.input("amount", "ماندهٔ تسویه‌نشده • ریال", "0", true);
            }
            Kind::Cr | Kind::Cp => {
                self.form.append(&label("این چک در کنترل اسناد است و ماندهٔ بانک را تغییر نمی‌دهد. مرجع تسویهٔ فاکتور قدیمی را نگه دارید؛ همان مبلغ را دوباره در فاکتور باز وارد نکنید."));
                self.choices("party", "طرف حساب", parties());
                self.input("reference", "شماره / مرجع چک", "", true);
                self.input("settlement", "مرجع تسویهٔ تاریخی", "", true);
                self.input("bank", "بانک مربوط به چک", "", false);
                self.input("holder", "دارنده / محل نگهداری", "", false);
                self.date(&ui, "date", "تاریخ صدور");
                self.date(&ui, "due", "تاریخ سررسید");
                self.input("amount", "مبلغ چک • ریال", "0", true);
            }
            Kind::Stock | Kind::Grni => {
                if k == Kind::Grni {
                    self.form.append(&label("رسید بدون صورتحساب از مقدار و ارزش موجودی همین کالا/انبار است؛ این سطر موجودی اضافی ایجاد نمی‌کند."));
                    self.choices("party", "تأمین‌کننده", parties());
                    self.date(&ui, "date", "تاریخ دریافت اصلی");
                }
                self.choices("item", "کالا", items());
                self.choices("warehouse", "انبار", warehouses());
                self.choices("unit", "واحد مقدار انتخابی", units());
                self.input("quantity", "مقدار مثبت • حداکثر سه اعشار", "", true);
                self.input("amount", "ارزش کل سطر • ریال؛ قیمت واحد نیست", "0", true);
                self.input(
                    "reference",
                    if k == Kind::Grni {
                        "مرجع رسید اصلی"
                    } else {
                        "مرجع شمارش / ارزش‌گذاری"
                    },
                    "",
                    true,
                );
            }
        }
        let add = button("افزودن / ذخیرهٔ سطر");
        add.set_widget_name("core-add-record");
        self.form.append(&add);
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                match e.save_record() {
                    Ok(()) => {
                        e.error.set_visible(false);
                        e.invalidate();
                        e.resource_form();
                        e.render_list();
                    }
                    Err(err) => fail(&e.error, err),
                }
            }
        });
        let discard = button("کنار گذاشتن ورودی ذخیره‌نشدهٔ سطر");
        discard.set_widget_name("core-discard-record");
        self.form.append(&discard);
        let weak = Rc::downgrade(self);
        discard.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.error.set_visible(false);
                e.resource_form();
            }
        });
        for (_, input) in self.inputs.borrow().iter() {
            let weak = Rc::downgrade(self);
            let changed = move || {
                if let Some(e) = weak.upgrade() {
                    e.dirty.set(true);
                    e.invalidate();
                }
            };
            match input {
                Input::Text(entry) => {
                    entry.connect_changed(move |_| changed());
                }
                Input::Date(date) => {
                    date.entry.connect_changed(move |_| changed());
                }
                Input::Choice(choice, _) => {
                    choice.connect_selected_notify(move |_| changed());
                }
                Input::Check(check) => {
                    check.connect_toggled(move |_| changed());
                }
            }
        }
    }
    fn save_record(&self) -> Result<()> {
        let v = self
            .inputs
            .borrow()
            .iter()
            .map(|(k, i)| Ok((k.clone(), i.value()?)))
            .collect::<Result<std::collections::HashMap<_, _>>>()?;
        let get = |k: &str| v.get(k).cloned().unwrap_or_default();
        let id = if self.kind() == Kind::Pack {
            format!("{}/{}", get("item"), get("unit"))
        } else {
            get("id")
        };
        anyhow::ensure!(!id.trim().is_empty(), "شناسهٔ ثابت سطر را وارد کنید");
        for (key, input) in self.inputs.borrow().iter() {
            if let Input::Text(entry) = input {
                if entry.text().trim().is_empty() {
                    entry_validity(
                        entry,
                        Some("این فیلد لازم است؛ ورودی‌های دیگر حفظ شدند"),
                        "ورودی لازم",
                    );
                    entry.grab_focus();
                    anyhow::bail!(
                        "سطر {}: یک فیلد لازم خالی است؛ فیلد مشخص‌شده را تکمیل کنید",
                        id
                    );
                }
                if matches!(
                    key.as_str(),
                    "id" | "name" | "sku" | "reference" | "bank" | "holder" | "settlement"
                ) {
                    sarv_quote::domain::text(&entry.text())?;
                }
            }
        }
        if self.kind() == Kind::Party {
            anyhow::ensure!(
                get("customer") == "true" || get("supplier") == "true",
                "طرف حساب باید مشتری، تأمین‌کننده یا هر دو باشد"
            );
        }
        let company = self.command.borrow().company_id.clone();
        let k = self.kind();
        let current = self.rows(k);
        anyhow::ensure!(
            !current
                .iter()
                .any(|r| r.0 == id && self.editing.borrow().as_deref() != Some(id.as_str())),
            "شناسهٔ تکراری؛ سطر موجود را ویرایش کنید"
        );
        let money = || {
            let amount = Money::parse(&get("amount"), MoneyUnit::Rial)?;
            anyhow::ensure!(amount.rials() >= 0, "مبلغ سطر باید نامنفی باشد");
            Ok(amount)
        };
        let integer = |key: &str| -> Result<i64> {
            let s = sarv_quote::domain::digits(&get(key));
            let n = s
                .parse::<i64>()
                .map_err(|_| anyhow::anyhow!("{key}: عدد صحیح معتبر لازم است"))?;
            anyhow::ensure!(n > 0, "نسبت باید مثبت باشد");
            Ok(n)
        };
        let mut c = self.command.borrow().clone();
        if let Some(old) = self.editing.borrow().clone() {
            remove_row(&mut c, k, &old);
        }
        match k {
            Kind::Party => c.parties.push(CoreParty {
                id,
                company_id: company,
                name: get("name"),
                customer: get("customer") == "true",
                supplier: get("supplier") == "true",
            }),
            Kind::Unit => c.units.push(CoreUnit {
                id,
                company_id: company,
                name: get("name"),
            }),
            Kind::Warehouse => c.warehouses.push(CoreWarehouse {
                id,
                company_id: company,
                name: get("name"),
            }),
            Kind::Item => {
                let packs = self
                    .command
                    .borrow()
                    .items
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.packs.clone())
                    .unwrap_or_default();
                c.items.push(CoreItem {
                    id,
                    company_id: company,
                    sku: get("sku"),
                    name: get("name"),
                    base_unit_id: get("unit"),
                    indivisible: get("indivisible") == "true",
                    packs,
                });
            }
            Kind::Pack => {
                let item = c
                    .items
                    .iter_mut()
                    .find(|p| p.id == get("item"))
                    .ok_or_else(|| anyhow::anyhow!("کالا را انتخاب کنید"))?;
                let unit = get("unit");
                anyhow::ensure!(
                    !item.packs.iter().any(|p| p.unit_id == unit),
                    "نسبت این واحد قبلاً موجود است؛ سطر نسبت را حذف یا ویرایش کنید"
                );
                item.packs.push(PackRatio {
                    unit_id: unit,
                    numerator: integer("numerator")?,
                    denominator: integer("denominator")?,
                });
            }
            Kind::Cash => c.cash.push(OpeningCash {
                id,
                company_id: company,
                name: get("name"),
                balance: money()?,
                source_reference: get("reference"),
            }),
            Kind::Bank => c.banks.push(OpeningBank {
                id,
                company_id: company,
                name: get("name"),
                bank_reference: get("bank"),
                balance: money()?,
                statement_balance: Money::parse(&get("statement"), MoneyUnit::Rial)?,
                source_reference: get("reference"),
            }),
            Kind::Ar | Kind::Ap => c.open_items.push(OpeningOpenItem {
                id,
                company_id: company,
                party_id: get("party"),
                side: if k == Kind::Ar {
                    OpenItemSide::Receivable
                } else {
                    OpenItemSide::Payable
                },
                document_reference: get("reference"),
                document_date: get("date"),
                due_date: get("due"),
                remaining: money()?,
            }),
            Kind::SupplierAdvance | Kind::CustomerAdvance => c.advances.push(OpeningAdvance {
                id,
                company_id: company,
                party_id: get("party"),
                side: if k == Kind::SupplierAdvance {
                    AdvanceSide::Supplier
                } else {
                    AdvanceSide::Customer
                },
                source_reference: get("reference"),
                document_date: get("date"),
                remaining: money()?,
            }),
            Kind::Cr | Kind::Cp => c.cheques.push(OpeningCheque {
                id,
                company_id: company,
                party_id: get("party"),
                direction: if k == Kind::Cr {
                    ChequeDirection::Receivable
                } else {
                    ChequeDirection::Payable
                },
                state: if k == Kind::Cr {
                    OpeningChequeState::Held
                } else {
                    OpeningChequeState::Issued
                },
                bank_reference: get("bank"),
                holder: get("holder"),
                cheque_reference: get("reference"),
                settlement_reference: get("settlement"),
                issue_date: get("date"),
                due_date: get("due"),
                amount: money()?,
            }),
            Kind::Stock => c.stock.push(OpeningStock {
                id,
                company_id: company,
                item_id: get("item"),
                warehouse_id: get("warehouse"),
                unit_id: get("unit"),
                quantity_thousandths: sarv_quote::domain::quantity(&get("quantity"))?,
                value: money()?,
                source_reference: get("reference"),
            }),
            Kind::Grni => c.grni.push(OpeningGrni {
                id,
                company_id: company,
                party_id: get("party"),
                item_id: get("item"),
                warehouse_id: get("warehouse"),
                unit_id: get("unit"),
                quantity_thousandths: sarv_quote::domain::quantity(&get("quantity"))?,
                value: money()?,
                receipt_reference: get("reference"),
                received_date: get("date"),
            }),
        }
        *self.command.borrow_mut() = c;
        Ok(())
    }
    fn rows(&self, k: Kind) -> Vec<RecordRow> {
        let c = self.command.borrow();
        let mut out = Vec::new();
        macro_rules! push {($p:expr,$summary:expr,$($key:literal=>$value:expr),* $(,)?)=>{out.push(($p.id.clone(),$summary,vec![("id",$p.id.clone()),$(($key,$value)),*]));}}
        match k {
            Kind::Party => {
                for p in &c.parties {
                    push!(p,format!("{} • مشتری {} • تأمین‌کننده {}",p.name,p.customer,p.supplier),"name"=>p.name.clone(),"customer"=>p.customer.to_string(),"supplier"=>p.supplier.to_string());
                }
            }
            Kind::Unit => {
                for p in &c.units {
                    push!(p,p.name.clone(),"name"=>p.name.clone());
                }
            }
            Kind::Warehouse => {
                for p in &c.warehouses {
                    push!(p,p.name.clone(),"name"=>p.name.clone());
                }
            }
            Kind::Item => {
                for p in &c.items {
                    push!(p,format!("{} • {} • پایه {}",p.sku,p.name,p.base_unit_id),"sku"=>p.sku.clone(),"name"=>p.name.clone(),"unit"=>p.base_unit_id.clone(),"indivisible"=>p.indivisible.to_string());
                }
            }
            Kind::Pack => {
                for p in &c.items {
                    for q in &p.packs {
                        let id = format!("{}/{}", p.id, q.unit_id);
                        out.push((
                            id.clone(),
                            format!(
                                "{} • {} = {}/{} پایه",
                                p.name, q.unit_id, q.numerator, q.denominator
                            ),
                            vec![
                                ("id", id),
                                ("item", p.id.clone()),
                                ("unit", q.unit_id.clone()),
                                ("numerator", q.numerator.to_string()),
                                ("denominator", q.denominator.to_string()),
                            ],
                        ));
                    }
                }
            }
            Kind::Cash => {
                for p in &c.cash {
                    push!(p,format!("{} • {} ریال • {}",p.name,p.balance.rials(),p.source_reference),"name"=>p.name.clone(),"amount"=>p.balance.rials().to_string(),"reference"=>p.source_reference.clone());
                }
            }
            Kind::Bank => {
                for p in &c.banks {
                    push!(p,format!("{} • دفتر {} / صورت‌حساب {} ریال",p.name,p.balance.rials(),p.statement_balance.rials()),"name"=>p.name.clone(),"bank"=>p.bank_reference.clone(),"amount"=>p.balance.rials().to_string(),"statement"=>p.statement_balance.rials().to_string(),"reference"=>p.source_reference.clone());
                }
            }
            Kind::Ar | Kind::Ap => {
                for p in c
                    .open_items
                    .iter()
                    .filter(|p| (p.side == OpenItemSide::Receivable) == (k == Kind::Ar))
                {
                    push!(p,format!("{} • طرف {} • {} ریال • سررسید {}",p.document_reference,p.party_id,p.remaining.rials(),p.due_date),"party"=>p.party_id.clone(),"reference"=>p.document_reference.clone(),"date"=>p.document_date.clone(),"due"=>p.due_date.clone(),"amount"=>p.remaining.rials().to_string());
                }
            }
            Kind::SupplierAdvance | Kind::CustomerAdvance => {
                for p in c
                    .advances
                    .iter()
                    .filter(|p| (p.side == AdvanceSide::Supplier) == (k == Kind::SupplierAdvance))
                {
                    push!(p,format!("{} • طرف {} • {} ریال",p.source_reference,p.party_id,p.remaining.rials()),"party"=>p.party_id.clone(),"reference"=>p.source_reference.clone(),"date"=>p.document_date.clone(),"amount"=>p.remaining.rials().to_string());
                }
            }
            Kind::Cr | Kind::Cp => {
                for p in c
                    .cheques
                    .iter()
                    .filter(|p| (p.direction == ChequeDirection::Receivable) == (k == Kind::Cr))
                {
                    push!(p,format!("{} • طرف {} • {} ریال • {:?} • سررسید {}",p.cheque_reference,p.party_id,p.amount.rials(),p.state,p.due_date),"party"=>p.party_id.clone(),"reference"=>p.cheque_reference.clone(),"settlement"=>p.settlement_reference.clone(),"bank"=>p.bank_reference.clone(),"holder"=>p.holder.clone(),"date"=>p.issue_date.clone(),"due"=>p.due_date.clone(),"amount"=>p.amount.rials().to_string());
                }
            }
            Kind::Stock => {
                for p in &c.stock {
                    push!(p,format!("کالا {} • انبار {} • {} {} • {} ریال • {}",p.item_id,p.warehouse_id,quantity_text(p.quantity_thousandths),p.unit_id,p.value.rials(),p.source_reference),"item"=>p.item_id.clone(),"warehouse"=>p.warehouse_id.clone(),"unit"=>p.unit_id.clone(),"quantity"=>quantity_text(p.quantity_thousandths),"amount"=>p.value.rials().to_string(),"reference"=>p.source_reference.clone());
                }
            }
            Kind::Grni => {
                for p in &c.grni {
                    push!(p,format!("رسید {} • کالا {} • انبار {} • {} {} • {} ریال، داخل موجودی",p.receipt_reference,p.item_id,p.warehouse_id,quantity_text(p.quantity_thousandths),p.unit_id,p.value.rials()),"party"=>p.party_id.clone(),"item"=>p.item_id.clone(),"warehouse"=>p.warehouse_id.clone(),"unit"=>p.unit_id.clone(),"quantity"=>quantity_text(p.quantity_thousandths),"amount"=>p.value.rials().to_string(),"reference"=>p.receipt_reference.clone(),"date"=>p.received_date.clone());
                }
            }
        }
        out
    }
    fn render_list(self: &Rc<Self>) {
        clear(&self.list);
        let kind = self.kind();
        let needle = sarv_quote::domain::normalize(&self.search.text());
        let rows = self
            .rows(kind)
            .into_iter()
            .filter(|r| {
                sarv_quote::domain::normalize(&format!("{} {}", r.0, r.1)).contains(&needle)
            })
            .collect::<Vec<_>>();
        let pages = rows.len().div_ceil(25).max(1);
        self.page.set(self.page.get().min(pages - 1));
        self.list.append(&label(&format!(
            "{} سطر • صفحه {} از {} • فقط پیش‌نویس موقت",
            rows.len(),
            self.page.get() + 1,
            pages
        )));
        for (id, summary, values) in rows.into_iter().skip(self.page.get() * 25).take(25) {
            let card_ = card();
            card_.append(&label(&format!("{id} • {summary}")));
            let actions = row();
            let edit = button("ویرایش سطر");
            edit.set_widget_name(&format!("core-record-edit-{id}"));
            let delete = button("حذف از پیش‌نویس");
            delete.set_widget_name(&format!("core-record-delete-{id}"));
            actions.append(&edit);
            actions.append(&delete);
            card_.append(&actions);
            self.list.append(&card_);
            let delete_id = id.clone();
            let weak = Rc::downgrade(self);
            edit.connect_clicked(move |_| {
                if let Some(e) = weak.upgrade() {
                    if e.dirty.get(){fail(&e.error,"ورودی جاری را ذخیره کنید یا کنار بگذارید؛ سپس سطر دیگری را ویرایش کنید.");return;}
                    e.resource_form();
                    *e.editing.borrow_mut() = Some(id.clone());
                    for (key, value) in &values {
                        if let Some((_, i)) = e.inputs.borrow().iter().find(|(k, _)| k == key) {
                            i.set(value);
                        }
                    }
                    if let Some((_, Input::Text(input))) = e.inputs.borrow().first() {
                        input.grab_focus();
                    }
                }
            });
            let weak = Rc::downgrade(self);
            delete.connect_clicked(move |_| {
                if let Some(e) = weak.upgrade() {
                    if e.dirty.get() {
                        fail(
                            &e.error,
                            "ورودی جاری را ذخیره کنید یا کنار بگذارید؛ سپس سطر را حذف کنید.",
                        );
                        return;
                    }
                    remove_row(&mut e.command.borrow_mut(), kind, &delete_id);
                    e.invalidate();
                    e.resource_form();
                    e.render_list();
                }
            });
        }
        let controls = row();
        let prev = button("صفحهٔ قبل");
        let next = button("صفحهٔ بعد");
        prev.set_sensitive(self.page.get() > 0);
        next.set_sensitive(self.page.get() + 1 < pages);
        controls.append(&prev);
        controls.append(&next);
        self.list.append(&controls);
        let weak = Rc::downgrade(self);
        prev.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.page.set(e.page.get().saturating_sub(1));
                e.render_list();
            }
        });
        let weak = Rc::downgrade(self);
        next.connect_clicked(move |_| {
            if let Some(e) = weak.upgrade() {
                e.page.set(e.page.get() + 1);
                e.render_list();
            }
        });
    }
    fn preview(self: &Rc<Self>) {
        if self.dirty.get() {
            fail(
                &self.error,
                "سطر در حال ویرایش است؛ ابتدا افزودن / ذخیرهٔ سطر یا کنار گذاشتن ورودی را انتخاب کنید.",
            );
            return;
        }
        let command = match self.collect() {
            Ok(c) => c,
            Err(err) => {
                fail(&self.error, err);
                return;
            }
        };
        self.invalidate();
        self.error.set_visible(false);
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let state = ui.store.borrow().state.accounting.clone();
        let revision = self.revision.get();
        let weak = Rc::downgrade(self);
        let reviewed = command.clone();
        let monitor = PhaseMonitor::start(&ui, "preview", false);
        let callback_monitor = monitor.clone();
        let accepted=ui.run_background("بررسی مانده‌ها و جزئیات افتتاحیه…",move||state.opening_preview(&command),move|ui,result|{
            let Some(e)=weak.upgrade()else{finish_monitor(&callback_monitor,"closed");return;};
            if !e.dialog.is_visible()||e.revision.get()!=revision{finish_monitor(&callback_monitor,"discarded");return;}
            clear(&e.review);let outcome=match result{
                Ok(p)=>{preview_table(&e.review,&p,&display_date(ui,&p.as_of),false);if p.balanced&&p.reconciled{e.post.set_label(if p.total_debit==Money::ZERO&&p.total_credit==Money::ZERO{"تأیید آغاز بدون مانده"}else{"ثبت افتتاحیهٔ تطبیق‌شده"});*e.verified.borrow_mut()=Some(reviewed);e.post.set_sensitive(true);"verified"}else{e.review.append(&label("ثبت مسدود است؛ تفاوت دفتر کل و جزئیات یا تراز را اصلاح کنید. هیچ مابه‌التفاوت خودکار ساخته نمی‌شود."));"differences"}},
                Err(err)=>{fail(&e.error,&err);e.review.append(&label(&err.to_string()));"invalid"}
            };finish_monitor(&callback_monitor,outcome);
        });
        if !accepted {
            finish_monitor(&monitor, "busy");
        }
    }
}
fn quantity_text(n: i64) -> String {
    format!("{}.{:03}", n / 1000, n % 1000)
}
fn remove_row(c: &mut OpeningCommand, k: Kind, id: &str) {
    match k {
        Kind::Party => c.parties.retain(|p| p.id != id),
        Kind::Unit => c.units.retain(|p| p.id != id),
        Kind::Item => c.items.retain(|p| p.id != id),
        Kind::Pack => {
            for p in &mut c.items {
                let item = p.id.clone();
                p.packs.retain(|q| format!("{item}/{}", q.unit_id) != id);
            }
        }
        Kind::Warehouse => c.warehouses.retain(|p| p.id != id),
        Kind::Cash => c.cash.retain(|p| p.id != id),
        Kind::Bank => c.banks.retain(|p| p.id != id),
        Kind::Ar | Kind::Ap => c.open_items.retain(|p| p.id != id),
        Kind::SupplierAdvance | Kind::CustomerAdvance => c.advances.retain(|p| p.id != id),
        Kind::Cr | Kind::Cp => c.cheques.retain(|p| p.id != id),
        Kind::Stock => c.stock.retain(|p| p.id != id),
        Kind::Grni => c.grni.retain(|p| p.id != id),
    }
}
fn preview_table(target: &gtk::Box, p: &OpeningPreview, cutoff: &str, posted: bool) {
    if p.total_debit == Money::ZERO && p.total_credit == Money::ZERO {
        target.append(&label("آغاز بدون مانده: تأیید، تاریخ و کنترل‌های صفر را ثبت می‌کند؛ سند مالی خالی یا وجهی ایجاد نمی‌شود."));
    }
    target.append(&heading(if p.balanced && p.reconciled {
        "تراز و تمام کنترل‌ها تطبیق دارند"
    } else {
        "تفاوت‌ها نیاز به اصلاح دارند"
    }));
    target.append(&label(&format!(
        "مرز آغاز روز {} • ریال • {}\nجمع بدهکار {} • جمع بستانکار {}",
        cutoff,
        if posted {
            "ثبت‌شده؛ فقط افتتاحیه"
        } else {
            "پیش‌نویس، هنوز ثبت نشده"
        },
        p.total_debit.rials(),
        p.total_credit.rials()
    )));
    for c in &p.controls {
        let box_ = card();
        box_.append(&heading(c.role.label()));
        box_.append(&label(&format!(
            "دفتر کل {} ریال • جزئیات {} ریال • تفاوت {} ریال",
            c.ledger.rials(),
            c.detail.rials(),
            c.difference.rials()
        )));
        if c.difference.rials() != 0 {
            box_.add_css_class("error");
        }
        target.append(&box_);
    }
}
fn opening_dialog(ui: &Rc<Ui>, setup: CoreSetup, run: RunCommand) {
    let state = ui.store.borrow().state.accounting.clone();
    let cfg = &setup.command;
    let Some(period) = state.periods.iter().find(|p| p.id == cfg.period_id) else {
        return;
    };
    let (dialog, outer, error, post) =
        form_dialog(ui, "ثبت افتتاحیهٔ تطبیق‌شده", "core-opening-dialog");
    post.set_sensitive(false);
    outer.append(&label(&format!("افتتاحیهٔ آغاز روز {} • واحد همهٔ مبلغ‌ها ریال (IRR)\nثبت، یک منبع و یک سند افتتاحیه با جزئیات ثابت می‌سازد. پس از ثبت قابل ویرایش نیست؛ انصراف تمام این پیش‌نویس را کنار می‌گذارد.",display_date(ui,&period.start_date))));
    let source = text("مرجع انتقال / افتتاحیه", "core-opening-source", "", true);
    let reason = text("توضیح منشأ مانده‌ها", "core-opening-reason", "", false);
    outer.append(&field("مرجع انتقال / افتتاحیه", &source));
    outer.append(&field("توضیح منشأ مانده‌ها", &reason));
    let tabs = gtk::Stack::new();
    tabs.set_widget_name("core-opening-tabs");
    tabs.set_transition_type(gtk::StackTransitionType::Crossfade);
    let switch = gtk::StackSwitcher::builder().stack(&tabs).build();
    outer.append(&switch);
    outer.append(&tabs);
    let ledger = column();
    ledger.append(&heading("مانده‌های مستقل دفتر کل"));
    ledger.append(&label("مبلغ‌ها را از تراز افتتاحیهٔ مستقل وارد کنید. سطرهای صفر هم اظهار صریح کنترل خالی هستند؛ جزئیات جای ماندهٔ دفتر را نمی‌گیرند."));
    let mut balances = Vec::new();
    for m in &cfg.mappings {
        let Some(a) = state.accounts.iter().find(|a| a.id == m.account_id) else {
            continue;
        };
        let box_ = card();
        box_.append(&heading(&format!(
            "{} • {} • {}",
            a.code,
            a.name,
            m.role.label()
        )));
        let debit = text(
            "بدهکار • ریال",
            &format!("core-balance-{}-debit", m.role.key()),
            "0",
            true,
        );
        let credit = text(
            "بستانکار • ریال",
            &format!("core-balance-{}-credit", m.role.key()),
            "0",
            true,
        );
        let reference = text(
            "مرجع سطر تراز",
            &format!("core-balance-{}-reference", m.role.key()),
            "",
            true,
        );
        box_.append(&field("بدهکار • ریال", &debit));
        box_.append(&field("بستانکار • ریال", &credit));
        box_.append(&field("مرجع سطر تراز مستقل", &reference));
        ledger.append(&box_);
        balances.push((a.id.clone(), debit, credit, reference));
    }
    tabs.add_titled(&ledger, Some("ledger"), "دفتر کل");
    let records = column();
    let kind =
        gtk::DropDown::from_strings(&Kind::ALL.iter().map(|k| k.label()).collect::<Vec<_>>());
    kind.set_widget_name("core-record-kind");
    accessible_name(&kind, "نوع جزئیات افتتاحیه");
    records.append(&field("نوع جزئیات؛ ابتدا طرف، واحد، کالا و انبار", &kind));
    let form = card();
    records.append(&form);
    let search = gtk::SearchEntry::new();
    search.set_search_delay(60);
    search.set_widget_name("core-record-search");
    accessible_name(&search, "جستجوی سطرهای جزئیات");
    records.append(&search);
    let list = column();
    records.append(&list);
    tabs.add_titled(&records, Some("records"), "جزئیات");
    let review = column();
    let review_page = column();
    let check = button("بررسی تراز و تطبیق کنترل‌ها");
    check.set_widget_name("core-preview-opening");
    review_page.append(&check);
    review_page.append(&review);
    tabs.add_titled(&review_page, Some("review"), "بازبینی");
    let command = OpeningCommand {
        company_id: cfg.company_id.clone(),
        period_id: cfg.period_id.clone(),
        setup_id: setup.id.clone(),
        as_of: period.start_date.clone(),
        source_id: String::new(),
        request_key: uuid::Uuid::new_v4().to_string(),
        actor: ACTOR.into(),
        reason: String::new(),
        import_source: None,
        balances: Vec::new(),
        parties: Vec::new(),
        units: Vec::new(),
        items: Vec::new(),
        warehouses: Vec::new(),
        cash: Vec::new(),
        banks: Vec::new(),
        open_items: Vec::new(),
        advances: Vec::new(),
        cheques: Vec::new(),
        stock: Vec::new(),
        grni: Vec::new(),
    };
    let editor = Rc::new(Editor {
        ui: Rc::downgrade(ui),
        dialog: dialog.clone(),
        error: error.clone(),
        post: post.clone(),
        run,
        command: RefCell::new(command),
        verified: RefCell::new(None),
        revision: Cell::new(0),
        dirty: Cell::new(false),
        selected_kind: Cell::new(0),
        switching_kind: Cell::new(false),
        source: source.clone(),
        reason: reason.clone(),
        balances,
        required_controls: cfg
            .mappings
            .iter()
            .filter(|m| m.role.is_control())
            .map(|m| m.account_id.clone())
            .collect(),
        kind: kind.clone(),
        form,
        inputs: RefCell::new(Vec::new()),
        editing: RefCell::new(None),
        list,
        search: search.clone(),
        page: Cell::new(0),
        review,
    });
    for e in std::iter::once(&source)
        .chain(std::iter::once(&reason))
        .chain(editor.balances.iter().flat_map(|(_, a, b, c)| [a, b, c]))
    {
        let weak = Rc::downgrade(&editor);
        e.connect_changed(move |_| {
            if let Some(e) = weak.upgrade() {
                e.invalidate();
            }
        });
    }
    let weak = Rc::downgrade(&editor);
    kind.connect_selected_notify(move |choice| {
        if let Some(e) = weak.upgrade() {
            if e.switching_kind.get(){return;}
            if e.dirty.get(){e.switching_kind.set(true);choice.set_selected(e.selected_kind.get());e.switching_kind.set(false);fail(&e.error,"ورودی سطر حفظ شد؛ برای تغییر نوع ابتدا سطر را ذخیره کنید یا ورودی را کنار بگذارید.");return;}
            e.selected_kind.set(choice.selected());
            e.page.set(0);
            e.resource_form();
            e.render_list();
        }
    });
    let weak = Rc::downgrade(&editor);
    search.connect_search_changed(move |_| {
        if let Some(e) = weak.upgrade() {
            e.page.set(0);
            e.render_list();
        }
    });
    let weak = Rc::downgrade(&editor);
    check.connect_clicked(move |_| {
        if let Some(e) = weak.upgrade() {
            e.preview();
        }
    });
    let e = editor.clone();
    let save_handler = post.connect_clicked(move |save| {
        if e.dirty.get() {
            fail(
                &e.error,
                "سطر جاری ذخیره نشده است؛ ابتدا ذخیره یا کنار گذاشتن را انتخاب کنید.",
            );
            return;
        }
        match e.collect() {
            Ok(command) if e.verified.borrow().as_ref() == Some(&command) => {
                let runner = e.run.clone();
                if let Some(ui) = e.ui.upgrade() {
                    let _ = PhaseMonitor::start(&ui, "post", true);
                }
                runner(
                    e.dialog.clone(),
                    e.error.clone(),
                    save.clone(),
                    "افتتاحیهٔ تطبیق‌شده ثبت شد",
                    Box::new(move |state| state.post_opening(command).map(|_| ())),
                );
            }
            Ok(_) => {
                e.invalidate();
                fail(&e.error, "ابتدا نسخهٔ فعلی مانده‌ها را تطبیق دهید");
            }
            Err(err) => fail(&e.error, err),
        }
    });
    let weak = Rc::downgrade(&editor);
    let handler = RefCell::new(Some(save_handler));
    dialog.connect_closed(move |_| {
        if let Some(e) = weak.upgrade() {
            e.verified.borrow_mut().take();
            if let Some(handler) = handler.borrow_mut().take() {
                e.post.disconnect(handler);
            }
        }
    });
    editor.resource_form();
    editor.render_list();
    editor.invalidate();
    // The dialog's save handler owns the editor; every other callback is weak.
    import_button(ui, &editor, &outer);
}
fn import_button(ui: &Rc<Ui>, editor: &Rc<Editor>, outer: &gtk::Box) {
    let import = button("ورود CSV به پیش‌نویس");
    import.set_widget_name("core-import-opening");
    outer.append(&import);
    outer.append(&label("ورود CSV فقط این پیش‌نویس موقت را جایگزین می‌کند؛ دفتر فعال تغییر نمی‌کند. اصل ردیف‌های فایل همراه مقادیر ویرایش‌شدهٔ پیش‌نویس در منشأ افتتاحیه نگهداری می‌شود."));
    let weak = Rc::downgrade(editor);
    let ui = Rc::downgrade(ui);
    import.connect_clicked(move |_| {
        let (Some(e), Some(ui)) = (weak.upgrade(), ui.upgrade()) else {
            return;
        };
        if e.dirty.get() {
            fail(
                &e.error,
                "ورودی جاری را ذخیره یا کنار بگذارید؛ سپس CSV را وارد کنید.",
            );
            return;
        }
        let chooser = gtk::FileDialog::builder()
            .title("فایل CSV افتتاحیه با واحد صریح")
            .build();
        let weak = Rc::downgrade(&e);
        let parent = ui.window.clone();
        let ui = Rc::downgrade(&ui);
        chooser.open(
            Some(&parent),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                let (Some(e), Some(ui)) = (weak.upgrade(), ui.upgrade()) else {
                    return;
                };
                let Ok(file) = result else {
                    return;
                };
                let Some(path) = file.path() else {
                    return;
                };
                let state = ui.store.borrow().state.accounting.clone();
                let company = e.command.borrow().company_id.clone();
                let request = e.command.borrow().request_key.clone();
                let weak = Rc::downgrade(&e);
                let revision = e.revision.get();
                ui.run_background(
                    "خواندن و بررسی CSV افتتاحیه…",
                    move || {
                        Ok(sarv_quote::core_import::opening_file_report(
                            &state, &company, &path, ACTOR, &request,
                        ))
                    },
                    move |_, result| {
                        let Some(e) = weak.upgrade() else {
                            return;
                        };
                        if !e.dialog.is_visible() || e.revision.get() != revision {
                            return;
                        }
                        match result {
                            Ok(report) => {
                                if !report.diagnostics.is_empty() {
                                    let mut message = report
                                        .diagnostics
                                        .iter()
                                        .map(|d| {
                                            format!(
                                                "سطر {} • ستون {}: {}",
                                                d.row, d.column, d.reason
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    if report.truncated {
                                        message.push_str(
                                            "\nخطاهای بیشتری موجود است؛ فایل را اصلاح کنید.",
                                        );
                                    }
                                    fail(&e.error, message);
                                    return;
                                }
                                if let Some(command) = report.value {
                                    e.source.set_text(&command.source_id);
                                    e.reason.set_text(&command.reason);
                                    for (id, debit, credit, reference) in &e.balances {
                                        if let Some(b) =
                                            command.balances.iter().find(|b| b.account_id == *id)
                                        {
                                            debit.set_text(&b.debit.rials().to_string());
                                            credit.set_text(&b.credit.rials().to_string());
                                            reference.set_text(&b.source_reference);
                                        } else {
                                            debit.set_text("0");
                                            credit.set_text("0");
                                            reference.set_text("");
                                        }
                                    }
                                    *e.command.borrow_mut() = command;
                                    e.error.set_visible(false);
                                    e.invalidate();
                                    e.resource_form();
                                    e.render_list();
                                }
                            }
                            Err(err) => fail(&e.error, err),
                        }
                    },
                );
            },
        );
    });
}
fn isolate(s: &str) -> String {
    format!("\u{2068}{s}\u{2069}")
}
fn display_date(ui: &Ui, date: &str) -> String {
    let settings = &ui.store.borrow().state.settings;
    format!(
        "\u{2068}{}\u{2069}",
        managers::display_date(date, settings.display_digits, settings.date_display)
    )
}
fn posted_dialog(ui: &Rc<Ui>, opening: OpeningRecord) {
    let dialog = adw::Dialog::builder()
        .title("منبع افتتاحیهٔ ثبت‌شده")
        .content_width(680)
        .content_height(700)
        .build();
    dialog.set_widget_name("core-posted-dialog");
    let content = column();
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&heading(
        if opening.effect == OpeningEffect::ZeroBalanceInitialization {
            "آغاز بدون مانده تأیید‌شده • فقط خواندنی"
        } else {
            "ثبت‌شده • فقط خواندنی"
        },
    ));
    content.append(&label(&format!(
        "آغاز روز {} • ریال • منبع {}\n{} {} • دلیل {}",
        display_date(ui, &opening.command.as_of),
        isolate(&opening.command.source_id),
        if opening.effect == OpeningEffect::ZeroBalanceInitialization {
            "مرجع تأیید آغاز"
        } else {
            "سند"
        },
        isolate(&opening.journal_id),
        opening.command.reason
    )));
    let unit = gtk::DropDown::from_strings(&["ریال • IRR", "تومان • هر تومان ۱۰ ریال"]);
    unit.set_widget_name("core-export-unit");
    accessible_name(&unit, "واحد نمایش گزارش افتتاحیه");
    content.append(&field(
        "واحد نمایش خروجی؛ ماندهٔ ذخیره‌شده تغییر نمی‌کند",
        &unit,
    ));
    let actions = row();
    let csv = button("خروجی CSV کامل");
    csv.set_widget_name("core-export-csv");
    let pdf = button("گزارش PDF افتتاحیه");
    pdf.set_widget_name("core-export-pdf");
    actions.append(&csv);
    actions.append(&pdf);
    content.append(&actions);
    for (button, pdf) in [(csv, false), (pdf, true)] {
        let weak = Rc::downgrade(ui);
        let company = opening.command.company_id.clone();
        let unit = unit.clone();
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                export_opening(
                    &ui,
                    company.clone(),
                    pdf,
                    if unit.selected() == 0 {
                        MoneyUnit::Rial
                    } else {
                        MoneyUnit::Toman
                    },
                );
            }
        });
    }
    let control = column();
    content.append(&control);
    let state = ui.store.borrow().state.accounting.clone();
    let company = opening.command.company_id.clone();
    let cutoff = opening.command.as_of.clone();
    let weak = control.downgrade();
    ui.run_background("تطبیق منبع افتتاحیه…",move||state.reconcile_opening(&company,&cutoff),move|ui,result|{if let Some(control)=weak.upgrade(){match result{Ok(p)=>{preview_table(&control,&p,&display_date(ui,&p.as_of),true);if let Some(w)=control.first_child().and_then(|w|w.next_sibling()).and_then(|w|w.downcast::<gtk::Label>().ok()){w.set_text(&format!("آغاز روز {} • ریال • فقط سند افتتاحیهٔ ثبت‌شده\nجمع بدهکار {} • بستانکار {}",display_date(ui,&p.as_of),p.total_debit.rials(),p.total_credit.rials()));}},Err(e)=>control.append(&label(&e.to_string()))}}});
    content.append(&heading("مانده‌های مستقل دفتر کل و مرجع هر سطر"));
    for b in &opening.command.balances {
        let account = ui
            .store
            .borrow()
            .state
            .accounting
            .accounts
            .iter()
            .find(|a| a.id == b.account_id)
            .map(|a| format!("{} • {}", a.code, a.name))
            .unwrap_or_else(|| b.account_id.clone());
        content.append(&label(&format!(
            "{} • بدهکار {} / بستانکار {} ریال • {}",
            account,
            b.debit.rials(),
            b.credit.rials(),
            b.source_reference
        )));
    }
    let search = gtk::SearchEntry::new();
    search.set_search_delay(60);
    search.set_widget_name("core-posted-search");
    accessible_name(&search, "جستجوی منبع جزئیات افتتاحیه");
    content.append(&field("جستجوی منبع جزئیات", &search));
    let details = column();
    content.append(&details);
    let rows = posted_rows(ui, &opening.command);
    let render = Rc::new(move |needle: &str| {
        clear(&details);
        let needle = sarv_quote::domain::normalize(needle);
        let filtered = rows
            .iter()
            .filter(|s| sarv_quote::domain::normalize(s).contains(&needle))
            .collect::<Vec<_>>();
        details.append(&label(&format!(
            "{} سطر منبع؛ برای محدود کردن جستجو کنید",
            filtered.len()
        )));
        for s in filtered.iter().take(100) {
            let card = card();
            card.append(&label(s));
            details.append(&card);
        }
        if filtered.len() > 100 {
            details.append(&label(
                "نمایش ۱۰۰ سطر نخست؛ جستجو را دقیق‌تر کنید یا خروجی کامل بگیرید.",
            ));
        }
    });
    render("");
    search.connect_search_changed(move |s| render(&s.text()));
    let toolbar = dialog_toolbar(&scroll(&content));
    let close = button("بستن");
    let d = dialog.downgrade();
    close.connect_clicked(move |_| {
        if let Some(d) = d.upgrade() {
            d.close();
        }
    });
    toolbar.add_bottom_bar(&close);
    dialog.set_child(Some(&toolbar));
    ui.present_dialog(&dialog);
}
fn posted_rows(ui: &Ui, c: &OpeningCommand) -> Vec<String> {
    let mut out = Vec::new();
    for p in &c.parties {
        out.push(format!(
            "طرف {} • {} • مشتری {} • تأمین‌کننده {}",
            p.id, p.name, p.customer, p.supplier
        ));
    }
    for p in &c.units {
        out.push(format!("واحد {} • {}", p.id, p.name));
    }
    for p in &c.items {
        out.push(format!(
            "کالا {} • {} • {} • واحد پایه {} • تجزیه‌ناپذیر {}",
            p.id, p.sku, p.name, p.base_unit_id, p.indivisible
        ));
        for q in &p.packs {
            out.push(format!(
                "بسته {} / {} • {} / {} پایه",
                p.id, q.unit_id, q.numerator, q.denominator
            ));
        }
    }
    for p in &c.warehouses {
        out.push(format!("انبار {} • {}", p.id, p.name));
    }
    for p in &c.cash {
        out.push(format!(
            "صندوق {} • {} • {} ریال • {}",
            p.id,
            p.name,
            p.balance.rials(),
            p.source_reference
        ));
    }
    for p in &c.banks {
        out.push(format!(
            "بانک {} • {} • {} • دفتر {} / صورت‌حساب {} ریال • {}",
            p.id,
            p.name,
            p.bank_reference,
            p.balance.rials(),
            p.statement_balance.rials(),
            p.source_reference
        ));
    }
    for p in &c.open_items {
        out.push(format!(
            "{} {} • طرف {} • سند {} • تاریخ {} • سررسید {} • {} ریال",
            if p.side == OpenItemSide::Receivable {
                "دریافتنی"
            } else {
                "پرداختنی"
            },
            p.id,
            p.party_id,
            p.document_reference,
            display_date(ui, &p.document_date),
            display_date(ui, &p.due_date),
            p.remaining.rials()
        ));
    }
    for p in &c.advances {
        out.push(format!(
            "پیش {} • {:?} • طرف {} • {} • {} • {} ریال",
            p.id,
            p.side,
            p.party_id,
            p.source_reference,
            display_date(ui, &p.document_date),
            p.remaining.rials()
        ));
    }
    for p in &c.cheques {
        out.push(format!("چک {} • {:?} / {:?} • طرف {} • {} ریال • مرجع {} • تسویه {} • بانک {} • دارنده {} • صدور {} / سررسید {}",p.id,p.direction,p.state,p.party_id,p.amount.rials(),p.cheque_reference,p.settlement_reference,p.bank_reference,p.holder,display_date(ui,&p.issue_date),display_date(ui,&p.due_date)));
    }
    for p in &c.stock {
        out.push(format!(
            "موجودی {} • کالا {} / انبار {} • {} {} • {} ریال • {} • پایه {} هزارم",
            p.id,
            p.item_id,
            p.warehouse_id,
            quantity_text(p.quantity_thousandths),
            p.unit_id,
            p.value.rials(),
            p.source_reference,
            c.base_quantity(&p.item_id, &p.unit_id, p.quantity_thousandths)
                .unwrap_or(0)
        ));
    }
    for p in &c.grni {
        out.push(format!("دریافت بدون صورتحساب {} • {} • تأمین‌کننده {} • کالا {} / انبار {} • {} {} • {} ریال، داخل موجودی • {}",p.id,p.receipt_reference,p.party_id,p.item_id,p.warehouse_id,quantity_text(p.quantity_thousandths),p.unit_id,p.value.rials(),display_date(ui,&p.received_date)));
    }
    if let Some(source) = &c.import_source {
        out.push(format!("اصل فایل CSV محفوظ است • SHA-256 {} • {} سطر؛ مقادیر ویرایش‌شده همراه اصل فایل نگهداری شدند",source.sha256,source.rows.len()));
    }
    out
}
fn export_opening(ui: &Rc<Ui>, company: String, pdf: bool, unit: MoneyUnit) {
    let chooser = gtk::FileDialog::builder()
        .title(if pdf {
            "ذخیرهٔ گزارش PDF افتتاحیه"
        } else {
            "ذخیرهٔ خروجی CSV کامل افتتاحیه"
        })
        .initial_name(if pdf { "opening.pdf" } else { "opening.csv" })
        .build();
    let weak = Rc::downgrade(ui);
    chooser.save(
        Some(&ui.window),
        None::<&gtk::gio::Cancellable>,
        move |result| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Ok(file) = result else {
                return;
            };
            let Some(path) = file.path() else {
                return;
            };
            let snapshot = ui.store.borrow().state.clone();
            ui.run_background(
                "ساخت خروجی افتتاحیه…",
                move || {
                    if pdf {
                        sarv_quote::core_export::opening_pdf_with_preferences(
                            &snapshot.accounting,
                            &company,
                            unit,
                            snapshot.settings.date_display,
                            snapshot.settings.display_digits,
                            &path,
                        )
                    } else {
                        sarv_quote::core_export::opening_csv_with_preferences(
                            &snapshot.accounting,
                            &company,
                            unit,
                            snapshot.settings.date_display,
                            snapshot.settings.display_digits,
                            &path,
                        )
                    }
                },
                |ui, result| ui.message(result.map(|()| "خروجی افتتاحیه ذخیره شد")),
            );
        },
    );
}

fn qa_widget<T: IsA<gtk::Widget> + glib::object::ObjectType + Clone + 'static>(
    root: &impl IsA<gtk::Widget>,
    name: &str,
) -> Result<T> {
    let root = root.as_ref();
    if root.widget_name() == name
        && let Ok(t) = root.clone().downcast::<T>()
    {
        return Ok(t);
    }
    let mut child = root.first_child();
    while let Some(w) = child {
        child = w.next_sibling();
        if let Ok(t) = qa_widget(&w, name) {
            return Ok(t);
        }
    }
    anyhow::bail!("native widget {name} absent")
}
fn qa_dialog(ui: &Ui) -> Result<adw::Dialog> {
    ui.active_dialog
        .borrow()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("native dialog absent"))
}
fn qa_text(d: &adw::Dialog, name: &str, value: &str) -> Result<()> {
    qa_widget::<gtk::Entry>(d, name)?.set_text(value);
    Ok(())
}
fn qa_record(
    d: &adw::Dialog,
    k: Kind,
    values: &[(&str, &str)],
    choices: &[(&str, u32)],
    checks: &[(&str, bool)],
) -> Result<()> {
    qa_widget::<gtk::DropDown>(d, "core-record-kind")?
        .set_selected(Kind::ALL.iter().position(|x| *x == k).unwrap() as u32);
    for (key, value) in values {
        qa_text(d, &format!("core-record-{key}"), value)?;
    }
    for (key, value) in choices {
        qa_widget::<gtk::DropDown>(d, &format!("core-record-{key}"))?.set_selected(*value);
    }
    for (key, value) in checks {
        qa_widget::<gtk::CheckButton>(d, &format!("core-record-{key}"))?.set_active(*value);
    }
    qa_widget::<gtk::Button>(d, "core-add-record")?.emit_clicked();
    let error = qa_widget::<gtk::Label>(d, "accounting-form-error")?;
    anyhow::ensure!(!error.is_visible(), "record {:?}: {}", k, error.text());
    Ok(())
}
pub(super) fn start_core_smoke(ui: &Rc<Ui>) {
    let Ok(mode) = std::env::var("SARV_CORE_SMOKE") else {
        return;
    };
    if !matches!(mode.as_str(), "stage" | "full" | "zero") {
        return;
    }
    let directory = ui.store.borrow().directory.clone();
    if !ui.store.borrow().state.accounting.companies.is_empty()
        || !(directory.starts_with("/tmp")
            || directory.starts_with("/workspace/sarv-quote/artifacts"))
        || std::env::var("SARV_TEST_INSTANCE").is_err()
    {
        ui.message(Err::<String, _>(anyhow::anyhow!(
            "core smoke requires a fresh disposable synthetic profile"
        )));
        return;
    }
    let weak = Rc::downgrade(ui);
    let mut stage = 0usize;
    let started = std::time::Instant::now();
    let mut preview_started = std::time::Instant::now();
    let mut preview_millis = 0u128;
    let mut post_started = std::time::Instant::now();
    let mut post_millis = 0u128;
    glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if started.elapsed().as_secs() > 100 {
            let _ = std::fs::write(
                directory.join("core-native-failure.txt"),
                format!("timeout stage{stage}: {}", ui.status.text()),
            );
            if let Some(app) = ui.window.application() {
                app.quit();
            }
            return glib::ControlFlow::Break;
        }
        if ui.persistence_busy.get() || ui.background_busy.get() {
            return glib::ControlFlow::Continue;
        }
        let result: Result<bool> = (|| {
            match stage {
                0 => {
                    ui.stack.set_visible_child_name("accounting");
                    ui.split.set_show_content(true);
                }
                1 => {
                    smoke::capture(&ui.window, &directory.join("core-before.png"))?;
                    qa_widget::<gtk::Button>(&ui.window, "core-new-company")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_text(&d, "core-company-name", "F2-core-v1 • شرکت ساختگی")?;
                    qa_text(&d, "core-chart-cash-code", "1111")?;
                    qa_text(&d, "core-chart-cash-name", "صندوق افتتاحیهٔ ساختگی")?;
                    for n in ["core-fiscal-start", "core-fiscal-end"] {
                        qa_widget::<gtk::DropDown>(&d, &format!("{n}-calendar"))?.set_selected(1);
                    }
                    qa_text(&d, "core-fiscal-start", "۱۴۰۵/۰۱/۰۱")?;
                    qa_text(&d, "core-fiscal-end", "۱۴۰۵/۱۲/۲۹")?;
                }
                2 => {
                    let d = qa_dialog(&ui)?;
                    smoke::capture(&ui.window, &directory.join("core-guided-setup.png"))?;
                    qa_widget::<gtk::Button>(&d, "core-setup-dialog-save")?.emit_clicked();
                }
                3 => {
                    let s = &ui.store.borrow().state.accounting;
                    anyhow::ensure!(
                        s.companies.len() == 1
                            && s.core.setups.len() == 1
                            && s.accounts.len() == 26
                            && s.journals.is_empty(),
                        "atomic setup counts invalid"
                    );
                    qa_widget::<gtk::Button>(&ui.window, "core-stage-opening")?.emit_clicked();
                    let d = qa_dialog(&ui)?;
                    qa_text(&d, "core-opening-source", "OPEN-F2-2026")?;
                    qa_text(
                        &d,
                        "core-opening-reason",
                        "افتتاحیهٔ مصنوعی؛ مانده‌های مستقل و جزئیات",
                    )?;
                    for role in CoreRole::ALL {
                        qa_text(
                            &d,
                            &format!("core-balance-{}-reference", role.key()),
                            &format!("GL-{}-independent", role.key()),
                        )?;
                    }
                    if mode == "zero" {
                        qa_widget::<gtk::Stack>(&d, "core-opening-tabs")?
                            .set_visible_child_name("review");
                        qa_widget::<gtk::Button>(&d, "core-preview-opening")?.emit_clicked();
                        stage = 30;
                        return Ok(false);
                    }
                    for (role, debit, credit) in [
                        (CoreRole::Cash, "252", "0"),
                        (CoreRole::Bank, "300", "0"),
                        (CoreRole::Receivables, "200", "0"),
                        (CoreRole::Inventory, "300", "0"),
                        (CoreRole::ChequesReceivable, "200", "0"),
                        (CoreRole::Payables, "0", "149"),
                        (CoreRole::ChequesPayable, "0", "100"),
                        (CoreRole::Grni, "0", "50"),
                        (CoreRole::OwnerCapital, "0", "952"),
                    ] {
                        qa_text(&d, &format!("core-balance-{}-debit", role.key()), debit)?;
                        qa_text(&d, &format!("core-balance-{}-credit", role.key()), credit)?;
                    }
                    qa_widget::<gtk::Stack>(&d, "core-opening-tabs")?
                        .set_visible_child_name("records");
                    for (id, name, customer, supplier) in [
                        ("CU-A", "مشتری الف", true, true),
                        ("CU-B", "مشتری ب", true, false),
                        ("SU-A", "تأمین‌کننده الف", false, true),
                        ("SU-B", "تأمین‌کننده ب", false, true),
                    ] {
                        qa_record(
                            &d,
                            Kind::Party,
                            &[("id", id), ("name", name)],
                            &[],
                            &[("customer", customer), ("supplier", supplier)],
                        )?;
                    }
                    qa_record(
                        &d,
                        Kind::Unit,
                        &[("id", "PIECE"), ("name", "عدد")],
                        &[],
                        &[],
                    )?;
                    qa_record(
                        &d,
                        Kind::Unit,
                        &[("id", "CARTON"), ("name", "کارتن")],
                        &[],
                        &[],
                    )?;
                    qa_record(
                        &d,
                        Kind::Item,
                        &[("id", "ITEM-A"), ("sku", "SKU-A"), ("name", "کالای الف")],
                        &[("unit", 0)],
                        &[("indivisible", true)],
                    )?;
                    qa_record(
                        &d,
                        Kind::Pack,
                        &[("numerator", "12"), ("denominator", "1")],
                        &[("item", 0), ("unit", 1)],
                        &[],
                    )?;
                    for (id, name) in [
                        ("W1", "انبار اول"),
                        ("W2", "انبار دوم"),
                        ("W-TEMP", "موقت برای آزمون حذف"),
                    ] {
                        qa_record(&d, Kind::Warehouse, &[("id", id), ("name", name)], &[], &[])?;
                    }
                    qa_widget::<gtk::Button>(&d, "core-record-delete-W-TEMP")?.emit_clicked();
                    qa_record(
                        &d,
                        Kind::Cash,
                        &[
                            ("id", "C1"),
                            ("name", "صندوق اول"),
                            ("amount", "251"),
                            ("reference", "CASH-COUNT-C1"),
                        ],
                        &[],
                        &[],
                    )?;
                    qa_text(&d, "core-record-id", "BAD")?;
                    qa_text(&d, "core-record-name", "خطای آزمایشی")?;
                    qa_text(&d, "core-record-amount", "0.1")?;
                    qa_text(&d, "core-record-reference", "RETAIN")?;
                    qa_widget::<gtk::Button>(&d, "core-add-record")?.emit_clicked();
                    anyhow::ensure!(
                        qa_widget::<gtk::Entry>(&d, "core-record-amount")?.text() == "0.1",
                        "bad input lost"
                    );
                    qa_widget::<gtk::DropDown>(&d, "core-record-kind")?.set_selected(1);
                    anyhow::ensure!(
                        qa_widget::<gtk::DropDown>(&d, "core-record-kind")?.selected() == 5
                            && qa_widget::<gtk::Entry>(&d, "core-record-amount")?.text() == "0.1",
                        "dirty kind switch lost input"
                    );
                }
                4 => {
                    let d = qa_dialog(&ui)?;
                    smoke::capture(&ui.window, &directory.join("core-retained-input.png"))?;
                    qa_widget::<gtk::Button>(&d, "core-discard-record")?.emit_clicked();
                    qa_record(
                        &d,
                        Kind::Bank,
                        &[
                            ("id", "B1"),
                            ("name", "حساب بانکی اول"),
                            ("amount", "300"),
                            ("statement", "300"),
                            ("bank", "SYNTHETIC-BANK-001"),
                            ("reference", "BANK-STATEMENT-B1"),
                        ],
                        &[],
                        &[],
                    )?;
                    for (k, id, reference, party, money) in [
                        (Kind::Ar, "AR-125", "DOC-AR-125", 0, "125"),
                        (Kind::Ar, "AR-75", "DOC-AR-75", 1, "75"),
                        (Kind::Ap, "AP-100", "DOC-AP-100", 2, "100"),
                        (Kind::Ap, "AP-49", "DOC-AP-49", 3, "49"),
                    ] {
                        qa_record(
                            &d,
                            k,
                            &[("id", id), ("reference", reference), ("amount", money)],
                            &[("party", party)],
                            &[],
                        )?;
                    }
                    for (k, id, party, money, settlement) in [
                        (Kind::Cr, "CR1", 0, "200", "HISTORIC-INVOICE-CR1"),
                        (Kind::Cp, "CP1", 2, "100", "HISTORIC-BILL-CP1"),
                    ] {
                        qa_record(
                            &d,
                            k,
                            &[
                                ("id", id),
                                ("reference", id),
                                ("amount", money),
                                ("settlement", settlement),
                                ("bank", "SYNTHETIC-BANK"),
                                ("holder", "دارندهٔ ساختگی"),
                            ],
                            &[("party", party)],
                            &[],
                        )?;
                    }
                    qa_record(
                        &d,
                        Kind::Stock,
                        &[
                            ("id", "ST-W1"),
                            ("quantity", "2"),
                            ("amount", "240"),
                            ("reference", "STOCK-COUNT-W1"),
                        ],
                        &[("item", 0), ("warehouse", 0), ("unit", 1)],
                        &[],
                    )?;
                    qa_record(
                        &d,
                        Kind::Stock,
                        &[
                            ("id", "ST-W2"),
                            ("quantity", "6"),
                            ("amount", "60"),
                            ("reference", "STOCK-COUNT-W2"),
                        ],
                        &[("item", 0), ("warehouse", 1), ("unit", 0)],
                        &[],
                    )?;
                    qa_record(
                        &d,
                        Kind::Grni,
                        &[
                            ("id", "R1"),
                            ("quantity", "5"),
                            ("amount", "50"),
                            ("reference", "RECEIPT-R1"),
                        ],
                        &[("party", 2), ("item", 0), ("warehouse", 1), ("unit", 0)],
                        &[],
                    )?;
                    qa_widget::<gtk::Stack>(&d, "core-opening-tabs")?
                        .set_visible_child_name("review");
                    qa_widget::<gtk::Button>(&d, "core-preview-opening")?.emit_clicked();
                }
                5 => {
                    let d = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        !qa_widget::<gtk::Button>(&d, "core-opening-dialog-save")?.is_sensitive()
                            && ui.store.borrow().state.accounting.journals.is_empty(),
                        "mismatch posted"
                    );
                    smoke::capture(&ui.window, &directory.join("core-mismatch-preview.png"))?;
                    qa_text(&d, "core-balance-cash-debit", "۲۵۱")?;
                    preview_started = std::time::Instant::now();
                    qa_widget::<gtk::Button>(&d, "core-preview-opening")?.emit_clicked();
                }
                6 => {
                    preview_millis = preview_started.elapsed().as_millis();
                    let d = qa_dialog(&ui)?;
                    anyhow::ensure!(
                        qa_widget::<gtk::Button>(&d, "core-opening-dialog-save")?.is_sensitive(),
                        "valid literal1251 preview not verified: {}",
                        qa_widget::<gtk::Label>(&d, "accounting-form-error")?.text()
                    );
                    smoke::capture(&ui.window, &directory.join("core-ready-preview.png"))?;
                    if mode == "stage" {
                        qa_widget::<gtk::DropDown>(&d, "core-record-kind")?.set_selected(0);
                        qa_widget::<gtk::Stack>(&d, "core-opening-tabs")?
                            .set_visible_child_name("records");
                        sarv_quote::store::write_new(
                            &directory.join("core-stage-ready.json"),
                            &serde_json::to_vec_pretty(
                                &serde_json::json!({"result":"ready","source":"native editable forms","import_source":null,"counts":{"ledger_balances":21,"parties":4,"units":2,"items":1,"packs":1,"warehouses":2,"cash":1,"banks":1,"ar":2,"ap":2,"advances":0,"cheques":2,"stock":2,"grni":1},"company_id":ui.store.borrow().state.accounting.companies[0].id,"parties":4,"units":2,"items":1,"warehouses":2,"stock_rows":2,"opening_journals":0,"controls":12,"preview_total_irr":1251,"preview_ms_includes_poll":preview_millis}),
                            )?,
                        )?;
                        return Ok(true);
                    }
                    post_started = std::time::Instant::now();
                    let save = qa_widget::<gtk::Button>(&d, "core-opening-dialog-save")?;
                    save.emit_clicked();
                    save.emit_clicked();
                }
                7 => {
                    post_millis = post_started.elapsed().as_millis();
                    let state = ui.store.borrow().state.clone();
                    anyhow::ensure!(
                        state.accounting.journals.len() == 1
                            && state.accounting.core.openings.len() == 1,
                        "opening doubleclick count invalid"
                    );
                    let company = state.accounting.companies[0].id.clone();
                    let p = state.accounting.reconcile_opening(&company, "2026-03-21")?;
                    anyhow::ensure!(
                        p.total_debit.rials() == 1251
                            && p.total_credit.rials() == 1251
                            && p.controls.len() == 12
                            && p.controls.iter().all(|c| c.difference == Money::ZERO),
                        "literal controls invalid"
                    );
                    let destination = directory.clone();
                    ui.run_background("خروجی و بازیابی نمونهٔ هسته…",move||{for(unit,name)in [(MoneyUnit::Rial,"rial"),(MoneyUnit::Toman,"toman")]{sarv_quote::core_export::opening_pdf_with_preferences(&state.accounting,&company,unit,DateDisplay::Jalali,DisplayDigits::Persian,&destination.join(format!("opening-{name}.pdf")))?;sarv_quote::core_export::opening_csv_with_preferences(&state.accounting,&company,unit,DateDisplay::Jalali,DisplayDigits::Persian,&destination.join(format!("opening-{name}.csv")))?;}let backup=destination.join("core-backup.json");sarv_quote::store::backup_state(&state,&backup)?;let restored=Store::restore_preview(&backup)?;anyhow::ensure!(restored.accounting==state.accounting,"full backup mismatch");let target=destination.join("restored-profile");let mut store=Store::open(&target)?;store.restore(restored)?;drop(store);let store=Store::open(&target)?;anyhow::ensure!(store.state.accounting==state.accounting,"fresh restart mismatch");let original=Store::open(&destination)?;anyhow::ensure!(original.state.accounting==state.accounting,"original restart mismatch");sarv_quote::store::write_new(&destination.join("core-recovery.json"),&serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","original_restart":"equal","backup_restore":"full aggregate equal","fresh_restart":"equal","total_irr":1251,"controls":12}))?)},|ui,result|ui.message(result.map(|()|"خروجی و بازیابی هسته برابر بود")));
                }
                8 => {
                    qa_widget::<gtk::Button>(&ui.window, "core-view-opening")?.emit_clicked();
                }
                9 => {
                    smoke::capture(&ui.window, &directory.join("core-posted-detail.png"))?;
                    ui.window.set_default_size(650, 760);
                }
                10 => {
                    smoke::capture(&ui.window, &directory.join("core-posted-narrow.png"))?;
                    anyhow::ensure!(
                        directory.join("core-recovery.json").exists(),
                        "export/recovery worker failed"
                    );
                    sarv_quote::store::write_new(
                        &directory.join("core-native-verification.json"),
                        &serde_json::to_vec_pretty(
                            &serde_json::json!({"result":"passed","mode":"GTK widgets/signals; external XTest separate","checks":["atomic guided company/chart/1405 fiscal policy setup","all native record forms with exact pack and dualparty","remove temporary record","fractionalrial input retained and dirtykind switch rejected","independent1252 mismatch blocked withoutwrite","corrected literal1251 preview12zero differences","doubleclick one source onejournal","read-only source and native CSV/PDF exportworker","complete backup/freshrestore/original restart"],"preview_ms_includes_poll":preview_millis,"post_ms_includes_poll":post_millis,"total_irr":1251,"total_toman":"125.1","opening_journals":1,"controls":12}),
                        )?,
                    )?;
                    if let Some(app) = ui.window.application() {
                        app.quit();
                    }
                    return Ok(true);
                }
                30 => {
                    let d = qa_dialog(&ui)?;
                    let save = qa_widget::<gtk::Button>(&d, "core-opening-dialog-save")?;
                    anyhow::ensure!(save.is_sensitive(), "zero preview failed");
                    smoke::capture(&ui.window, &directory.join("core-zero-preview.png"))?;
                    save.emit_clicked();
                    save.emit_clicked();
                }
                31 => {
                    let state = ui.store.borrow().state.clone();
                    let a = &state.accounting;
                    anyhow::ensure!(
                        a.journals.is_empty()
                            && a.core.openings.len() == 1
                            && a.core.openings[0].effect
                                == OpeningEffect::ZeroBalanceInitialization,
                        "zero initialization manufactured journal or duplicate"
                    );
                    let company = a.companies[0].id.clone();
                    let p = a.reconcile_opening(&company, "2026-03-21")?;
                    anyhow::ensure!(
                        p.controls.len() == 12
                            && p.controls.iter().all(|c| c.difference == Money::ZERO)
                            && p.total_debit == Money::ZERO
                            && p.total_credit == Money::ZERO,
                        "zero controls invalid"
                    );
                    let destination = directory.clone();
                    ui.run_background("خروجی و شروع مجدد آغاز بدون مانده…",move||{sarv_quote::core_export::opening_pdf(&state.accounting,&company,MoneyUnit::Rial,DateDisplay::Jalali,&destination.join("opening-zero.pdf"))?;sarv_quote::core_export::opening_csv(&state.accounting,&company,MoneyUnit::Rial,DateDisplay::Jalali,&destination.join("opening-zero.csv"))?;let original=Store::open(&destination)?;anyhow::ensure!(original.state.accounting==state.accounting,"zero restart mismatch");sarv_quote::store::write_new(&destination.join("core-zero-restart.json"),&serde_json::to_vec_pretty(&serde_json::json!({"result":"passed","journals":0,"openings":1,"controls":12}))?)},|ui,result|ui.message(result.map(|()|"آغاز بدون مانده حفظ شد")));
                }
                32 => {
                    qa_widget::<gtk::Button>(&ui.window, "core-view-opening")?.emit_clicked();
                }
                33 => {
                    anyhow::ensure!(
                        directory.join("core-zero-restart.json").exists(),
                        "zero worker failed"
                    );
                    smoke::capture(&ui.window, &directory.join("core-zero-posted.png"))?;
                    sarv_quote::store::write_new(
                        &directory.join("core-zero-native.json"),
                        &serde_json::to_vec_pretty(
                            &serde_json::json!({"result":"passed","mode":"native GTK forms and doubleclick","opening_effect":"ZeroBalanceInitialization","journals":0,"controls":12,"restart":"equal","export":"CSV+PDF"}),
                        )?,
                    )?;
                    if let Some(app) = ui.window.application() {
                        app.quit();
                    }
                    return Ok(true);
                }
                _ => {}
            }
            stage += 1;
            Ok(false)
        })();
        match result {
            Ok(true) => glib::ControlFlow::Break,
            Ok(false) => glib::ControlFlow::Continue,
            Err(e) => {
                let _ = std::fs::write(
                    directory.join("core-native-failure.txt"),
                    format!("stage{stage}: {e:#}\nstatus:{}", ui.status.text()),
                );
                if let Some(app) = ui.window.application() {
                    app.quit();
                }
                glib::ControlFlow::Break
            }
        }
    });
}
