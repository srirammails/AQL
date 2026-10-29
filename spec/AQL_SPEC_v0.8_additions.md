# AQL v0.8 — Additions (draft): graph reads

> **Date:** 29 Oct 2026 · @Sriram · **Status:** accepted for v0.8; grammar, parser
> and ADB implemented (ADB `.ai/P1_GRAPH.md`); conformance suite `23_graph.yaml`.
> The WASM playground parses v0.8 but ignores direction and `MIN_WEIGHT`.
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md),
> [`AQL_SPEC_v0.6_additions.md`](AQL_SPEC_v0.6_additions.md) and
> [`AQL_SPEC_v0.7_additions.md`](AQL_SPEC_v0.7_additions.md). Every older statement
> parses as before, and its AST serializes to the same JSON.

Before v0.8, `FOLLOW LINKS` and `WITH LINKS` read one type of outgoing link, and
`DEPTH` was parsed but not defined. v0.8 defines multi-hop traversal and lets both
modifiers choose links by type, direction and weight.

## 1. Grammar (BNF)

```bnf
with_links_mod   ::= "WITH" "LINKS" link_select link_direction? min_weight?
follow_links_mod ::= "FOLLOW" "LINKS" link_select? link_direction? min_weight?
                     ("DEPTH" integer)?

link_select      ::= "ALL"
                   | "TYPE" string_literal
                   | "TYPE" "IN" "(" string_literal ("," string_literal)* ")"
link_direction   ::= "OUTBOUND" | "INBOUND" | "BOTH"
min_weight       ::= "MIN_WEIGHT" number
```

`FOLLOW LINKS` without a type follows every type, as `ALL` does. The direction
defaults to `OUTBOUND`.

## 2. Multi-hop `FOLLOW LINKS … DEPTH n`

`FOLLOW LINKS` returns the records it reaches, not the ones it started from. With
`DEPTH n` it returns **every record reached within 1..n hops**:

- breadth-first; each record once, at its shortest hop; the starting records never;
- within a hop, by **path weight** (the product of the link weights along the best
  path to the record), then link order;
- each result carries `_hop` (1..n) and `_path_weight`;
- `DEPTH` defaults to 1. An implementation may cap it (ADB: 5) and the number of
  records one traversal returns (ADB: 1,000).

```sql
RECALL FROM SEMANTIC WHERE concept = "a" FOLLOW LINKS TYPE "next" DEPTH 3
-- a →1.0→ b →0.5→ c →0.5→ d:  b (_hop 1, 1.0), c (2, 0.5), d (3, 0.25)
```

`RECALL`, `LOOKUP` and `REFLECT` all follow links this way. (Before v0.8, ADB's
`LOOKUP … FOLLOW LINKS` returned the source record.)

## 3. Direction

| Direction | Links read |
|---|---|
| `OUTBOUND` (default) | leaving the record; the walk goes to their targets |
| `INBOUND` | pointing at the record; the walk goes to their sources |
| `BOTH` | either |

`WITH LINKS … INBOUND` / `BOTH` adds `from_type` and `from_id` to each link in a
record's `links`.

```sql
RECALL FROM SEMANTIC WHERE concept = "b" FOLLOW LINKS TYPE "next" INBOUND
LOOKUP FROM SEMANTIC KEY id = "c" WITH LINKS ALL BOTH
```

## 4. `MIN_WEIGHT`

Only links whose weight is at least `w` are read or followed.

```sql
RECALL FROM PROCEDURAL WHERE pattern = "oom" FOLLOW LINKS TYPE "applied_to" MIN_WEIGHT 0.7
```

## 5. Several types

`TYPE IN ("a", "b")` reads links of any of the listed types; `ALL` reads every type.

```sql
RECALL FROM SEMANTIC WHERE concept = "ada" FOLLOW LINKS TYPE IN ("knows", "works_with") DEPTH 2
```
