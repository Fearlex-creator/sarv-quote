use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Editable contact details are separate from immutable invoice snapshots.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PartyProfile {
    pub company_id: String,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub organization: String,
    pub phone: String,
    pub mobile: String,
    pub email: String,
    pub address: String,
    pub city: String,
    pub postal_code: String,
    pub national_id: String,
    pub tax_id: String,
    pub website: String,
    pub notes: String,
}
impl PartyProfile {
    pub fn validate(&self) -> Result<()> {
        for text in [
            &self.company_id,
            &self.display_name,
            &self.first_name,
            &self.last_name,
            &self.organization,
            &self.phone,
            &self.mobile,
            &self.email,
            &self.city,
            &self.postal_code,
            &self.national_id,
            &self.tax_id,
            &self.website,
        ] {
            ensure!(
                text.chars().count() <= 160 && !text.chars().any(char::is_control),
                "Contact field is too long or contains control characters"
            );
        }
        ensure!(
            !self.company_id.trim().is_empty(),
            "Contact company required"
        );
        for text in [&self.address, &self.notes] {
            ensure!(text.chars().count() <= 2000, "Contact text is too long");
        }
        ensure!(
            self.email.is_empty()
                || (self.email.contains('@') && !self.email.contains(char::is_whitespace)),
            "Invalid contact email"
        );
        Ok(())
    }
}
