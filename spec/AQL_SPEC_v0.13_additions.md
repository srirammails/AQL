# AQL v0.13 — Additions: MERGE ENTITY within a namespace

> **Date:** 17 Nov 2026 · @Sriram · **Status:** implemented in the grammar, parser and
> ADB; conformance suite `32_merge_namespace.yaml`. ClawDB and the WASM playground
> reject `MERGE ENTITY` (as since v0.10).
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md) and the v0.6–v0.12 additions.
> Every older statement parses as before, and its AST serializes to the same JSON.

v0.12 let every statement but one name its namespace. `MERGE ENTITY` (v0.10) still
couldn't, so a server that keeps each project's statements inside the project's
memory had no way to merge a project's entities. v0.13 closes that.

## 1. `MERGE ENTITY … NAMESPACE`

```sql
MERGE ENTITY "person:ada-lovelace" INTO "person:ada" NAMESPACE "crm"
```

- Both entities are **the namespace's**: `"person:ada-lovelace"` and `"person:ada"`
  of namespace `crm`. An entity of the same key in another namespace, or without a
  namespace, is untouched. With no such entity in the namespace, `MERGE` fails as it
  always has ("no entity …").
- The merged key becomes an alias **in that namespace**: `LOOKUP FROM SEMANTIC KEY id =
  "person:ada-lovelace" NAMESPACE "crm"`, and a `FACT` stored in `crm` about it, find
  the survivor. The survivor's `aliases` field holds the bare key.
- Without `NAMESPACE`, `MERGE ENTITY` merges entities that have no namespace, as
  before.
- `NAMESPACE` is the **last** clause. `MERGE` takes no `BRANCH`.

## 2. Grammar

```
merge_stmt ::= "MERGE" "ENTITY" string_literal "INTO" string_literal namespace_mod?
```

The AST carries it as `namespace` on the `Merge` statement, absent when not given.

## Validation (additions to v0.5 §19)

```
NAMESPACE is the last clause:
  MERGE ENTITY "a" NAMESPACE "p" INTO "b" → parse error
MERGE takes no BRANCH:
  MERGE ENTITY "a" INTO "b" NAMESPACE "p" BRANCH "x" → parse error
Both entities are in the namespace (server):
  MERGE ENTITY "a" INTO "b" NAMESPACE "empty" → error: "MERGE: no entity a in namespace \"empty\""
```
