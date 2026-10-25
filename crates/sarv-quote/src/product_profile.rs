use crate::accounting::Money;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// Commercial defaults do not change the carried cost of posted inventory.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ProductProfile {
    pub company_id: String,
    pub purchase_price: Option<Money>,
    pub sale_price: Option<Money>,
    pub barcode: String,
    pub category: String,
    pub brand: String,
    pub minimum_stock_thousandths: i64,
    pub preferred_supplier_id: String,
    pub notes: String,
}
impl ProductProfile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.company_id.trim().is_empty(),
            "Product company required"
        );
        for s in [
            &self.company_id,
            &self.barcode,
            &self.category,
            &self.brand,
            &self.preferred_supplier_id,
        ] {
            ensure!(
                s.chars().count() <= 160 && !s.chars().any(char::is_control),
                "Invalid product field"
            );
        }
        for price in [self.purchase_price, self.sale_price].into_iter().flatten() {
            ensure!(
                (0..=crate::domain::MAX_MONEY).contains(&price.rials()),
                "Product price must be nonnegative and in range"
            );
        }
        ensure!(
            (0..=1_000_000_000).contains(&self.minimum_stock_thousandths),
            "Invalid minimum stock"
        );
        ensure!(
            self.notes.chars().count() <= 2000,
            "Product notes are too long"
        );
        Ok(())
    }
}
