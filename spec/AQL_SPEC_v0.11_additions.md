# AQL v0.11 — Additions: memory branches

> **Date:** 4 Nov 2026 · @Sriram · **Status:** released in AQL 0.12.0; implemented in the grammar, parser and
> ADB; conformance suite `30_branch.yaml`. ClawDB and the WASM playground reject
> `BRANCH`.
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md) and the v0.6–v0.10 additions.
> Every older statement parses as before, and its AST serializes to the same JSON.

An agent's memory can be branched like a code repository: an agent tries a new
behaviour on a branch of its memory, and the branch is merged back, or dropped,
once the result is known. v0.11 lets a statement say which branch it reads and
writes.

## 1. `BRANCH "<name>"`

```sql
RECALL FROM SEMANTIC WHERE subject = "acme" NAMESPACE "crm" BRANCH "try-new-recall" LIMIT 5
STORE INTO EPISODIC (event = "tried the new recall", outcome = "better") NAMESPACE "crm" BRANCH "try-new-recall"
FORGET FROM SEMANTIC WHERE key = "stale" NAMESPACE "crm" BRANCH "try-new-recall"
```

- `BRANCH` is a **modifier**, like `NAMESPACE`: it goes on `RECALL`, `LOOKUP`, `LOAD`,
  `SCAN`, `STORE`, `UPDATE` and `FORGET`, in any order with the other modifiers. In
  v0.12 `LINK`, `UPDATE LINK` and `UNLINK` take it too, after their `NAMESPACE`
  ([v0.12 §1](AQL_SPEC_v0.12_additions.md)).
- A branch belongs to **one namespace**: a statement naming a branch names its
  `NAMESPACE` too.
- `BRANCH "main"` is the namespace itself, as is a statement without `BRANCH`.
- **Reads** on a branch see the namespace as it was at the branch point, plus the
  branch's own writes.
- **Writes** on a branch land on the branch only. `UPDATE` of a record the branch
  inherited writes the new version on the branch, and `FORGET` hides it on the
  branch; the namespace keeps it.
- A **pipeline** on a branch names it in every stage.

Branches are **created, merged, rebased and dropped through the server's API**, not
through AQL: those are operations on a memory space, with their own permissions,
and they don't fit a statement's result. ADB's API is described in its own docs.

## 2. What a server may not support yet

A server that keeps branches may still refuse some reads on them. ADB currently
refuses `LIKE`, `WITH LINKS`, `FOLLOW LINKS`, `PATTERN` and the history reads (`AS
OF`, `VALID AT`, `WITH HISTORY`) on a branch, with an error that says so.

## 3. Grammar

```
modifier   ::= … | namespace_mod | branch_mod
branch_mod ::= "BRANCH" string_literal
```

The AST carries it as `modifiers.branch` (absent when not given).

## Validation (additions to v0.5 §19)

```
A branch is named by a string:
  RECALL FROM SEMANTIC ALL NAMESPACE "crm" BRANCH b1 → parse error
A branch needs its namespace (server):
  STORE INTO SEMANTIC (key = "k") BRANCH "b1" → error: "BRANCH "b1" needs NAMESPACE"
A branch must exist (server):
  RECALL FROM SEMANTIC ALL NAMESPACE "crm" BRANCH "nope" → error: "no branch "nope" in namespace "crm""
A pipeline names one branch in every stage, or none (server):
  … → error: "every stage of a pipeline on branch "b1" must name it"
```
