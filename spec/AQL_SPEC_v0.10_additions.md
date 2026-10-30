# AQL v0.10 — Additions (draft): time

> **Date:** 30 Oct 2026 · @Sriram · **Status:** draft; implemented in the grammar,
> parser and ADB; conformance suite `27_time.yaml`. ClawDB and the WASM playground
> reject `AS OF`, `VALID AT` and `WITH HISTORY`.
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md) and the v0.6–v0.9 additions.
> Every older statement parses as before, and its AST serializes to the same JSON.

Memory has two clocks. **Recorded time** is when a version was written: every
`UPDATE` of a SEMANTIC or PROCEDURAL record, and every `STORE` that replaces a fact,
keeps the old version (ADB's temporal model, D2). **Valid time** is when a fact holds
in the world, which only the agent knows: "Ada works at Acme from 2020", "the price is
valid until 1 Nov". v0.10 lets facts carry valid time and lets reads look back along
both clocks.

## 1. Fact validity: `valid_from` / `valid_to`

```sql
STORE INTO SEMANTIC FACT (subject = "person:ada", predicate = "works_at",
                          object = "org:acme", valid_from = "2020-01-01")
STORE INTO SEMANTIC FACT (subject = "sku:42", predicate = "price", value = 10,
                          valid_to = "2026-11-01T00:00:00+01:00")
```

- On `STORE … FACT`, `valid_from` and `valid_to` are **reserved fields**: a date
  (`"2026-11-01"`, midnight UTC), a time without an offset (`"2026-11-01T09:30:00"`,
  UTC) or an RFC 3339 time. They are stored in one form, RFC 3339 in UTC
  (`"2026-11-01T00:00:00Z"`, with microseconds only when given). Either may be left
  out (unbounded); `valid_from` must be before `valid_to`; anything else is an error.
- A fact **holds** at t when `valid_from ≤ t < valid_to`. A fact that doesn't hold now
  is **not in the current view**: `RECALL`, `LOOKUP`, `LIKE`, `AGGREGATE`, `FOLLOW
  LINKS` and `WITH LINKS` skip it, as they skip a superseded version. It becomes
  visible at `valid_from` and leaves at `valid_to` without a write.
- It is still the fact's **current version**: storing the same (subject, predicate)
  again replaces it, as before (v0.8 §6), and `FORGET` removes it. `UPDATE` matches
  the current view only, so a fact that doesn't hold now can't be updated; store it
  again instead.
- `UPDATE … (valid_to = "…")` moves a bound of a fact that holds now; `valid_to =
  null` removes it (v0.9 §3). The same checks apply.
- Records other than facts have no validity: `valid_from` / `valid_to` are ordinary
  fields there.

**Events.** When a bound passes, ADB commits an event: subscribers to the fact's scope
receive `begin` (at `valid_from`) or `end` (at `valid_to`) for the fact's id, and
continuous queries over SEMANTIC re-run. A bound that passed while ADB was stopped is
delivered when it starts. A fact's `put` carries its bounds, so a subscriber knows
whether it holds yet.

## 2. History reads

```bnf
modifier         ::= … | as_of_mod | valid_at_mod | with_history_mod
as_of_mod        ::= "AS" "OF" time_point
valid_at_mod     ::= "VALID" "AT" time_point
with_history_mod ::= "WITH" "HISTORY"
time_point       ::= string_literal | variable | duration "AGO"
```

```sql
RECALL FROM SEMANTIC WHERE subject = "person:ada" AS OF "2026-10-01"
RECALL FROM SEMANTIC WHERE subject = "person:ada" VALID AT "2015-06-01"
LOOKUP FROM PROCEDURAL KEY pattern_id = "deploy" WITH HISTORY
RECALL FROM SEMANTIC WHERE predicate = "price" AS OF 7d AGO
RECALL FROM SEMANTIC WHERE subject = $who AS OF $then
```

| Modifier | Returns |
|---|---|
| `AS OF t` | What memory held at t: each record's version current at t (recorded time), if it held at t (valid time). |
| `VALID AT v` | The facts that hold at v, as memory knows them now (or at `AS OF`, if given). |
| `WITH HISTORY` | Every version recorded (by `AS OF`, if given), current and replaced, whatever its validity; with `VALID AT`, those holding at v. |

- On `RECALL` and `LOOKUP` over **SEMANTIC and PROCEDURAL**, the memory types that
  keep versions. `VALID AT` only on SEMANTIC. Anything else is an error.
- A time point is a string as in §1, a variable bound to one, or `<duration> AGO`.
- Each record carries `_recorded_at` (when its version was written) and
  `_replaced_at` (when a newer version replaced it, as of `AS OF`; else `null`), next
  to `metadata.version`.
- `WHERE`, `IN SCOPE`, `NAMESPACE`, `ORDER BY`, `LIMIT`, `RETURN` and `AGGREGATE` /
  `GROUP BY` apply as usual (an aggregate counts versions with `WITH HISTORY`).
  Without `ORDER BY`, oldest first.
- Not with `LIKE`, `PATTERN`, `FOLLOW LINKS` or `WITH LINKS` in v0.10 (an error).
- **What history is kept** is the implementation's: a forgotten record has none (D2
  T5), nor does an expired one. ADB keeps replaced versions for 90 days.

## Validation (additions to v0.5 §19)

```
AS OF, VALID AT and WITH HISTORY only on RECALL and LOOKUP:
  SCAN FROM WORKING WITH HISTORY → error: "AS OF, VALID AT and WITH HISTORY are only
  valid on RECALL and LOOKUP"
On SEMANTIC and PROCEDURAL only:
  RECALL FROM EPISODIC … AS OF 1d AGO → error: "AS OF and WITH HISTORY are only valid
  on SEMANTIC and PROCEDURAL"
VALID AT on SEMANTIC only:
  RECALL FROM PROCEDURAL … VALID AT "2026-01-01" → error: "VALID AT is only valid on
  SEMANTIC"
```

`AS OF` after an aggregate is the modifier, not an alias: `AGGREGATE COUNT(*) AS OF 1d
AGO` counts as of a day ago; `AS n AS OF 1d AGO` names the count `n`.
