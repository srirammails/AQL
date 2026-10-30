# AQL v0.9 — Additions (draft): grouping and spec completion

> **Date:** 30 Oct 2026 · @Sriram · **Status:** draft; §1 implemented in the grammar,
> parser and ADB; conformance suite `24_group_by.yaml`. ClawDB and the WASM playground
> reject `GROUP BY`.
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md) and the v0.6–v0.8 additions.
> Every older statement parses as before, and its AST serializes to the same JSON.

v0.9 closes the questions v0.5 §22 left open that agents need, and the gaps between
the v0.5 text and the grammar.

## 1. `GROUP BY` (v0.5 §22 question 1)

```bnf
modifier     ::= … | group_by_mod
group_by_mod ::= "GROUP" "BY" field ("," field)*
```

```sql
RECALL FROM EPISODIC WHERE campaign = "summer"
  GROUP BY url
  AGGREGATE COUNT(*) AS impressions, AVG(ctr) AS ctr
  HAVING impressions > 10
  ORDER BY impressions DESC LIMIT 20
```

- `GROUP BY` needs `AGGREGATE` (as `HAVING` does, §19); either may come first.
- The result is **one row per distinct value** of the group fields, holding the group
  fields and the aggregates: `[{url: "a", impressions: 40, ctr: 0.02}, …]`. A record
  without a group field belongs to the group where it is `null`.
- Fields are payload fields or dotted paths, including `metadata.*`, as in `WHERE`. A
  dotted group field is nested in the row (`metadata.scope` → `{metadata: {scope: …}}`).
- With `GROUP BY`, `HAVING` keeps the groups it matches, then `ORDER BY` (a group field
  or an aggregate alias), `LIMIT` and `RETURN` apply to the **groups**, not the records.
  Without an `ORDER BY`, groups come in the order of their first record in the read's
  default order (EPISODIC newest first, the others oldest first).
- No group passes: an empty list. Without `GROUP BY` an aggregate is one object, as
  before.
- `GROUP BY` applies to `RECALL`, the statement `AGGREGATE` applies to.

## Validation (additions to v0.5 §19)

```
GROUP BY requires AGGREGATE:
  RECALL FROM EPISODIC WHERE a = 1 GROUP BY url → error: "GROUP BY requires AGGREGATE"
HAVING requires AGGREGATE (v0.5 §19, now enforced by the parser):
  RECALL FROM EPISODIC WHERE a = 1 HAVING n > 1 → error: "HAVING requires AGGREGATE"
```
