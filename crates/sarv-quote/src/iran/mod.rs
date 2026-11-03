//! Iran-specific domain rules: identifiers, Moadian Tax IDs, effective-dated law
//! data and the cheque lifecycle. Every rule the spec marks "verify" carries a
//! `verify` flag so the UI can show it for review instead of hiding it in code.
pub mod cheque;
pub mod ids;
pub mod law;
pub mod sheba;
pub mod tax_id;
