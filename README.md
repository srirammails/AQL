# AQL — Agent Query Language

A declarative query language for agent memory systems. AQL enables LLM agents to store, retrieve, link, and learn from their experiences across five memory types.

```sql
RECALL FROM EPISODIC WHERE outcome = "success" ORDER BY confidence DESC LIMIT 5 RETURN *
```

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/AQL-v0.13-teal.svg)](CHANGELOG.md)
[![Release](https://img.shields.io/badge/release-0.13.0-blue.svg)](CHANGELOG.md)
[![Tests](https://img.shields.io/badge/tests-334%20conformance-green.svg)](tests/)

---

## Why AQL?

**Agents need memory that persists, learns, and scales.**

| Traditional Approach | AQL Approach |
|---------------------|--------------|
| Retraining on new data | Write to memory, query later |
| Hardcoded knowledge | Dynamic ontology via LINK |
| Context stuffing | Windowed working memory |
| Manual tool selection | Queryable tool registry |

**Core thesis:** Agent learning is the accumulation of relationships between memory records and tuned execution parameters—both queryable and writable through AQL.

---

## Try It Now

### Browser Playground

**[Launch AQL Playground](https://srirammails.github.io/AQL/playground/)** — Parse and execute AQL queries instantly in your browser. No installation required.

> Click example queries to execute them, or write your own. All operations run locally via WASM.
> The playground parses AQL v0.13 and executes the v0.5 core (and v0.7 `UPDATE LINK` /
> `UNLINK`); what only a server implements (subscriptions, `WATCH`, history reads,
> `MERGE ENTITY`, `BRANCH`, …) is refused with an error naming its version.

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
│   RECALL FROM EPISODIC WHERE confidence > 0.8 LIMIT 10 RETURN *     │
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

### Operations

```sql
-- Read Operations (FROM)
SCAN FROM WORKING WINDOW LAST 10            -- Recent context
RECALL FROM EPISODIC WHERE user = "alice"   -- Query experiences
LOOKUP FROM SEMANTIC KEY concept = "auth"   -- Direct key lookup
LOAD FROM TOOLS WHERE category = "api"      -- Load tools

-- Write Operations (INTO)
STORE INTO EPISODIC (event = "login") TTL 7d
UPDATE INTO PROCEDURAL WHERE pattern = "checkout" (confidence = 0.95)
FORGET FROM WORKING WHERE stale = true

-- Linking (Dynamic Ontology)
LINK FROM SEMANTIC WHERE concept = "auth"
     TO PROCEDURAL WHERE pattern = "login_flow"
     TYPE "implements"

-- Traversal
RECALL FROM SEMANTIC WHERE concept = "payments"
      FOLLOW LINKS TYPE "implements"
      RETURN pattern, confidence

-- Aggregation
RECALL FROM EPISODIC WHERE outcome = "success"
      AGGREGATE COUNT(*) AS total, AVG(confidence) AS avg_conf
      HAVING avg_conf > 0.8
      RETURN total, avg_conf

-- Pipelines (Time-Bounded)
PIPELINE rtb_decision TIMEOUT 50ms
    RECALL FROM SEMANTIC WHERE category = $category FOLLOW LINKS TYPE "triggers" LIMIT 10
    | RECALL FROM PROCEDURAL WHERE confidence > 0.5 AGGREGATE MAX(confidence) AS best

-- Similarity ($q is text or a vector, passed as a variable)
RECALL FROM SEMANTIC LIKE $q MIN_CONFIDENCE 0.7 LIMIT 5

-- Subscriptions and continuous queries (v0.6)
SUBSCRIBE TOPIC "orders" WHERE status = "paid" FROM NOW
PUBLISH TOPIC "orders" (order_id = "o-1", status = "paid") TTL 7d
WATCH RECALL FROM SEMANTIC IN SCOPE "agent:42" WHERE predicate = "owns" DEBOUNCE 100ms

-- Facts and history (v0.8, v0.10)
STORE INTO SEMANTIC FACT (subject = "person:ada", predicate = "works_at", object = "org:acme")
RECALL FROM SEMANTIC WHERE subject = "person:ada" AS OF 7d AGO

-- Memory spaces and branches (v0.11, v0.12)
RECALL FROM SEMANTIC ALL NAMESPACE "crm" BRANCH "try-new-recall"
LINK FROM SEMANTIC WHERE key = "a" TO EPISODIC WHERE key = "b" TYPE "rel" NAMESPACE "crm"
```

---

## Versions

AQL has one version number: language **v0.N** is released as **0.N.x** (the crates
here, tagged `v0.N.x`). The current version is **v0.13 (release 0.13.0)**. The base
specification is [v0.5](spec/AQL_SPEC_v0.5.md); each later version adds to it, and
every older statement parses as before.

| Version | Adds | Spec |
|---|---|---|
| v0.12 | `NAMESPACE` on `LINK` / `UPDATE LINK` / `UNLINK` and the topic statements | [additions](spec/AQL_SPEC_v0.12_additions.md) |
| v0.13 | `NAMESPACE` on `MERGE ENTITY` | [additions](spec/AQL_SPEC_v0.13_additions.md) |
| v0.11 | `BRANCH "<name>"`: memory branches | [additions](spec/AQL_SPEC_v0.11_additions.md) |
| v0.10 | fact validity, `AS OF` / `VALID AT` / `WITH HISTORY`, `MERGE ENTITY`, keys per scope | [additions](spec/AQL_SPEC_v0.10_additions.md) |
| v0.9 | `GROUP BY`, `WINDOW` on episodic `RECALL`, `UPDATE field = null` | [additions](spec/AQL_SPEC_v0.9_additions.md) |
| v0.8 | multi-hop and directed `FOLLOW LINKS`, `MIN_WEIGHT`, `STORE … FACT` | [additions](spec/AQL_SPEC_v0.8_additions.md) |
| v0.7 | `UPDATE LINK`, `UNLINK` | [additions](spec/AQL_SPEC_v0.7_additions.md) |
| v0.6 | `SUBSCRIBE` / `PUBLISH` / `ACK`, `WATCH`, `IN SCOPE` | [additions](spec/AQL_SPEC_v0.6_additions.md) |
| v0.5 | the base: `FROM` / `INTO`, five memory types, links, pipelines, aggregates | [specification](spec/AQL_SPEC_v0.5.md) |

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
│   └── aql.pest              # PEG grammar (source of truth)
├── crates/
│   ├── aql-parser/           # Parser + AST (reusable)
│   ├── clawdb/               # Reference implementation (LanceDB)
│   └── clawdb-wasm/          # Browser WASM build
├── playground/
│   ├── index.html            # Browser playground
│   └── pkg/                  # WASM bindings
├── spec/
│   ├── AQL_SPEC_v0.5.md      # The base specification
│   └── AQL_SPEC_v0.6_additions.md … AQL_SPEC_v0.13_additions.md  # Each later version
├── tests/
│   ├── fixtures/seed.aql     # Canonical test data
│   ├── suites/*.yaml         # 32 suites, 334 conformance tests
│   └── coverage/             # Coverage matrix
└── CHANGELOG.md              # Versions and releases
```

---

## Test Suite

The conformance test suite ensures any AQL implementation matches the specification.

```
Coverage Summary
──────────────────────────────────────────
v0.5 core               116 tests   (suites 01-17, 21)
v0.6-v0.10 additions    177 tests   (suites 18-20, 22-29)
v0.11 BRANCH              6 tests   (suite 30)
v0.12 NAMESPACE          22 tests   (suite 31)
v0.13 MERGE NAMESPACE    13 tests   (suite 32)
──────────────────────────────────────────
Total                   334 tests   (ADB: 334/334)
```

### Running Tests

Tests are YAML files describing queries and expected outcomes:

```yaml
- id: T02-01
  name: "WHERE = string"
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
use aql_parser::{parse, Statement};

let stmt = parse("RECALL FROM EPISODIC WHERE x = 1 RETURN x")?;
match stmt {
    Statement::Recall(r) => {
        // r.memory_type, r.predicate, r.modifiers: build your own execution plan
    }
    _ => {}
}
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

The base specification is `spec/AQL_SPEC_v0.5.md`; versions v0.6 to v0.13 add to it
(`spec/AQL_SPEC_v0.N_additions.md`, see [Versions](#versions)). Key design principles:

1. **FROM and INTO express direction explicitly** — Read uses FROM, write uses INTO
2. **AQL expresses intent, not implementation** — Storage details are backend concerns
3. **Every statement is scoped to a memory type** — EPISODIC, SEMANTIC, PROCEDURAL, WORKING, TOOLS, or ALL
4. **Ontology is dynamic and agent-owned** — LINK TYPE is arbitrary, meaning emerges from use
5. **Pipelines are first-class** — Time-bounded query chains for real-time decisions

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

1. Read `spec/AQL_SPEC_v0.5.md` and the additions up to the current version
2. A language change is a new version: a `spec/AQL_SPEC_v0.N_additions.md`, a
   conformance suite, the grammar in `crates/aql-parser/src/grammar/aql.pest` (copied
   to `grammar/aql.pest`), the workspace version `0.N.0`, `LANGUAGE_VERSION`, and
   `CHANGELOG.md`
3. Run the parser tests and the conformance suites (against a server with `--url`)
4. Ensure coverage matrix stays complete

---

*AQL v0.13 · release 0.13.0 · November 2026 · Formal grammar with design rationale*

*Sriram Reddy · The query language every AI agent has been missing.*
