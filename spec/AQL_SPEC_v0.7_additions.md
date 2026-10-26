# AQL v0.7 — Additions (draft): changing and removing links

> **Date:** 26 Oct 2026 · @Sriram · **Status:** accepted for v0.7; grammar, parser,
> WASM playground source and ADB implemented; conformance suite `22_link_update.yaml`.
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md) and
> [`AQL_SPEC_v0.6_additions.md`](AQL_SPEC_v0.6_additions.md). Every v0.5 / v0.6
> statement parses to the same AST as before.

v0.5 could create a link (`LINK`, §9) but not change or remove one: a second
`LINK` added a parallel link, and a link disappeared only when a record at one of
its ends was forgotten. v0.5 §22 listed "LINK WEIGHT update" as a future extension.
v0.7 adds it, adds `UNLINK`, and makes a repeated `LINK` update instead of
duplicating:

| Statement | What it does |
|---|---|
| `UPDATE LINK` | Change the weight and/or the type of existing links |
| `UNLINK` | Remove links; the records at both ends stay |
| `LINK` (changed) | Linking the same two records with the same type again sets that link's weight |

## 1. Grammar (BNF)

```bnf
write_stmt       ::= … | update_link_stmt | unlink_stmt

update_link_stmt ::= "UPDATE" "LINK"
                     "FROM" memory_type "WHERE" condition
                     "TO" memory_type "WHERE" condition
                     link_type_filter?
                     ( "WEIGHT" float | "(" link_assignment ("," link_assignment)* ")" )

unlink_stmt      ::= "UNLINK"
                     "FROM" memory_type "WHERE" condition
                     "TO" memory_type "WHERE" condition
                     link_type_filter?

link_type_filter ::= "TYPE" string_literal
link_assignment  ::= "weight" "=" number | "type" "=" string_literal
```

## 2. UPDATE LINK

```sql
-- The v0.5 §22 form: a new weight
UPDATE LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  TO EPISODIC WHERE incident_id = "inc-001"
  TYPE "applied_to" WEIGHT 0.99

-- An assignment list (as in UPDATE INTO): a new type, a new weight, or both
UPDATE LINK FROM SEMANTIC WHERE key = "acme-owner"
  TO SEMANTIC WHERE key = "acme-renewal"
  TYPE "related" (type = "owns", weight = 0.9)
```

- The `FROM … TO …` ends are matched like `LINK`'s; the statement addresses **every
  link from a matched source to a matched target**, of `TYPE` if given (every type
  otherwise).
- `WEIGHT w` and `(weight = w)` are the same. The assignment list sets only `weight`
  (a number) and `type` (a string); anything else is a validation error, and so is
  an `UPDATE LINK` that sets nothing.
- The result is the number of links changed. No matching records or links → `0`,
  not an error (as `UPDATE` and `FORGET` with no match). `ALL` is not allowed as an
  end (as for `LINK`, B14).
- History: an engine that versions records versions links the same way (ADB: a new
  version of the edge; the old one is kept as history, D2). Subscribers see a
  `supersede` change of kind `edge`.

## 3. UNLINK, and LINK again

```sql
UNLINK FROM SEMANTIC WHERE key = "acme-owner" TO SEMANTIC WHERE key = "acme-renewal" TYPE "related"
UNLINK FROM SEMANTIC WHERE key = "acme-owner" TO SEMANTIC WHERE key = "acme-renewal"
```

- Removes the links between the matched records, of `TYPE` if given (every link
  between them otherwise). The records stay. The result is the number removed; `0`
  when there are none.
- Subscribers see an `unlink` change of kind `edge` (`forget` remains the change for
  records; forgetting a record still removes its links, v0.5).
- **`LINK` between the same two records with the same type again sets that link's
  weight** (no `WEIGHT`: the weight is unchanged) instead of adding a parallel link.
  Different types between the same records are different links.

## 4. New keywords

`UNLINK`. (`UPDATE`, `LINK`, `TYPE`, `WEIGHT` already exist.) Case-insensitive; not
reserved as field names.

## 5. Implementation status

| Component | Status |
|---|---|
| `aql-parser` grammar, AST (`UpdateLinkStmt`, `UnlinkStmt`) | Done — 4 parser tests |
| WASM playground (`clawdb-wasm`) | Done in source; the built `pkg/` needs `wasm-pack build crates/clawdb-wasm` (not run here) |
| ClawDB server (`clawdb`) | Rejects them ("ADB implements them") |
| ADB | Done (durable and in-memory engines; MCP `unlink` tool) |
| Conformance | `tests/suites/22_link_update.yaml`, 24 tests (ADB 140/140 with all suites); the runner now checks `links_count`, `links_contains`, `links_present`, `links_type_only` |
