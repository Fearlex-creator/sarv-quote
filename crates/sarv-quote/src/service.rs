//! Headless application service for Tauri (and other non-GTK hosts).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::domain::{Draft, Settings, State};
use crate::store::Store;
use crate::cashbook::{CashEntry, CashEntryInput, Wallet, WalletInput};
use crate::views::{
    account_rows, all_ledger_rows, contact_rows, dashboard, settings_view, suggestions,
    upsert_contact, wallet_account_rows, AccountRow, ContactInput, ContactRow, DashboardView,
    LedgerRow, SettingsView, SuggestView,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub generation: u64,
    pub business_name: String,
    pub catalog_count: usize,
    pub customer_count: usize,
    pub quote_count: usize,
    pub draft: Draft,
    pub settings: Settings,
}

pub struct AppService {
    store: Mutex<Store>,
}

impl AppService {
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Self> {
        let store = Store::open(data_dir.as_ref()).context("باز کردن پایگاه داده")?;
        Ok(Self {
            store: Mutex::new(store),
        })
    }

    pub fn open_default() -> Result<Self> {
        let data = std::env::var_os("SARV_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"));
        Self::open(data)
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let store = self.store.lock().expect("store lock");
        Ok(snapshot_from_state(
            &store.state,
            store.local_generation() as u64,
        ))
    }

    pub fn dashboard(&self) -> Result<DashboardView> {
        let store = self.store.lock().expect("store lock");
        Ok(dashboard(&store.state))
    }

    pub fn list_ledger(&self) -> Result<Vec<LedgerRow>> {
        let store = self.store.lock().expect("store lock");
        Ok(all_ledger_rows(&store.state))
    }

    pub fn list_accounts(&self) -> Result<Vec<AccountRow>> {
        let mut store = self.store.lock().expect("store lock");
        store.change(|state| {
            state.cashbook.ensure_default_wallets();
            Ok(())
        })?;
        let mut wallets = wallet_account_rows(&store.state);
        let formal = account_rows(&store.state.accounting);
        if !formal.is_empty() {
            wallets.extend(formal);
        }
        Ok(wallets)
    }

    pub fn save_wallet(&self, wallet: WalletInput) -> Result<Wallet> {
        let mut store = self.store.lock().expect("store lock");
        store.change(|state| state.cashbook.upsert_wallet(wallet))
    }

    pub fn suggestions(&self) -> Result<SuggestView> {
        let store = self.store.lock().expect("store lock");
        Ok(suggestions(&store.state))
    }

    pub fn add_cash_entry(&self, input: CashEntryInput) -> Result<CashEntry> {
        let mut store = self.store.lock().expect("store lock");
        store.change(|state| state.cashbook.add(input))
    }

    pub fn list_contacts(&self) -> Result<Vec<ContactRow>> {
        let store = self.store.lock().expect("store lock");
        Ok(contact_rows(&store.state))
    }

    pub fn save_contact(&self, contact: ContactInput) -> Result<ContactRow> {
        let mut store = self.store.lock().expect("store lock");
        let saved = store.change(|state| upsert_contact(state, contact))?;
        let rows = contact_rows(&store.state);
        rows.into_iter()
            .find(|c| c.id == saved.id)
            .ok_or_else(|| anyhow::anyhow!("مخاطب ذخیره نشد"))
    }

    pub fn get_settings_view(&self) -> Result<SettingsView> {
        let store = self.store.lock().expect("store lock");
        Ok(settings_view(&store.state.settings))
    }

    pub fn get_settings(&self) -> Result<Settings> {
        let store = self.store.lock().expect("store lock");
        Ok(store.state.settings.clone())
    }

    pub fn update_settings(&self, settings: Settings) -> Result<Settings> {
        settings.validate()?;
        let mut store = self.store.lock().expect("store lock");
        store.change(|state| {
            state.settings = settings.clone();
            Ok(())
        })?;
        Ok(settings)
    }

    pub fn get_draft(&self) -> Result<Draft> {
        let store = self.store.lock().expect("store lock");
        Ok(store.state.draft.clone())
    }

    pub fn save_draft(&self, draft: Draft) -> Result<Draft> {
        let mut store = self.store.lock().expect("store lock");
        store.change(|state| {
            state.draft = draft.clone();
            Ok(())
        })?;
        Ok(draft)
    }

    pub fn with_store<R>(&self, f: impl FnOnce(&mut Store) -> Result<R>) -> Result<R> {
        let mut store = self.store.lock().expect("store lock");
        f(&mut store)
    }
}

fn snapshot_from_state(state: &State, generation: u64) -> Snapshot {
    Snapshot {
        generation,
        business_name: state.settings.business_name.clone(),
        catalog_count: state.catalogs.len(),
        customer_count: state.customers.len(),
        quote_count: state.quotes.len(),
        draft: state.draft.clone(),
        settings: state.settings.clone(),
    }
}
