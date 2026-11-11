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

/// Digits the user typed, in the active display unit. Do not convert toman to rial here.
pub fn typed_display_amount(raw: &str) -> String {
    let digits: String = crate::i18n::latin_digits(raw).chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { "0".into() } else { digits }
}

/// Preview then post `fulfill`, `receive`, or `bill`. Empty remaining quantity is an error.
pub fn post_operation(
    st: &mut State,
    record_id: &str,
    operation: &str,
    reference: &str,
    reason: &str,
    request_key: &str,
) -> Result<Value, String> {
    let opts = st.query(
        "operation_options",
        json!({ "record_id": record_id, "operation": operation }),
    )?;
    let lines: Vec<Value> = opts["lines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let rem = crate::data::s(&l["remaining_quantity"]);
            if rem == "0" || rem.is_empty() {
                None
            } else {
                Some(json!({ "id": l["id"], "quantity": qty_from_thousandths(&rem) }))
            }
        })
        .collect();
    if lines.is_empty() {
        return Err(st.fmt.t("No quantities remaining"));
    }
    let date = crate::data::today();
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
    let review = st.query(&format!("{operation}_preview"), payload.clone())?;
    payload["expected_generation"] = review["_generation"].clone();
    st.run(&format!("{operation}_post"), payload, None)
}
