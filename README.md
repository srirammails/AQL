# AQL — Agent Query Language

The language an agent uses to talk to its memory. AQL v0.13 is small: **13 verbs plus
`USE`**, each working on every memory type. The agent says what it wants; the engine
works out how.

```sql
RECALL FROM PROCEDURAL LIKE "pod OOMKilled restart" CONFIDENCE 0.7 LIMIT 3
```

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/AQL-v0.13-teal.svg)](spec/AQL_SPEC_v0.13.md)
[![Release](https://img.shields.io/badge/release-0.13.0-blue.svg)](CHANGELOG.md)
[![Tests](https://img.shields.io/badge/tests-406%20conformance-green.svg)](tests/)

---

## Why AQL?

**Agents need memory that persists, learns, and scales.**

| Traditional Approach | AQL Approach |
|---------------------|--------------|
| Retraining on new data | Write to memory, recall later |
| Hardcoded knowledge | Records and links the agent owns (`LINK`) |
| Context stuffing | Recall by key, by similarity, or the last 30 seconds |
| Manual tool selection | `RECALL FROM TOOLS` ranks tools for the task |
| Risky changes to memory | `BRANCH`, try it, `MERGE` |

**Core thesis:** Agent learning is the accumulation of relationships between memory records and tuned execution parameters—both queryable and writable through AQL.

---

## Try It Now

### Browser Playground

**[Launch AQL Playground](https://srirammails.github.io/AQL/playground/)** — Parse and execute AQL queries instantly in your browser. No installation required.

> Click an example, or write your own. Everything runs locally via WASM.
> The playground parses AQL v0.13 and runs the core: `RECALL`, `STORE`, `UPDATE`,
> `FORGET`, `LINK`, `UNLINK` and `PIPELINE` on one in-browser memory. What only a
> server does (`USE` scripts, namespaces and `BRANCH` / `FORK` / `MERGE`, history
> reads, `SUBSCRIBE` / `PUBLISH`) is refused with an error that says so.

### Rust Reference Implementation

```bash
cargo add clawdb
```

```rust
use clawdb::ClawDB;

let db = ClawDB::new_memory().await?;
db.execute("STORE INTO EPISODIC (event = \"user_login\", user_id = \"u123\") TTL 24h").await?;
let results = db.execute("RECALL FROM EPISODIC WHERE user_id = \"u123\" RETURN *").await?;
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          AQL Query                                   │
│   RECALL FROM SEMANTIC LIKE $question CONFIDENCE 0.7 LIMIT 5        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────────┐
│                         aql-parser                                    │
│  ┌────────────┐    ┌────────────┐    ┌────────────────────────────┐ │
│  │  grammar/  │───▶│   Parser   │───▶│  AST (Statement, Expr...)  │ │
│  │  aql.pest  │    │  (PEG)     │    │                            │ │
│  └────────────┘    └────────────┘    └────────────────────────────┘ │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
   ┌─────────────┐     ┌─────────────┐     ┌─────────────────┐
   │  clawdb     │     │   Your DB   │     │  clawdb-wasm    │
   │  (LanceDB)  │     │  (ADB, etc) │     │  (Browser)      │
   └─────────────┘     └─────────────┘     └─────────────────┘
```

### Memory Types

| Type | Purpose | Example |
|------|---------|---------|
| **WORKING** | Active context for current reasoning | Session state, live variables |
| **EPISODIC** | Timestamped experiences | "User asked about refunds at 2pm" |
| **SEMANTIC** | Domain knowledge and concepts | "Refund policy requires receipt" |
| **PROCEDURAL** | Executable patterns with confidence | Steps to process a refund (0.92 confidence) |
| **TOOLS** | Available capabilities | API endpoints, functions, services |
| **ALL** | Every type at once (reads and `FORGET`) | `RECALL FROM ALL WHERE url = "example.com"` |

Every statement says whose memory it touches with `NAMESPACE "<ns>"`, or takes it from
a `USE` in force. `IN SCOPE "<scope>"` narrows to a shared group inside the namespace.

### The 13 verbs (and USE)

```
Memory      RECALL  STORE  UPDATE  FORGET  LINK  UNLINK
Namespaces  BRANCH  FORK  MERGE
Flow        PIPELINE  SUBSCRIBE  UNSUBSCRIBE  PUBLISH
Session     USE
```

Reads say `FROM`, writes say `INTO`. `RECALL` is the only read: the engine picks the
access path from what you ask.

```sql
-- Read: one verb, many ways in
RECALL FROM SEMANTIC WHERE key = "person:ada"                                  -- by key
RECALL FROM SEMANTIC LIKE $question CONFIDENCE 0.7 LIMIT 5                     -- by similarity
RECALL FROM EPISODIC WHERE pod = "payments" ORDER BY created_at DESC LIMIT 10  -- filtered
RECALL FROM TOOLS WHERE task = "bidding" LIMIT 3                               -- tools for a task
RECALL FROM WORKING WINDOW LAST 30s                                            -- the session, newest first
RECALL FROM SEMANTIC WHERE key = "org:acme" FOLLOW LINKS TYPE "works_at" DEPTH 2
RECALL FROM EPISODIC WHERE outcome = "failure" AGGREGATE COUNT(*) AS n GROUP BY pod HAVING n > 5

-- Write
STORE  INTO EPISODIC (event = "login", user = "u123") TTL 7d
UPDATE INTO SEMANTIC WHERE key = "person:ada" (role = "CTO")         -- the old version stays as history
FORGET FROM WORKING WHERE stale = true

-- Link: relationships are links (LINK on an existing link sets its weight)
LINK   FROM SEMANTIC WHERE key = "person:ada" INTO SEMANTIC WHERE key = "org:acme" TYPE "works_at" WEIGHT 0.9
UNLINK FROM SEMANTIC WHERE key = "person:ada" INTO SEMANTIC WHERE key = "org:acme" TYPE "works_at"
```

### An agent's memory loop

A script is statements separated by `;`. `USE` names the namespace once; every later
statement without its own `NAMESPACE` works there. The answer is the last statement's.

```sql
USE "agent:support-42";
RECALL FROM SEMANTIC WHERE key = "customer:ada";
RECALL FROM PROCEDURAL LIKE $question CONFIDENCE 0.7 LIMIT 3;
STORE INTO EPISODIC (key = "ep:7781", event = "refund_asked", customer = "customer:ada", outcome = "approved") TTL 90d;
LINK FROM EPISODIC WHERE key = "ep:7781" INTO SEMANTIC WHERE key = "customer:ada" TYPE "about";
UPDATE INTO PROCEDURAL WHERE name = "refund_flow" (success_count = 13, confidence = 0.93)
```

A statement's own `NAMESPACE` wins over the `USE`:

```sql
USE "crm";
RECALL FROM SEMANTIC WHERE key = "person:ada";
STORE INTO EPISODIC (event = "called", who = "person:ada");
RECALL FROM EPISODIC WHERE pod = "payments" NAMESPACE "ops"
```

### Reading several namespaces

A `RECALL` (or `SUBSCRIBE RECALL`) may list namespaces; the records of all of them
are ranked together. Writes, links and topics are always in one namespace.

```sql
RECALL FROM SEMANTIC LIKE $question NAMESPACE ("crm", "kb") LIMIT 5
```

### Branch, try, merge

A namespace can be branched like code. A branch sees its base plus its own writes;
nothing is copied. `FORK` makes a full, independent copy.

```sql
BRANCH FROM "crm" INTO "crm-try";
UPDATE INTO SEMANTIC WHERE key = "person:ada" (role = "CTO") NAMESPACE "crm-try";
RECALL FROM SEMANTIC WHERE key = "person:ada" NAMESPACE "crm-try";
MERGE FROM "crm-try" INTO "crm";
FORGET FROM ALL NAMESPACE "crm-try"
```

```sql
MERGE  FROM "crm-try" INTO "crm" PREFER SOURCE        -- on conflict, the branch wins
MERGE  FROM "crm" INTO "crm-try"                      -- bring crm's newer changes into the branch
BRANCH FROM "crm" INTO "crm-before-import" AS OF "2026-11-01T09:00:00Z"
FORK   FROM "crm" INTO "crm-nov-1" AS OF "2026-11-01T00:00:00Z"
```

### One thing, two names

When two keys turn out to name the same thing, give the survivor an alias and forget
the duplicate. Reads and links by the alias reach the survivor; nothing is rewritten.

```sql
UPDATE INTO SEMANTIC WHERE key = "person:ada" (aliases = ["person:ada-lovelace"]) NAMESPACE "crm"
FORGET FROM SEMANTIC WHERE key = "person:ada-lovelace" NAMESPACE "crm"
```

### History

```sql
RECALL FROM SEMANTIC WHERE key = "person:ada" AS OF "2026-11-01T09:00:00Z"
RECALL FROM SEMANTIC WHERE key = "person:ada"
       WITH HISTORY BETWEEN "2026-10-01T00:00:00Z" AND "2026-11-01T00:00:00Z"
```

### Pipelines

Stages run in order within a time budget; `{name}` uses a value from an earlier stage.
The last stage may write.

```sql
PIPELINE bid TIMEOUT 80ms
  RECALL FROM TOOLS WHERE task = "bidding" LIMIT 3
  | RECALL FROM SEMANTIC WHERE key = {url}
  | RECALL FROM EPISODIC WHERE url = {url} ORDER BY created_at DESC LIMIT 10
  | STORE INTO EPISODIC (url = {url}, decision = "bid")
```

### Live: topics and live queries

```sql
PUBLISH   TOPIC "orders" (order_id = "o-1", status = "paid") TTL 7d NAMESPACE "shop"
SUBSCRIBE TOPIC "orders" WHERE status = "paid" NAMESPACE "shop"
SUBSCRIBE RECALL FROM SEMANTIC WHERE kind = "customer" ORDER BY updated_at DESC LIMIT 20 NAMESPACE ("crm", "kb")
UNSUBSCRIBE "sub-7f3a"
```

`SUBSCRIBE RECALL` is a live query: its result, as it changes.

---

## Versions

AQL has one version number: language **v0.N** is released as **0.N.x** (the crates
here, tagged `v0.N.x`). The current version is **v0.13 (release 0.13.0)**, specified
in one document: [spec/AQL_SPEC_v0.13.md](spec/AQL_SPEC_v0.13.md).

v0.13 replaces v0.5 and the v0.6–v0.12 additions with one smaller language. It is
not compatible with them: statements change form (`LOOKUP` / `LOAD` / `SCAN` are
`RECALL`, `MERGE ENTITY` is an alias, `WATCH` is `SUBSCRIBE RECALL`, …). The spec's
§9 maps every old form to the new one.

| Version | What it is | Spec |
|---|---|---|
| v0.13 | One language: 13 verbs + `USE`, `RECALL` the only read, `BRANCH` / `FORK` / `MERGE` on namespaces, aliases, `NAMESPACE (…)` lists on reads | [specification](spec/AQL_SPEC_v0.13.md) |
| v0.6–v0.12 | Subscriptions, link updates, graph reads, facts, history, memory branches, namespaces on every statement (superseded) | [additions](spec/) |
| v0.5 | The first base: `FROM` / `INTO`, five memory types, links, pipelines, aggregates (superseded) | [specification](spec/AQL_SPEC_v0.5.md) |

The full history is in [CHANGELOG.md](CHANGELOG.md). In code,
`aql_parser::LANGUAGE_VERSION` is `"v0.13"` and `aql_parser::VERSION` is `"0.13.0"`.

---

## Engine Architecture

This repo holds the AQL specification and reference implementations. How the
production engine (ADB) stores and serves AQL — partitions, replicated log, Lance on
object storage, embedded / single-node / cluster profiles — is documented in the ADB
repository: `ADB/.ai/ARCHITECTURE.md`.

## Project Structure

```
AQL/
├── grammar/
│   └── aql.pest              # PEG grammar (copy of crates/aql-parser/src/grammar/aql.pest)
├── crates/
│   ├── aql-parser/           # Parser + AST (reusable)
│   ├── clawdb/               # Reference implementation (LanceDB)
│   └── clawdb-wasm/          # Browser WASM build
├── playground/
│   ├── index.html            # Browser playground
│   └── pkg/                  # WASM bindings
├── spec/
│   ├── AQL_SPEC_v0.13.md     # The language (current)
│   └── AQL_SPEC_v0.2.md … AQL_SPEC_v0.12_additions.md  # Earlier versions (superseded)
├── tests/
│   ├── fixtures/seed.aql     # Canonical test data
│   ├── suites/*.yaml         # 34 suites, 406 conformance tests
│   └── coverage/             # Coverage matrix
└── CHANGELOG.md              # Versions and releases
```

---

## Test Suite

The conformance test suite ensures any AQL implementation matches the specification.

Every suite is written in v0.13. Suites marked `requires: server` need a server (ADB).

```
Coverage Summary
──────────────────────────────────────────────────
Core: read, write, link, pipeline, aggregate  116 tests  (suites 01-17, 21)
Live queries, links, graph, groups, history   176 tests  (suites 18-20, 22-29)
Namespaces and aliases                         36 tests  (suites 30-32)
BRANCH / FORK / MERGE                          65 tests  (suite 33)
USE, scripts, NAMESPACE lists                  13 tests  (suite 34)
──────────────────────────────────────────────────
Total                                         406 tests  in 34 suites
```

### Running Tests

Tests are YAML files describing queries and expected outcomes:

```yaml
- id: T02-01
  name: "RECALL WHERE = string"
  query: |
    RECALL FROM EPISODIC WHERE bid_id = "e-001" RETURN bid_id
  expect:
    success: true
    count: 1
    contains:
      - bid_id: "e-001"
```

Load `tests/fixtures/seed.aql` first, then validate each test suite against your implementation.
The runner does both:

```bash
cd tests && npm install
node run-tests.mjs                              # the WASM build (suites a server needs are skipped)
node run-tests.mjs --url http://localhost:3000  # any AQL server's POST /query (ADB, ClawDB)
```

---

## Crates

### aql-parser

The parser crate is independent and can be used with any backend.

```toml
[dependencies]
aql-parser = { git = "https://github.com/srirammails/AQL", tag = "v0.13.0" }
```

```rust
use aql_parser::{parse, parse_script, Statement};

let stmt = parse("RECALL FROM EPISODIC WHERE x = 1 RETURN x")?;
match stmt {
    Statement::Recall(r) => {
        // r.memory_type, r.predicate, r.modifiers: build your own execution plan
    }
    _ => {}
}

// A script: USE gives the later statements that name no namespace that one.
// The second argument is a session's namespace in force before the script.
let stmts = parse_script("USE \"crm\"; RECALL FROM SEMANTIC WHERE key = \"person:ada\"", None)?;
assert_eq!(aql_parser::LANGUAGE_VERSION, "v0.13");
```

### clawdb

Single-node reference implementation using LanceDB for vector storage.

```toml
[dependencies]
clawdb = { git = "https://github.com/srirammails/AQL", tag = "v0.13.0" }
```

```rust
use clawdb::{ClawDB, ClawConfig};

// In-memory (testing)
let db = ClawDB::new_memory().await?;

// Persistent (production)
let db = ClawDB::new("./agent_memory").await?;

// Execute AQL
let results = db.execute("RECALL FROM SEMANTIC WHERE topic = \"auth\" RETURN *").await?;
```

### clawdb-wasm

WASM build for browser-based parsing and validation.

```javascript
import init, { parse, execute } from './pkg/clawdb_wasm.js';

await init();
console.log(parse("RECALL FROM EPISODIC WHERE x = 1 RETURN x"));   // the AST as JSON
console.log(execute("STORE INTO EPISODIC (event = \"login\")"));    // a QueryResult as JSON
```

---

## Building

Requires stable Rust and `protoc` (the Protocol Buffers compiler — Lance's build
needs it; `apt install protobuf-compiler`, or set `PROTOC` to its path).

```bash
# Check all crates
cargo check

# Build release
cargo build --release

# Build WASM
cd crates/clawdb-wasm
wasm-pack build --target web --out-dir ../../playground/pkg
```

---

## Specification

The language is [`spec/AQL_SPEC_v0.13.md`](spec/AQL_SPEC_v0.13.md); the grammar is
[`grammar/aql.pest`](grammar/aql.pest). Key design principles:

1. **Intent, not implementation** — the agent says what it wants; the engine picks how
2. **Few verbs, every memory type** — 13 verbs, each on EPISODIC, SEMANTIC, PROCEDURAL, TOOLS, WORKING (and ALL for reads and FORGET)
3. **FROM and INTO express direction** — reads say FROM, writes say INTO
4. **Ontology is dynamic and agent-owned** — LINK TYPE is arbitrary; relationships are links, values are fields
5. **Namespaces are like code** — branch, fork, merge
6. **Pipelines are first-class** — time-bounded chains for real-time decisions

---

## Use Cases

**Real-Time Bidding** — Agent evaluates bid requests in < 80ms using a pipeline across semantic, episodic, and procedural memory.

**Kubernetes Log Analysis** — Agent matches log events to known runbooks, checks incident history, acts, and learns from outcomes.

**Compliance and Audit** — Every agent decision is stored with full context — what the agent knew, what procedure it followed, what outcome resulted.

---

## License

Apache 2.0 — see [LICENSE](LICENSE)

---

## Contributing

1. Read `spec/AQL_SPEC_v0.13.md`
2. A language change is a new version: the spec, a
   conformance suite, the grammar in `crates/aql-parser/src/grammar/aql.pest` (copied
   to `grammar/aql.pest`), the workspace version `0.N.0`, `LANGUAGE_VERSION`, and
   `CHANGELOG.md`
3. Run the parser tests and the conformance suites (against a server with `--url`)
4. Ensure coverage matrix stays complete

---

*AQL v0.13 · release 0.13.0 · November 2026 · Formal grammar with design rationale*

*Sriram Reddy · The query language every AI agent has been missing.*
