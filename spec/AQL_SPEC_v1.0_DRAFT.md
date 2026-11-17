# AQL 1.0 — draft for review

> **Status:** draft, 17 Nov 2026, not implemented. Replaces v0.5 and the v0.6–v0.13
> additions with one language. No compatibility with 0.x: statements change form.

AQL is how an agent talks to its memory. 1.0 keeps it small: **13 verbs**, each
working on every memory type; the engine works out *how* from what is asked.

```
Memory      RECALL  STORE  UPDATE  FORGET  LINK  UNLINK
Namespaces  BRANCH  FORK  MERGE
Flow        PIPELINE  SUBSCRIBE  UNSUBSCRIBE  PUBLISH
```

Memory types: `EPISODIC` (what happened), `SEMANTIC` (what is known: entities and
facts), `PROCEDURAL` (how to do things), `TOOLS` (what can be used), `WORKING` (the
current session), and `ALL` for reads and FORGET.

Every statement names whose memory it touches with `NAMESPACE "<ns>"` (a server may
require it). `IN SCOPE "<scope>"` narrows to a shared group inside the namespace.

---

## 1. RECALL — every read

```sql
RECALL FROM <memory> [IN SCOPE "s"] [WHERE <conditions> | LIKE <text or $var>] <modifiers>
```

One verb; the engine picks the access path:

| You write | The engine does |
|---|---|
| `RECALL FROM SEMANTIC WHERE key = "person:ada"` | exact lookup by key |
| `RECALL FROM SEMANTIC WHERE subject = "person:ada"` | facts about an entity (and its aliases, §4) |
| `RECALL FROM EPISODIC WHERE pod = "payments" ORDER BY created_at DESC LIMIT 10` | filtered read |
| `RECALL FROM SEMANTIC LIKE $question MIN_CONFIDENCE 0.7` | similarity (vector + words) |
| `RECALL FROM PROCEDURAL LIKE "pod OOMKilled restart"` | the best-matching procedures |
| `RECALL FROM TOOLS WHERE task = "bidding" LIMIT 3` | tools ranked for the task |
| `RECALL FROM WORKING WINDOW LAST 30s` | the session's current state, newest first |
| `RECALL FROM ALL WHERE url = "example.com"` | every memory type |

No `WHERE` reads everything (within LIMIT). `LIKE` takes a variable or literal text.

## 2. STORE, UPDATE, FORGET

```sql
STORE  INTO <memory> (<field> = <value>, …) <modifiers>
UPDATE INTO <memory> WHERE <conditions> (<field> = <value>, …) <modifiers>
FORGET FROM <memory> [WHERE <conditions>] <modifiers>
```

- A SEMANTIC payload with `subject` and `predicate` is a **fact**
  (`STORE INTO SEMANTIC (subject = "person:ada", predicate = "works_at", object = "org:acme")`);
  `multi = true` keeps several objects for one predicate.
- `FORGET` erases (a tombstone no read can miss). `FORGET FROM ALL NAMESPACE "x"` with
  no `WHERE` drops the whole namespace — also how a branch is dropped.
- `TTL 7d` on STORE / UPDATE.

## 3. LINK, UNLINK

```sql
LINK   FROM <memory> WHERE <c> TO <memory> WHERE <c> TYPE "t" [WEIGHT 0.8] <modifiers>
UNLINK FROM <memory> WHERE <c> TO <memory> WHERE <c> [TYPE "t"] <modifiers>
```

LINK on a link that exists sets its weight (create or update). A link's two ends are
in the statement's namespace.

## 4. Entities known by another name (no MERGE ENTITY)

Two keys turn out to name one thing. Two ordinary statements:

```sql
UPDATE INTO SEMANTIC WHERE key = "person:ada" (aliases = ["person:ada-lovelace"]) NAMESPACE "crm"
FORGET FROM SEMANTIC WHERE key = "person:ada-lovelace" NAMESPACE "crm"
```

- An entity's `aliases` are other keys for it: a read or a new fact using an alias
  reaches the entity; `WHERE subject = "person:ada"` also matches facts stored under
  its aliases. Nothing is rewritten.
- FORGET of an entity removes its own record, not the facts that mention it.
- Either order is safe; a failure between the two leaves nothing half-done.

## 5. BRANCH, FORK, MERGE — namespaces

```sql
BRANCH FROM "crm" INTO "crm-try"                         -- crm as it is now
BRANCH FROM "crm" INTO "crm-before-import" AS OF "2026-11-01T09:00:00Z"
FORK   FROM "crm" INTO "crm-copy"                        -- a full, independent copy
FORK   FROM "crm" INTO "crm-nov-1" AS OF "2026-11-01T00:00:00Z"
MERGE  FROM "crm-try" INTO "crm"                         -- the branch's changes into crm
MERGE  FROM "crm" INTO "crm-try"                         -- crm's newer changes into the branch
MERGE  FROM "crm-try" INTO "crm" PREFER SOURCE           -- conflicts: the branch wins
FORGET FROM ALL NAMESPACE "crm-try"                      -- drop the branch
```

