#!/usr/bin/env python3
"""Independent fixture self-consistency only. Does not import or test Sarv."""
import json
from pathlib import Path
p = Path(__file__).with_name("literal-expected.json")
f = json.loads(p.read_text())
c = f["constants"]
rows = f["billed_branch"] + f["unbilled_interleaved_branch"]
for row in rows:
    debit = c["cash"] + c["bank"] + c["ar"] + row["inventory"] + c["cr"] + c["sales_returns"] + c["cogs"]
    credit = row["ap"] + c["cp"] + row["grni"] + c["capital"] + c["sales"]
    assert debit == credit == row["tb"], (row["event"], debit, credit)
    assert row["w1_v"] + c["w2_v"] == row["inventory"]
assert c["sales"] - c["sales_returns"] - c["cogs"] == c["earnings"]
b = f["billed_branch"]
assert [r["ap"] - r["invoice150_remaining"] for r in b] == [179, 179, 179]
assert [b[i - 1]["inventory"] - b[i]["inventory"] for i in (1, 2)] == [45, 60]
assert [b[i - 1]["w1_q"] - b[i]["w1_q"] for i in (1, 2)] == [3, 4]
u = f["unbilled_interleaved_branch"]
assert {k: u[0][k] for k in ("w1_q", "w1_v", "inventory", "ap", "grni", "tb")} == {k: u[-1][k] for k in ("w1_q", "w1_v", "inventory", "ap", "grni", "tb")}
assert [r["receipt_unbilled_q"] for r in u] == [0, 6, 4, 3, 0, 0, 0, 0]
assert [r["receipt_unbilled_v"] for r in u] == [0, 90, 60, 45, 0, 0, 0, 0]
assert 15 + 30 + 45 == 90
assert 15 + 15 + 45 == 75
for name in ("tiny_receipt_consumption", "tiny_billed_returns"):
    tiny = f[name]
    assert sum(e["q"] for e in tiny["events"]) == tiny["original_q_thousandths"]
    assert sum(e["v"] for e in tiny["events"]) == tiny["original_value"]
    assert [e["v"] for e in tiny["events"]] == [0, 1, 0]
print(json.dumps({"fixture_self_consistency": "PASS", "rows": len(rows), "product_behavior_tested": False}, indent=2))
