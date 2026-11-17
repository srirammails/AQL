# AQL v0.12 — Additions: every statement can name its namespace

> **Date:** 17 Nov 2026 · @Sriram · **Status:** released in AQL 0.12.0; implemented in the grammar, parser and
> ADB; conformance suite `31_namespace_statements.yaml`. ClawDB and the WASM
> playground reject `NAMESPACE` on links (they don't implement topics).
> **Extends:** [`AQL_SPEC_v0.5.md`](AQL_SPEC_v0.5.md) and the v0.6–v0.11 additions.
> Every older statement parses as before, and its AST serializes to the same JSON.

A server that holds many agents' and projects' memory needs every statement to say
whose memory it touches. On a partitioned server, a statement that names no
namespace has to run on every partition; and a platform can only keep a project's
statements inside the project's memory if each one names it. Until v0.12, `LINK`,
`UPDATE LINK`, `UNLINK` and the topic statements couldn't name a namespace at all.
v0.12 lets them; the semantics of statements without one don't change.

## 1. Links within a namespace

```sql
LINK FROM SEMANTIC WHERE key = "person:ada" TO EPISODIC WHERE key = "call-17"
     TYPE "took_part" WEIGHT 0.8 NAMESPACE "crm"
UPDATE LINK FROM SEMANTIC WHERE key = "person:ada" TO EPISODIC WHERE key = "call-17"
     TYPE "took_part" WEIGHT 0.9 NAMESPACE "crm"
UNLINK FROM SEMANTIC WHERE key = "person:ada" TO EPISODIC WHERE key = "call-17" NAMESPACE "crm"
LINK FROM SEMANTIC WHERE key = "a" TO SEMANTIC WHERE key = "b" TYPE "rel"
     NAMESPACE "crm" BRANCH "try-new-recall"
```

- `NAMESPACE "<ns>"` goes **after** the rest of the statement (after `WEIGHT` on `LINK`;
  after the new weight or type on `UPDATE LINK`; after `TYPE` on `UNLINK`).
- Both ends are matched **in that namespace only**: a `LINK` with a namespace links
  that namespace's records, never another's. With no match it fails as `LINK` always
  has (`UPDATE LINK` and `UNLINK` count 0).
- `BRANCH "<name>"` (v0.11) may follow the `NAMESPACE`: the link is on that branch.
- Without `NAMESPACE`, a link statement matches its ends in every namespace, as before.

## 2. A namespace's topics

```sql
SUBSCRIBE TOPIC "orders" WHERE status = "paid" FROM NOW NAMESPACE "shop"
PUBLISH TOPIC "orders" (order_id = "o-1", status = "paid") TTL 7d NAMESPACE "shop"
ACK "sub-7f3a" OFFSET 1042 NAMESPACE "shop"
UNSUBSCRIBE "sub-7f3a" NAMESPACE "shop"
```

- `SUBSCRIBE TOPIC "t" … NAMESPACE "ns"` and `PUBLISH TOPIC "t" … NAMESPACE "ns"` use
  **the namespace's own topic** `t`. It is apart from topic `t` of every other
  namespace and from the org-wide topic `t` (a statement with no `NAMESPACE`). A
  server names it `<ns>/<t>` in its answers (`"topic": "shop/orders"`).
- `ACK` and `UNSUBSCRIBE` with `NAMESPACE "ns"` act only on a subscription to a topic
  of `ns`. For any other subscription, `ACK` fails as for an unknown one ("not found")
  and `UNSUBSCRIBE` answers `"unsubscribed": false`.
- `NAMESPACE` is the **last** clause: after `FROM NOW` / `FROM OFFSET` on `SUBSCRIBE`,
  after `OFFSET` on `ACK`. (`PUBLISH` takes it among its modifiers, as since v0.6.)
- Topics are not on branches: a topic statement takes no `BRANCH`.
- `SUBSCRIBE SCOPE … NAMESPACE` is reserved; a server may refuse it.

## 3. Grammar

```
link_stmt        ::= "LINK" "FROM" memory_type "WHERE" condition_list
                     "TO" memory_type "WHERE" condition_list
                     "TYPE" string_literal ("WEIGHT" float_literal)?
                     namespace_mod? branch_mod?
update_link_stmt ::= "UPDATE" "LINK" … link_type_filter? (link_weight_set | payload)
                     namespace_mod? branch_mod?
unlink_stmt      ::= "UNLINK" … link_type_filter? namespace_mod? branch_mod?
subscribe_stmt   ::= "SUBSCRIBE" subscribe_target kinds_clause? where_pred?
                     start_clause? namespace_mod?
unsubscribe_stmt ::= "UNSUBSCRIBE" handle namespace_mod?
ack_stmt         ::= "ACK" handle "OFFSET" integer_literal namespace_mod?
```

The AST carries them as `namespace` (and `branch` on the link statements), absent
when not given, so an older statement serializes to the same JSON.

## Validation (additions to v0.5 §19)

```
NAMESPACE is the last clause:
  ACK "s" NAMESPACE "p" OFFSET 1 → parse error
A topic is not on a branch:
  SUBSCRIBE TOPIC "t" NAMESPACE "p" BRANCH "b" → parse error
A LINK's ends are in its namespace (server):
  LINK … NAMESPACE "empty" → error: "No source records found"
```
