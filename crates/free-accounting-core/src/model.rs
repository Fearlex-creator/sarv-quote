use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub profile: String,
    pub phone: String,
    pub address: String,
    pub tax_id: String,
    pub logo: String,
    pub fiscal_month: u32,
    pub currency: String,
    pub calendar: String,
    pub language: String,
    pub direction: String,
    pub digits: String,
    pub date_format: String,
    pub theme: String,
    pub accent: String,
    pub mode: String,
    pub tax_format: String,
    pub closed_through: String,
    pub widgets: Vec<String>,
    pub hidden_widgets: Vec<String>,
    pub last_account: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            profile: "My workspace".into(),
            phone: String::new(),
            address: String::new(),
            tax_id: String::new(),
            logo: String::new(),
            fiscal_month: 1,
            currency: "USD".into(),
            calendar: "gregorian".into(),
            language: "en".into(),
            direction: "ltr".into(),
            digits: "latin".into(),
            date_format: "medium".into(),
            theme: "light".into(),
            accent: "#17665b".into(),
            mode: "personal".into(),
            tax_format: "exclusive".into(),
            closed_through: String::new(),
            widgets: vec![
                "trend".into(),
                "balances".into(),
                "recent".into(),
                "upcoming".into(),
            ],
            hidden_widgets: vec![],
            last_account: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Split {
    pub category_id: String,
    pub amount: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Receipt {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TransactionInput {
    pub date: String,
    pub kind: String,
    pub amount: String,
    pub paid_amount: String,
    pub account_id: String,
    pub to_account_id: String,
    pub payee: String,
    pub contact_id: String,
    pub category_id: String,
    pub memo: String,
    pub tags: String,
    pub due_date: String,
    pub splits: Vec<Split>,
    pub receipt: Option<Receipt>,
    pub correction_id: String,
    pub correction_date: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountInput {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub currency: String,
    pub group: String,
    pub opening_balance: String,
    pub archived: bool,
    pub sort_order: i32,
    pub opening_date: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ContactInput {
    pub id: String,
    pub name: String,
    pub customer: bool,
    pub supplier: bool,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub tax_id: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Filters {
    pub search: String,
    pub from: String,
    pub to: String,
    pub account: String,
    pub category: String,
    pub contact: String,
    pub status: String,
    pub currency: String,
    pub offset: u32,
    pub limit: u32,
}
