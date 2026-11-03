import assert from "node:assert/strict";
import { test } from "node:test";
import fixtures from "../../../crates/sarv-quote/tests/fixtures/iran_ids.json" with { type: "json" };
import {
  classifyPartyId,
  legalId,
  nationalId,
  postalCode,
  sayadId,
  sheba,
  type IdResult,
} from "./iran-ids.ts";

type Case = { value: string; reason: string };
const kinds: Array<[string, (s: string) => IdResult]> = [
  ["national_id", nationalId],
  ["legal_id", legalId],
];

for (const [kind, check] of kinds) {
  test(`${kind}: shared fixtures`, () => {
    const f = fixtures[kind as "national_id" | "legal_id"];
    for (const v of f.valid)
      assert.deepEqual(check(v), { ok: true }, `${kind} ${v}`);
    for (const c of f.invalid as Case[])
      assert.deepEqual(
        check(c.value),
        { ok: false, reason: c.reason },
        `${kind} ${c.value}`,
      );
  });
}

test("postal, sayad and sheba: shared fixtures", () => {
  for (const [kind, check] of [
    ["postal_code", postalCode],
    ["sayad_id", sayadId],
  ] as const) {
    for (const v of fixtures[kind].valid)
      assert.equal(check(v).ok, true, `${kind} ${v}`);
    for (const v of fixtures[kind].invalid)
      assert.equal(check(v).ok, false, `${kind} ${v}`);
  }
  for (const v of fixtures.sheba.valid) assert.equal(sheba(v).ok, true, v);
  for (const v of fixtures.sheba.invalid) assert.equal(sheba(v).ok, false, v);
});

test("classifyPartyId hints without forcing a type", () => {
  assert.equal(classifyPartyId("0084575948"), "national");
  assert.equal(classifyPartyId("۱۴۰۰۳۷۷۸۹۹۰"), "legal");
  assert.equal(classifyPartyId("ID-1"), "other");
  assert.equal(classifyPartyId("   "), "empty");
});
