use sarv_quote::{
    desktop::{dispatch, seed_fixture},
    store::Store,
};
use serde_json::{Value, json};

fn call(dir: &std::path::Path, action: &str, p: Value) -> Value {
    dispatch(dir, action, p).unwrap()
}

fn fixture() -> (tempfile::TempDir, String, String) {
    let d = tempfile::tempdir().unwrap();
    seed_fixture(d.path()).unwrap();
    let s = Store::open(d.path()).unwrap();
    let c = s.state.accounting.companies[0].id.clone();
    let p = s.state.accounting.periods[0].id.clone();
    (d, c, p)
}

fn ctx(c: &str, p: &str, key: &str) -> Value {
    json!({
        "company_id": c,
        "period_id": p,
        "source_id": key,
        "request_key": key,
        "date": "2026-10-08",
        "reason": "Contact profile test",
        "reference": key,
        "due_date": "2026-10-30"
    })
}

#[test]
fn party_profile_save_and_contact_detail_round_trip_every_field() {
    let (d, c, _) = fixture();
    let profile = json!({
        "company_id": c,
        "display_name": "Acme Trading Co.",
        "first_name": "Reza",
        "last_name": "Ahmadi",
        "organization": "Acme Group",
        "phone": "02188776655",
        "mobile": "09121112233",
        "email": "reza@acme.example",
        "address": "No. 42 Valiasr St",
        "city": "Tehran",
        "postal_code": "14155-1234",
        "national_id": "0011223344",
        "tax_id": "10101010101",
        "website": "https://acme.example",
        "notes": "VIP customer"
    });

    call(
        d.path(),
        "party_profile",
        json!({
            "company_id": c,
            "party_id": "CU-A",
            "profile": profile
        }),
    );

    let detail = call(
        d.path(),
        "contact_detail",
        json!({
            "company_id": c,
            "party_id": "CU-A",
            "as_of": "2026-10-08"
        }),
    );
    assert_eq!(detail["profile"], profile);
}

#[test]
fn editing_profile_preserves_posted_document_snapshot() {
    let (d, c, p) = fixture();
    let s = Store::open(d.path()).unwrap();
    let setup = s.state.accounting.core.setups[0].id.clone();

    // Set initial profile before posting the sale.
    call(
        d.path(),
        "party_profile",
        json!({
            "company_id": c,
            "party_id": "CU-A",
            "profile": {
                "company_id": c,
                "display_name": "Original Name",
                "address": "Original Address",
                "phone": "02111111111"
            }
        }),
    );

    let draft = json!({
        "id": "snap-draft",
        "revision": "0",
        "company_id": c,
        "period_id": p,
        "setup_id": setup,
        "customer_id": "CU-A",
        "reference": "snap-sales",
        "document_date": "2026-10-08",
        "financial_date": "2026-10-08",
        "date_display": "Iso",
        "display_unit": "Rial",
        "document_discount": "0",
        "supplied_total": "0",
        "reason": "Snapshot test sale",
        "quotation": null,
        "lines": [{
            "id": "snap-sale-line",
            "item_id": "ITEM-A",
            "warehouse_id": "W1",
            "unit_id": "PIECE",
            "quantity": "1",
            "unit_price": "20",
            "line_discount": "0",
            "price_reference": "Manual",
            "price_date": "2026-10-08",
            "override_reason": ""
        }]
    });
    let preview = call(d.path(), "draft_preview", json!({ "draft": draft }));
    let mut draft = draft;
    draft["supplied_total"] = preview["supplied_total"].clone();
    call(d.path(), "draft_save", json!({ "draft": draft, "expected_revision": null }));
    let order = call(
        d.path(),
        "draft_post",
        json!({ "id": "snap-draft", "revision": "1", "request_key": "snap-post" }),
    );
    let mut fulfill = ctx(&c, &p, "snap-invoice");
    fulfill["order_id"] = order["record_id"].clone();
    let invoice = call(d.path(), "fulfill_post", fulfill);
    let invoice_id = invoice["record_id"].as_str().unwrap().to_owned();

    let before_detail = call(d.path(), "detail", json!({ "company_id": c, "id": invoice_id }));

    // Edit the customer's profile after the sale is posted.
    call(
        d.path(),
        "party_profile",
        json!({
            "company_id": c,
            "party_id": "CU-A",
            "profile": {
                "company_id": c,
                "display_name": "Updated Name",
                "address": "Updated Address",
                "phone": "02199999999"
            }
        }),
    );

    let after_detail = call(d.path(), "detail", json!({ "company_id": c, "id": invoice_id }));
    assert_eq!(
        after_detail["data"], before_detail["data"],
        "posted document data snapshot must not change when profile is edited"
    );
    assert_eq!(
        after_detail["command"], before_detail["command"],
        "posted document command snapshot must not change when profile is edited"
    );
}
