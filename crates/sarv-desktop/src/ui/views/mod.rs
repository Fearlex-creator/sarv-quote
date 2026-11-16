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

pub fn ensure_rows(st: &mut crate::app::State) {
    sales::ensure(st);
    purchases::ensure(st);
    payments::ensure(st);
    banking::ensure(st);
    contacts::ensure(st);
    inventory::ensure(st);
    accounting::ensure(st);
    reports::ensure(st);
}
