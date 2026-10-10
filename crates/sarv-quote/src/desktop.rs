//! Accounting dispatcher and stored preference compatibility.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all="camelCase")]
pub struct DesktopPreferences {
    pub completed: bool, pub active_company: String, pub profile: String, pub language: String,
    pub calendar: String, pub theme: String, pub audit: Vec<DesktopAudit>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="camelCase")]
pub struct DesktopAudit { pub id: String, pub at: String, pub operation: String, pub detail: String }
impl DesktopPreferences {
    pub fn is_empty(&self)->bool { self == &Self::default() }
    pub fn validate(&self)->Result<()> {
        ensure!(["", "fa", "en"].contains(&self.language.as_str()), "زبان نامعتبر");
        ensure!(["", "Jalali", "Gregorian"].contains(&self.calendar.as_str()), "تقویم نامعتبر");
        ensure!(["", "light", "dark", "system"].contains(&self.theme.as_str()), "ظاهر نامعتبر");
        ensure!(self.audit.len() <= 100_000, "سابقه بسیار بزرگ است"); Ok(())
    }
    pub fn validate_transition(&self,next:&Self)->Result<()> {ensure!(next.audit.starts_with(&self.audit),"سابقه برنامه قابل حذف یا ویرایش نیست");Ok(())}
}

#[path = "desktop_bridge.rs"]
mod bridge;
pub use bridge::{dispatch, request_id, seed_fixture, wire};
