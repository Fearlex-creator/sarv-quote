pub mod accounting;
#[cfg(feature = "native-gtk")]
pub mod accounting_export;
pub mod calendar;
pub mod core;
#[cfg(feature = "native-gtk")]
pub mod core_export;
pub mod core_import;
pub mod desktop;
pub mod domain;
#[cfg(feature = "native-gtk")]
pub mod export;
pub mod imports;
pub mod store;

pub mod procurement;
#[cfg(feature = "native-gtk")]
pub mod procurement_export;

pub mod sales;
pub mod sales_draft;

pub mod agent;
#[cfg(feature = "native-gtk")]
pub mod sales_export;
