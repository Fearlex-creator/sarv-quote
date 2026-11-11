//! Shared resource create helpers (customer / vendor / item / unit / warehouse).

use serde_json::json;

use crate::app::{State, Tone};
use crate::data;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Customer,
    Vendor,
    Item,
    Unit,
    Warehouse,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ResourceKind::Customer => "customer",
            ResourceKind::Vendor => "vendor",
            ResourceKind::Item => "item",
            ResourceKind::Unit => "unit",
            ResourceKind::Warehouse => "warehouse",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ResourceKind::Customer => "New customer",
            ResourceKind::Vendor => "New supplier",
            ResourceKind::Item => "New product",
            ResourceKind::Unit => "New unit",
            ResourceKind::Warehouse => "New warehouse",
        }
    }
}

impl State {
    pub fn uid(&self) -> String {
        format!("ui-{}", chrono::Utc::now().timestamp_millis())
    }

    pub fn setup_id(&self) -> String {
        self.books.workspace["setups"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|s| s["id"].as_str())
            .unwrap_or("")
            .to_string()
    }

    /// Create a party and return its id.
    pub fn create_party(&mut self, kind: ResourceKind, name: &str, phone: &str, email: &str, notes: &str) -> Result<String, String> {
        let kind_s = match kind {
            ResourceKind::Customer => "customer",
            ResourceKind::Vendor => "vendor",
            _ => return Err("Not a contact kind".into()),
        };
        let id = self.uid();
        self.run(
            "resource",
            json!({
                "kind": kind_s,
                "id": id,
                "name": name,
                "profile": {
                    "phone": phone,
                    "email": email,
                    "notes": notes,
                    "display_name": name
                },
                "reason": "Register contact"
            }),
            Some("Contact saved"),
        )?;
        Ok(id)
    }

    /// Create an item (+ product profile) and return its id.
    pub fn create_item(
        &mut self,
        name: &str,
        sku: &str,
        unit_id: &str,
        sale_price_rial: &str,
        purchase_price_rial: Option<&str>,
        barcode: &str,
        notes: &str,
        preferred_supplier_id: &str,
        brand: &str,
        category: &str,
        minimum_stock: &str,
        service: bool,
    ) -> Result<String, String> {
        let id = self.uid();
        let category = if service && category.trim().is_empty() {
            "Service"
        } else {
            category
        };
        let min_stock = if minimum_stock.trim().is_empty() {
            "0"
        } else {
            minimum_stock
        };
        let mut payload = json!({
            "kind": "item",
            "id": id,
            "name": name,
            "sku": sku,
            "default_unit_name": "Piece",
            "reason": "Register item",
            "product_profile": {
                "sale_price": sale_price_rial,
                "purchase_price": purchase_price_rial,
                "barcode": barcode,
                "category": category,
                "brand": brand,
                "notes": notes,
                "minimum_stock": min_stock,
                "preferred_supplier_id": preferred_supplier_id
            }
        });
        if !unit_id.is_empty() {
            payload["unit_id"] = json!(unit_id);
        }
        self.run("resource", payload, Some("Product saved"))?;
        Ok(id)
    }

    pub fn create_named(&mut self, kind: ResourceKind, name: &str) -> Result<String, String> {
        match kind {
            ResourceKind::Unit | ResourceKind::Warehouse => {
                let id = self.uid();
                self.run(
                    "resource",
                    json!({
                        "kind": kind.as_str(),
                        "id": id,
                        "name": name,
                        "reason": "Register resource"
                    }),
                    Some("Saved"),
                )?;
                Ok(id)
            }
            ResourceKind::Customer | ResourceKind::Vendor => self.create_party(kind, name, "", "", ""),
            ResourceKind::Item => self.create_item(name, "", "", "0", None, "", "", "", "", "", "0", false),
        }
    }

    pub fn parse_money_input(&self, raw: &str) -> i64 {
        let latin = crate::i18n::latin_digits(raw).replace(',', "");
        let amt: i64 = latin.trim().parse().unwrap_or(0);
        if self.fmt.unit == crate::i18n::Unit::Toman { amt * 10 } else { amt }
    }

    pub fn ensure_walk_in_customer(&mut self) -> Result<String, String> {
        let label = self.fmt.t("Walk-in customer");
        if let Some(id) = self
            .books
            .workspace["resources"]["parties"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|p| p["customer"].as_bool().unwrap_or(false) && data::s(&p["name"]) == label)
            .map(|p| data::s(&p["id"]))
        {
            return Ok(id);
        }
        self.create_party(ResourceKind::Customer, &label, "", "", "")
    }

    pub fn toast_err(&mut self, e: String) {
        self.toast(Tone::Danger, e);
    }
}
