# AQL v0.6 — Additions (draft)

> **Date:** 9 Oct 2026 · @Sriram · **Status:** accepted for v0.6 (ADB D9); grammar and
> parser implemented, execution pending in ADB.
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md). Everything in v0.5 is unchanged:
> every v0.5 conformance and seed query parses to the same AST as before.
> **Engine semantics:** ADB `SUBSCRIPTIONS_CQ_SPEC.md` and `ARCHITECTURE.md` §11.7.

v0.6 adds six verbs that let an agent learn about changes without polling, and a
named-scope clause:

| Verb | What it does |
|---|---|
| `SUBSCRIBE` | Follow every committed change in a scope or topic |
| `UNSUBSCRIBE` | Stop a subscription |
| `ACK` | Move a subscription's cursor forward |
| `PUBLISH` | Write a message to a topic |
| `WATCH` | Register a continuous query: a snapshot, then diffs as the result changes |
| `UNWATCH` | Stop a continuous query |
| `IN SCOPE "…"` | Name the scope a statement reads or writes |

## 1. Top-level grammar (BNF)

```bnf
statement    ::= read_stmt | write_stmt | forget_stmt | link_stmt
               | reflect_stmt | pipeline_stmt
               | subscribe_stmt | unsubscribe_stmt | ack_stmt
               | publish_stmt | watch_stmt | unwatch_stmt          -- v0.6
```

## 2. Subscriptions

```bnf
subscribe_stmt   ::= "SUBSCRIBE" subscribe_target kinds_clause? where_pred? start_clause?
subscribe_target ::= "SCOPE" string_literal | "TOPIC" string_literal
kinds_clause     ::= "KINDS" record_kind ("," record_kind)*
record_kind      ::= "entity" | "fact" | "edge" | "episode" | "tool" | "event"
start_clause     ::= "FROM" ("NOW" | "OFFSET" integer_literal)

unsubscribe_stmt ::= "UNSUBSCRIBE" handle
ack_stmt         ::= "ACK" handle "OFFSET" integer_literal
publish_stmt     ::= "PUBLISH" "TOPIC" string_literal payload modifier*
handle           ::= string_literal | variable | identifier
```

```sql
SUBSCRIBE SCOPE "team:billing" KINDS fact, edge FROM NOW
SUBSCRIBE TOPIC "orders.events" WHERE status = "paid" FROM OFFSET 1042
ACK "sub-7f3a" OFFSET 1050
PUBLISH TOPIC "orders.events" (order_id = "o-1", status = "paid") TTL 7d
UNSUBSCRIBE "sub-7f3a"
```

**Semantics** (engine detail in ADB's spec):

- A scope or topic lives in one partition, so every subscriber sees changes in one
  order. Each subscriber has its own cursor (broadcast, not a work queue).
- Deliveries carry `(partition, offset, txn_id, effects[])` and are **at-least-once**;
  clients drop offsets at or below their last `ACK`.
- `FROM NOW` (the default) starts at the commit tip. `FROM OFFSET n` replays from `n` if
  it is still retained; otherwise the engine returns `cursor_expired` with the oldest
  available offset, or a reset with current state.
- `WHERE` filters may use only plaintext columns (`tenant_id`, `scope`, `memory`,
  `kind`, `predicate`, `subject_id`, `object_id`, time columns, `text`).
- `PUBLISH` is an ordinary committed write of kind `event`; `TTL` sets its retention.

## 3. Continuous queries

```bnf
watch_stmt      ::= "WATCH" watch_query debounce_clause? start_clause?
watch_query     ::= recall_stmt | lookup_stmt | scan_stmt | load_stmt
debounce_clause ::= "DEBOUNCE" duration_literal
unwatch_stmt    ::= "UNWATCH" handle
```

```sql
WATCH RECALL FROM SEMANTIC IN SCOPE "agent:42" WHERE predicate = "owns" LIMIT 20
      DEBOUNCE 100ms FROM NOW
UNWATCH "cq-91"
```

- The response is a snapshot `{cq_id, offset: N, result}` followed by diffs
  `{cq_id, offset: M, added, removed, changed}`. Snapshot at N then tail from N + 1
  means no missed and no duplicated changes.
- The grammar only allows read statements under `WATCH`; writes, `REFLECT` and
  `PIPELINE` are parse errors. The engine further limits the query to the **WATCH
  subset** (filters / point lookups, current facts, count / sum / min / max, graph
  traversal up to depth 3, top-k vector recall, hybrid recall) and rejects anything
  else at registration with the reason.
- `DEBOUNCE` (50–500 ms, default 100 ms) applies to queries the engine re-runs; it is
  ignored for incrementally maintained ones.

## 4. Named scope: `IN SCOPE`

```bnf
in_scope_mod ::= "IN" "SCOPE" string_literal
recall_stmt  ::= "RECALL" "FROM" memory_type in_scope_mod? predicate? modifier*
lookup_stmt  ::= "LOOKUP" "FROM" memory_type in_scope_mod? predicate modifier*
modifier     ::= … | in_scope_mod
```

```sql
RECALL FROM SEMANTIC IN SCOPE "agent:42" WHERE predicate = "owns"
STORE INTO SEMANTIC (fact = "renewal due") IN SCOPE "team:billing" TTL 30d
```

- `IN SCOPE "<scope>"` names a memory boundary (`agent:{id}`, `team:{name}`,
  `account:{id}`), which decides placement in ADB. It may follow the memory type in
  `RECALL` / `LOOKUP`, or appear anywhere among a statement's modifiers.
- It is **separate from** the v0.5 `SCOPE private | shared | cluster` modifier, which
  is unchanged. The two can appear together.
- `IN` inside a `WHERE` condition is still the membership operator
  (`WHERE pod IN ["a", "b"] IN SCOPE "agent:1"` parses as expected).

## 5. New keywords

`SUBSCRIBE`, `UNSUBSCRIBE`, `ACK`, `PUBLISH`, `WATCH`, `UNWATCH`, `TOPIC`, `KINDS`,
`NOW`, `OFFSET`, `DEBOUNCE`. Keywords are case-insensitive, as in v0.5. They are not
reserved as field names.

## 6. Implementation status

| Component | Status |
|---|---|
| `aql-parser` grammar, AST (`SubscribeStmt`, `UnsubscribeStmt`, `AckStmt`, `PublishStmt`, `WatchStmt`, `UnwatchStmt`, `Modifiers.in_scope`) | Done — 11 parser tests |
| v0.5 compatibility | Verified: all 132 conformance + seed queries produce identical ASTs |
| ClawDB executor, WASM playground | Reject the new statements with "not supported yet" |
| ADB execution | Done: subscriptions (stage E), `WATCH` (W6), `IN SCOPE` (W3) |
| Conformance suite | `tests/suites/18_subscriptions.yaml`, `19_watch.yaml`, `20_in_scope.yaml` (42 tests, `requires: server`); ADB 182/182 with all suites (28 Oct) |
