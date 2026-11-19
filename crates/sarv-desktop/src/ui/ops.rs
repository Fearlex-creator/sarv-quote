//! Follow-on document operations. Quantities sent to the engine are display units;
//! `remaining_quantity` on an operation line is already in thousandths.

use serde_json::{Value, json};

use crate::app::State;

/// `remaining_quantity` is thousandths. `quantity()` multiplies a display string by 1000.
pub fn qty_from_thousandths(raw: &str) -> String {
    let digits = crate::i18n::latin_digits(raw);
    let q: i64 = digits.parse().unwrap_or(0);
    let neg = q < 0;
    let q = q.unsigned_abs();
    let whole = q / 1000;
    let frac = q % 1000;
    let body = if frac == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{frac:03}").trim_end_matches('0').to_string()
    };
    if neg { format!("-{body}") } else { body }
}

/// Validated display-unit amount normalized to Latin digits without grouping.
/// Preserves a single decimal separator (e.g. `"۱۲۳۴٫۵"` -> `"1234.5"`). Do not convert toman to rial here.
pub fn typed_display_amount(raw: &str, unit: crate::i18n::Unit) -> Result<String, crate::i18n::MoneyError> {
    let norm = crate::i18n::latin_digits(raw.trim()).replace([',', ' '], "");
    if norm.is_empty() {
        return Ok("0".into());
    }
    crate::i18n::parse_money(raw, unit)?;
    Ok(norm)
}

pub struct OperationReview {
    pub payload: Value,
    pub lines: Vec<(String, String)>,
    pub total: String,
}

pub(crate) fn item_name(st: &State, item_id: &str) -> String {
    st.books.workspace["resources"]["items"]
        .as_array()
        .and_then(|arr| arr.iter().find(|it| crate::data::s(&it["id"]) == item_id))
        .map(|it| crate::data::s(&it["name"]))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| item_id.to_string())
}

/// Preview `fulfill`, `receive`, `bill`, `sale_return`, or `purchase_return`.
/// Empty remaining quantity is an error. Every `Err` returned has already been toasted.
pub fn review_operation(
    st: &mut State,
    record_id: &str,
    operation: &str,
    reference: &str,
    reason: &str,
    request_key: &str,
    date: &str,
) -> Result<OperationReview, String> {
    let opts = match st.query("operation_options", json!({ "record_id": record_id, "operation": operation })) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e.clone());
            return Err(e);
        }
    };
    let mut lines = Vec::new();
    let mut display_lines = Vec::new();
    for l in opts["lines"].as_array().into_iter().flatten() {
        let rem = crate::data::s(&l["remaining_quantity"]);
        let rem_th: i64 = rem.parse().unwrap_or(0);
        if rem_th > 0 {
            lines.push(json!({ "id": l["id"], "quantity": qty_from_thousandths(&rem) }));
            display_lines.push((item_name(st, &crate::data::s(&l["item_id"])), st.fmt.qty(&rem)));
        }
    }
    if lines.is_empty() {
        let msg = st.fmt.t("No quantities remaining");
        st.toast_err(msg.clone());
        return Err(msg);
    }
    let mut payload = json!({
        "record_id": record_id,
        "order_id": record_id,
        "receipt_id": record_id,
        "source_id": request_key,
        "request_key": request_key,
        "date": date,
        "due_date": date,
        "reference": reference,
        "reason": reason,
        "lines": lines
    });
    let review = match st.query(&format!("{operation}_preview"), payload.clone()) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e.clone());
            return Err(e);
        }
    };
    payload["expected_generation"] = review["_generation"].clone();
    let total = st.fmt.money(&crate::data::s(&review["total"]));
    Ok(OperationReview {
        payload,
        lines: display_lines,
        total,
    })
}

/// Post a previously reviewed operation payload.
pub fn post_reviewed(st: &mut State, operation: &str, payload: Value) -> Result<Value, String> {
    st.run(&format!("{operation}_post"), payload, None)
}

/// Preview then post `fulfill`, `receive`, or `bill`. Empty remaining quantity is an error.
/// Every `Err` returned has already been toasted (from `query`, validation, or `run`).
pub fn post_operation(
    st: &mut State,
    record_id: &str,
    operation: &str,
    reference: &str,
    reason: &str,
    request_key: &str,
    date: &str,
) -> Result<Value, String> {
    let review = review_operation(st, record_id, operation, reference, reason, request_key, date)?;
    post_reviewed(st, operation, review.payload)
}
