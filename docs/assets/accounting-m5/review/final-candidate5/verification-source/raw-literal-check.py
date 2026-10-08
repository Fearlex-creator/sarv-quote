"""Extract exact integers from actual persisted output, without importing Sarv formulas."""
import json
import sys
from pathlib import Path

state = json.loads(Path(sys.argv[1]).read_text())
out = Path(sys.argv[2])
expected = json.loads(Path(__file__).with_name("literal-expected.json").read_text())
accounting = state["accounting"]
records = accounting["sales"]["records"]
by_source = {next(iter(r["command"].values()))["context"]["source_id"]: r for r in records}
assert len(records) == 5
assert len(accounting["journals"]) == 11
assert len(accounting["procurement"]["records"]) == 10
assert len(accounting["procurement"]["requests"]) == 10
assert len(accounting["sales"]["requests"]) == 5

order = by_source["sales1499"]["data"]["Order"]
for i, line in enumerate(order["lines"]):
    for key in ("gross", "document_discount", "net"):
        source = {"gross":"gross_irr", "document_discount":"allocated_document_discount_irr", "net":"net_irr"}[key]
        assert line[key] == expected["sales_order"][source][i]
    assert line["input"]["line_discount"] == expected["sales_order"]["line_discount_irr"][i]

for source, key in [("fulfill633", "first_fulfillment"), ("fulfill866", "second_fulfillment")]:
    data = by_source[source]["data"]["Fulfill"]
    assert data["delivery_id"] != data["invoice_id"]
    assert data["delivery_state"] == "Delivered" and data["invoice_state"] == "Posted"
    assert data["total"] == expected[key]["total_net_irr"]
    assert data["cost"] == expected[key]["total_cost_irr"]
    for i, line in enumerate(data["lines"]):
        for field, original in [("net","net_irr"),("cost","issue_cost_irr"),("gross","derived_gross_irr"),("line_discount","allocated_line_discount_irr"),("document_discount","allocated_document_discount_irr")]:
            assert line[field] == expected[key][original][i], (source, field, line)
        assert line["gross"] == line["net"] + line["line_discount"] + line["document_discount"]

actual_returns = []
for source in ("return235", "return398"):
    data = by_source[source]["data"]["Return"]
    assert data["return_id"] != data["credit_id"]
    assert data["return_state"] == "Restocked" and data["credit_state"] == "Posted"
    for line in data["lines"]:
        actual_returns.append((line["quantity_thousandths"],line["net"],line["cost"],line["line_discount"],line["document_discount"],line["gross"]))
assert actual_returns == [(1000,235,12,1,16,252),(1000,234,11,1,16,251),(1000,164,11,1,11,176)]

operational = accounting["procurement"]["records"] + records
sequences = sorted(r["sequence"] for r in operational)
assert sequences == list(range(2,17)), sequences
assert len({r["id"] for r in operational}) == 15
assert sum(len(r["movements"]) for r in operational) == 14
for r in operational:
    if r["journal_id"] is not None:
        assert r["journal_id"] != r["id"]
report = {"phase":"raw persisted integer extraction", "literal_order_components":True, "literal_two_fulfillment_components":True,"literal_three_return_components":True,"distinct_document_ids_states":True,"original_cost_not_current_average":True,"fifteen_operational_sequences_2_to_16":True,"journal_count":11,"movement_count":14,"does_not_import_production_calculation":True}
out.write_text(json.dumps(report,indent=2)+"\n")
print("PASS raw persisted exact components: order1499, invoices633/866, returns235/234/164 costs12/11/11, distinct documents, 15 global sequences and11 journals.")
