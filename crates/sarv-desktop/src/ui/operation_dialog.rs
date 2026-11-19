//! Reusable operation dialog with editable line quantities (`receive`, `bill`, `fulfill`, `sale_return`, `purchase_return`).

use rune::{El, col, row};
use serde_json::{Value, json};

use crate::app::{State, Tone};
use crate::ui::confirm::Confirm;
use crate::ui::glass::{Btn, DIALOG_MD, Ui};
use crate::ui::icons;
use crate::ui::ops::{item_name, qty_from_thousandths};

#[derive(Clone, Debug)]
pub struct OpLine {
    pub id: Value,
    pub name: String,
    pub remaining_thousandths: String,
}

#[derive(Default)]
pub struct OperationState {
    pub open: bool,
    pub record_id: String,
    pub operation: String,
    pub title: String,
    pub reference: String,
    pub reason: String,
    pub key: String,
    pub lines: Vec<OpLine>,
    pub posting: bool,
}

/// Parse a display-unit decimal quantity string (with optional Persian/Arabic digits) into integer thousandths.
/// Rejects negative numbers, non-numeric input, and values with more than 3 fractional digits.
pub fn parse_thousandths(raw: &str) -> Result<i64, String> {
    let norm = crate::i18n::latin_digits(raw.trim()).replace([',', ' '], "");
    if norm.is_empty() {
        return Ok(0);
    }
    if norm.starts_with('-') || norm.starts_with('+') {
        return Err("Enter a valid quantity.".into());
    }
    let mut parts = norm.split('.');
    let whole_str = parts.next().unwrap_or("");
    let frac_str = parts.next();
    if parts.next().is_some() {
        return Err("Enter a valid quantity.".into());
    }
    if whole_str.is_empty() && frac_str.is_none_or(str::is_empty) {
        return Err("Enter a valid quantity.".into());
    }
    if !whole_str.chars().all(|c| c.is_ascii_digit()) || !frac_str.is_none_or(|f| f.chars().all(|c| c.is_ascii_digit())) {
        return Err("Enter a valid quantity.".into());
    }
    let whole: i64 = if whole_str.is_empty() {
        0
    } else {
        whole_str.parse().map_err(|_| "Enter a valid quantity.".to_string())?
    };
    let frac_val: i64 = match frac_str {
        Some(frac) if frac.len() > 3 => return Err("Enter a valid quantity.".into()),
        Some(frac) if !frac.is_empty() => format!("{frac:0<3}").parse().map_err(|_| "Enter a valid quantity.".to_string())?,
        _ => 0,
    };
    whole
        .checked_mul(1000)
        .and_then(|w| w.checked_add(frac_val))
        .ok_or_else(|| "Enter a valid quantity.".to_string())
}

/// Open the operation dialog for `record_id` and `operation` if any line has remaining quantity.
pub fn open(st: &mut State, record_id: &str, operation: &str, title_key: &str, reference: String, reason_key: &str) {
    let opts = match st.query("operation_options", json!({ "record_id": record_id, "operation": operation })) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e);
            return;
        }
    };
    let mut lines = Vec::new();
    for l in opts["lines"].as_array().into_iter().flatten() {
        let rem = crate::data::s(&l["remaining_quantity"]);
        let rem_num: i64 = rem.parse().unwrap_or(0);
        if rem_num > 0 {
            lines.push(OpLine {
                id: l["id"].clone(),
                name: item_name(st, &crate::data::s(&l["item_id"])),
                remaining_thousandths: rem,
            });
        }
    }
    if lines.is_empty() {
        st.toast(Tone::Info, st.fmt.t("Nothing left to process on this document."));
        return;
    }

    let key = st.mint_key(operation);
    st.forms.clear_prefix("op-");
    for (i, line) in lines.iter().enumerate() {
        st.forms
            .set(&format!("op-qty-{i}"), &qty_from_thousandths(&line.remaining_thousandths));
    }
    st.operation = OperationState {
        open: true,
        record_id: record_id.to_string(),
        operation: operation.to_string(),
        title: st.fmt.t(title_key),
        reference,
        reason: st.fmt.t(reason_key),
        key,
        lines,
        posting: false,
    };
}

fn close(st: &mut State) {
    st.operation.open = false;
}

