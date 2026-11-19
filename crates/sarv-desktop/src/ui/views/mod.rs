pub mod accounting;
pub mod banking;
pub mod contacts;
pub mod inventory;
pub mod lock;
pub mod overview;
pub mod payments;
pub mod pos;
pub mod purchases;
pub mod recovery;
pub mod reports;
pub mod sales;
pub mod settings;
pub mod setup;
pub mod sheets;

pub fn ensure_page(st: &mut crate::app::State, page: crate::app::Page) {
    use crate::app::Page;
    match page {
        Page::Sales => sales::ensure(st),
        Page::Purchases => purchases::ensure(st),
        Page::Payments => payments::ensure(st),
        Page::Banking => banking::ensure(st),
        Page::Contacts => contacts::ensure(st),
        Page::Inventory => inventory::ensure(st),
        Page::Accounting => accounting::ensure(st),
        Page::Reports => reports::ensure(st),
        Page::Sheets => sheets::ensure(st),
        Page::Overview | Page::Pos | Page::Settings => {}
    }
}