- **BRANCH** makes `INTO` a new namespace whose base is `FROM` at a point in time (now,
  or `AS OF` a time). Reads see the base at that point plus the branch's own writes;
  writes stay in the branch. Nothing is copied. A branch can be branched again.
- **FORK** is BRANCH with a full copy: `INTO` gets `FROM`'s records as they were at
  the point (now, or `AS OF`) and no longer reads through, so later changes to (or a
  drop of) `FROM` don't reach it. It remembers where it came from, so it can still be
  merged either way.
- **MERGE** applies the changes `FROM` made since the two parted to `INTO`, in one
  write. When both changed the same record it fails and lists the conflicts, unless
  `PREFER SOURCE` or `PREFER TARGET` says which side wins. Merging the base into its
  branch moves the branch's point forward (what "rebase" did). Two namespaces with no
  common base can't be merged.
- Names are any namespace names; what a branch *means* (an agent's experiment, a
  release) is the caller's business.

## 6. PIPELINE

```sql
PIPELINE bid TIMEOUT 80ms
  RECALL FROM TOOLS WHERE task = "bidding" LIMIT 3
  | RECALL FROM SEMANTIC WHERE key = {url}
  | RECALL FROM EPISODIC WHERE url = {url} LIMIT 10
```

Stages run in order within the time budget; `{name}` uses a value from an earlier
stage. The last stage may be a STORE or UPDATE (what REFLECT … THEN did).

## 7. SUBSCRIBE, UNSUBSCRIBE, PUBLISH

```sql
PUBLISH     TOPIC "orders" (order_id = "o-1", status = "paid") TTL 7d NAMESPACE "shop"
SUBSCRIBE   TOPIC "orders" WHERE status = "paid" FROM NOW NAMESPACE "shop"
SUBSCRIBE   RECALL FROM SEMANTIC WHERE predicate = "owns" LIMIT 20 NAMESPACE "crm" DEBOUNCE 100ms
UNSUBSCRIBE "sub-7f3a"
```

A subscription to a RECALL is a live query (what WATCH was): its result as it changes.
Delivery and acknowledgements are the server's API, not AQL.

## 8. Modifiers

`NAMESPACE`, `IN SCOPE`, `LIMIT`, `ORDER BY … [ASC|DESC]`, `RETURN f, …`, `TTL`,
`MIN_CONFIDENCE`, `AGGREGATE COUNT(*) AS n, AVG(x) AS a`, `GROUP BY`, `HAVING`,
`WINDOW LAST n | LAST 30s | SINCE …` (anything with time), `WITH LINKS [TYPE …]`,
`FOLLOW LINKS [TYPE …] DEPTH n`, `AS OF <time>`, `VALID AT <time>`, `WITH HISTORY`,
`TIMEOUT` (PIPELINE only).

Conditions: `= != < <= > >= CONTAINS STARTS_WITH ENDS_WITH IN`, `AND`, `OR`, `( )`.

## 9. What changed from 0.x

| 0.x | 1.0 |
|---|---|
| LOOKUP, LOAD, SCAN | RECALL (§1) |
| `KEY x = v`, `ALL` | `WHERE x = v`, no WHERE |
| `PATTERN … THRESHOLD t` | `LIKE "text" MIN_CONFIDENCE t` |
| REFLECT … THEN | PIPELINE with a write last |
| `STORE INTO SEMANTIC FACT [MULTI]` | a payload with subject and predicate; `multi = true` |
| UPDATE LINK | LINK (create or update) |
| MERGE ENTITY | UPDATE aliases + FORGET (§4) |
| WATCH / UNWATCH | SUBSCRIBE RECALL … / UNSUBSCRIBE |
| ACK | the server's API |
| `SCOPE private\|shared\|cluster` | removed (NAMESPACE, IN SCOPE) |
| `BRANCH "b"` modifier | BRANCH / FORK / MERGE on namespaces (§5); a branch is a namespace |
| TIMEOUT on any statement | PIPELINE only |

## Open points

1. `VALID AT` (fact validity) could fold into `AS OF`; kept for now.
2. `FROM` / `INTO` could go (`STORE SEMANTIC (…)`); kept for now — reads naturally.
3. A namespace with branches being dropped: refuse, or drop its branches too? (Forks
   are unaffected: they hold their own copy.)
4. MERGE of a branch of a branch into main: through its parent only, or directly?
