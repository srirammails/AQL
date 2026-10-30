# AQL v0.9 — Additions (draft): grouping and spec completion

> **Date:** 30 Oct 2026 · @Sriram · **Status:** draft; §1–§4 implemented in the grammar,
> parser and ADB; conformance suites `24_group_by.yaml`, `25_window_episodic.yaml`,
> `26_update_null.yaml`.
> ClawDB and the WASM playground reject `GROUP BY` and `WINDOW` on `RECALL`.
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

## 2. `WINDOW` on `RECALL FROM EPISODIC` (v0.5 §22 question 2)

```bnf
recall_stmt ::= "RECALL" "FROM" memory_type predicate modifier*   -- modifier may be window_mod
window_mod  ::= "WINDOW" ( "LAST" (integer | duration)
                         | "TOP" integer "BY" field
                         | "SINCE" condition )
```

```sql
RECALL FROM EPISODIC WHERE pod = "payments-api" WINDOW LAST 7d
RECALL FROM EPISODIC WHERE pod = "payments-api" WINDOW SINCE event_id = "inc-001"
RECALL FROM EPISODIC WHERE pod = "payments-api" WINDOW LAST 50 AGGREGATE COUNT(*) AS n
```

- v0.5 §19 allowed `WINDOW` on `SCAN FROM WORKING` only; v0.9 also allows it on
  `RECALL FROM EPISODIC`. Episodes are events, and "the last 7 days" or "since the
  incident" is how agents read them.
- The window runs over the matching episodes **in commit order** (oldest first), as on
  WORKING: `LAST n` keeps the newest n, `LAST <duration>` those committed within it,
  `TOP n BY field` the n with the highest field, `SINCE c` the first one matching `c`
  and every later one.
- Then the rest of the statement applies to what the window kept: `ORDER BY`,
  `LIMIT`, `RETURN`, `AGGREGATE` / `GROUP BY`, and `LIKE` ranking. Without `ORDER BY`
  the result is newest first, as for every EPISODIC read.
- `WATCH` with `WINDOW LAST <duration>` re-runs on writes; episodes leave the window by
  age only at the next run.

## 3. Removing a field: `UPDATE … (field = null)` (D2 §8)

```sql
UPDATE INTO SEMANTIC WHERE concept = "acme" (draft_note = null, status = "final")
```

- In `UPDATE`, a field set to `null` is **removed** from the record; the other fields
  are merged as before. A removed field no longer matches `WHERE`, and `RETURN` leaves
  it out.
- On SEMANTIC and PROCEDURAL the new version lacks the field (older versions keep
  it, as history); on EPISODIC, WORKING and TOOLS the record is patched in place.
  Subscribers receive the patch with the field `null`.
- Removing `content` also drops the record's text and vector.
- `STORE` is unchanged: there, `field = null` stores a null.

## 4. Corrections to v0.5

- **§14 modifier list.** v0.5 §14's BNF listed `weight_mod ::= "WEIGHT" field`,
  `threshold_mod ::= "THRESHOLD" float` and `source_mod ::= "FROM" identifier_list`. No
  grammar had them and no example used them as modifiers; they are withdrawn.
  `THRESHOLD` stays where §16's examples use it, after a pattern
  (`LOOKUP FROM PROCEDURAL PATTERN $p THRESHOLD 0.7`), and `WEIGHT` on `LINK` (§9).
- The modifiers, as the grammar defines them through v0.9: `LIMIT`, `ORDER BY`,
  `RETURN`, `TIMEOUT`, `MIN_CONFIDENCE`, `SCOPE`, `NAMESPACE`, `TTL`, `AGGREGATE`,
  `GROUP BY`, `HAVING`, `WITH LINKS`, `FOLLOW LINKS`, `WINDOW`, `IN SCOPE`.
- **§19 `WINDOW`** is replaced by §2 above.

## Validation (additions to v0.5 §19)

```
GROUP BY requires AGGREGATE:
  RECALL FROM EPISODIC WHERE a = 1 GROUP BY url → error: "GROUP BY requires AGGREGATE"
HAVING requires AGGREGATE (v0.5 §19, now enforced by the parser):
  RECALL FROM EPISODIC WHERE a = 1 HAVING n > 1 → error: "HAVING requires AGGREGATE"
WINDOW only on SCAN FROM WORKING and RECALL FROM EPISODIC (replaces the v0.5 rule):
  RECALL FROM SEMANTIC … WINDOW LAST 10 → error: "WINDOW is only valid on SCAN FROM
  WORKING and RECALL FROM EPISODIC"
```
