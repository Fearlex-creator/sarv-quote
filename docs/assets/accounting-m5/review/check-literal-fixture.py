"""Check independently written literals for internal conservation, without importing Sarv."""
import json
from pathlib import Path

fixture = json.loads(Path(__file__).with_name("literal-expected.json").read_text())
for row in fixture["cutoffs"]:
    assert row["w1"][1] + row["w2"][1] == row["inventory_irr"], row
    assert row["ar_irr"] == 200 + row["sales_irr"] - row["sales_returns_irr"], row
    assert row["pnl_irr"] == row["sales_irr"] - row["sales_returns_irr"] - row["net_cogs_irr"], row
    debit = 251 + 300 + row["ar_irr"] + row["inventory_irr"] + 200 + row["sales_returns_irr"] + row["net_cogs_irr"]
    credit = row["ap_irr"] + 100 + row["grni_irr"] + 952 + row["sales_irr"]
    assert debit == credit == row["tb_irr"], row
order = fixture["sales_order"]
first, second = fixture["first_fulfillment"], fixture["second_fulfillment"]
for i in range(2):
    assert order["gross_irr"][i] - order["line_discount_irr"][i] - order["allocated_document_discount_irr"][i] == order["net_irr"][i]
    for component in ("net_irr", "allocated_line_discount_irr", "allocated_document_discount_irr", "derived_gross_irr"):
        original = {"net_irr": "net_irr", "allocated_line_discount_irr": "line_discount_irr", "allocated_document_discount_irr": "allocated_document_discount_irr", "derived_gross_irr": "gross_irr"}[component]
        assert first[component][i] + second[component][i] == order[original][i]
returns = fixture["first_invoice_returns"]
assert sum(r["net_irr"] for r in returns) == 633
assert sum(r["original_issue_cost_irr"] for r in returns) == 34
print("PASS independent literal self-consistency: 8 cutoffs, exact double entry/AR/P&L, fulfillment components and original-return conservation. No M5 code executed.")