fn review(st: &mut State) {
    if !st.operation.open || st.operation.posting {
        return;
    }
    st.operation.posting = true;
    let mut req_lines = Vec::new();
    let mut confirm_lines = Vec::new();
    for (i, line) in st.operation.lines.iter().enumerate() {
        let raw = st.forms.get(&format!("op-qty-{i}"));
        let parsed = match parse_thousandths(&raw) {
            Ok(v) => v,
            Err(msg) => {
                st.toast(Tone::Danger, st.fmt.t(&msg));
                st.operation.posting = false;
                return;
            }
        };
        let rem: i64 = line.remaining_thousandths.parse().unwrap_or(0);
        if parsed > rem {
            st.toast(Tone::Danger, st.fmt.t("Quantity exceeds remaining."));
            st.operation.posting = false;
            return;
        }
        if parsed == 0 {
            continue;
        }
        let parsed_str = parsed.to_string();
        req_lines.push(json!({
            "id": line.id,
            "quantity": qty_from_thousandths(&parsed_str),
        }));
        confirm_lines.push((line.name.clone(), st.fmt.qty(&parsed_str)));
    }
    if req_lines.is_empty() {
        st.toast(Tone::Danger, st.fmt.t("Enter at least one quantity."));
        st.operation.posting = false;
        return;
    }
    let Ok(date) = st.form_date("op-date") else {
        st.operation.posting = false;
        return;
    };

    let op = st.operation.operation.clone();
    let mut payload = json!({
        "record_id": st.operation.record_id,
        "order_id": st.operation.record_id,
        "receipt_id": st.operation.record_id,
        "source_id": st.operation.key,
        "request_key": st.operation.key,
        "date": date,
        "due_date": date,
        "reference": st.operation.reference,
        "reason": st.operation.reason,
        "lines": req_lines,
    });

    let review_resp = match st.query(&format!("{op}_preview"), payload.clone()) {
        Ok(v) => v,
        Err(e) => {
            st.toast_err(e);
            st.operation.posting = false;
            return;
        }
    };
    payload["expected_generation"] = review_resp["_generation"].clone();
    let total = st.fmt.money(&crate::data::s(&review_resp["total"]));
    confirm_lines.push((st.fmt.t("Total"), total));

    let title = st.operation.title.clone();
    let danger = op == "sale_return" || op == "purchase_return";
    let verb_key = if danger { "Post return" } else { "Post" };
    st.operation.posting = false;
    st.confirm = Some(Confirm::one(
        title,
        String::new(),
        confirm_lines,
        verb_key,
        st.fmt.t(verb_key),
        danger,
        move |st| {
            if st.operation.posting || !st.operation.open {
                return;
            }
            st.operation.posting = true;
            if st.run(&format!("{op}_post"), payload.clone(), Some("Posted")).is_ok() {
                st.operation.open = false;
                st.operation.key.clear();
            }
            st.operation.posting = false;
        },
    ));
}

pub fn mount(s: &State, ui: &Ui, layer: &mut Vec<El>) {
    if !s.operation.open {
        return;
    }
    let header = row()
        .w_full()
        .px(4.0)
        .gap(8.0)
        .child(ui.muted(ui.t("Product")).grow(1.0))
        .child(ui.muted(ui.t("Remaining")).w(90.0))
        .child(ui.muted(ui.t("Qty")).w(110.0));

    let mut lines_col = col().key("op-lines").w_full().gap(8.0).child(header);
    for (i, line) in s.operation.lines.iter().enumerate() {
        lines_col = lines_col.child(
            row()
                .key(format!("op-line-{i}"))
                .w_full()
                .px(4.0)
                .gap(8.0)
                .center_y()
                .child(ui.body(line.name.clone()).grow(1.0).min_w(0.0))
                .child(ui.muted(s.fmt.qty(&line.remaining_thousandths)).w(90.0))
                .child(ui.field(s, &format!("op-qty-{i}"), "0").w(110.0)),
        );
    }

    let form = col()
        .gap(16.0)
        .child(ui.section("op-lines-sec", &ui.t("Products"), lines_col))
        .child(ui.date_field(s, "op-date", "Date"));

    ui.form_dialog(
        layer,
        &mut Vec::new(),
        "op-dialog",
        &s.operation.title,
        "",
        true,
        DIALOG_MD,
        form,
        vec![
            ui.btn(Btn::Ghost, "Cancel", None, close),
            ui.btn(Btn::Primary, "Review", Some(&icons::CHECK), review),
        ],
        close,
    );
}

#[cfg(test)]
mod tests {
    use super::parse_thousandths;

    #[test]
    fn parse_thousandths_handles_decimals_persian_digits_and_rejects_invalid() {
        assert_eq!(parse_thousandths("2.5"), Ok(2500));
        assert_eq!(parse_thousandths("۲٫۵"), Ok(2500));
        assert_eq!(parse_thousandths("0"), Ok(0));
        assert_eq!(parse_thousandths("3"), Ok(3000));
        assert_eq!(parse_thousandths("1.25"), Ok(1250));
        assert_eq!(parse_thousandths("0.005"), Ok(5));
        assert!(parse_thousandths("0.0005").is_err());
        assert!(parse_thousandths("-1").is_err());
        assert!(parse_thousandths("abc").is_err());
        assert!(parse_thousandths("1.2.3").is_err());
    }
}
