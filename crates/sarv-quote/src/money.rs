//! Exact rial money via `rust_decimal` (scale 0).
//! SQLite and legacy domain fields still use integer rials; convert at boundaries.

use anyhow::{bail, Result};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::MAX_MONEY;

/// Exact whole-rial amount (scale 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(Decimal);

impl Money {
    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    pub fn from_rial(rial: i64) -> Result<Self> {
        if !(0..=MAX_MONEY).contains(&rial) {
            bail!("مبلغ خارج از محدوده مجاز است");
        }
        Ok(Self(Decimal::from(rial)))
    }

    pub fn to_rial(self) -> Result<i64> {
        if self.0.scale() > 0 && self.0.fract() != Decimal::ZERO {
            bail!("مبلغ باید عدد صحیح ریال باشد");
        }
        let rial = self
            .0
            .to_i64()
            .ok_or_else(|| anyhow::anyhow!("مبلغ قابل تبدیل به ریال نیست"))?;
        if !(0..=MAX_MONEY).contains(&rial) {
            bail!("مبلغ خارج از محدوده مجاز است");
        }
        Ok(rial)
    }

    pub fn as_decimal(self) -> Decimal {
        self.0
    }

    pub fn from_decimal(value: Decimal) -> Result<Self> {
        if value.scale() > 0 && value.fract() != Decimal::ZERO {
            bail!("مبلغ باید عدد صحیح ریال باشد");
        }
        if value.is_sign_negative() {
            bail!("مبلغ نمی‌تواند منفی باشد");
        }
        let normalized = value.trunc();
        let rial = normalized
            .to_i64()
            .ok_or_else(|| anyhow::anyhow!("مبلغ قابل تبدیل به ریال نیست"))?;
        Self::from_rial(rial)
    }
}

impl From<Money> for Decimal {
    fn from(value: Money) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rial_round_trip() {
        let m = Money::from_rial(12_500).unwrap();
        assert_eq!(m.to_rial().unwrap(), 12_500);
    }
}
