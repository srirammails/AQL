# AQL — Agent Query Language

A declarative query language for agent memory systems. AQL enables LLM agents to store, retrieve, link, and learn from their experiences across five memory types.

```sql
RECALL FROM EPISODIC WHERE outcome = "success" ORDER BY confidence DESC LIMIT 5 RETURN *
```

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/AQL%20spec-v0.5-teal.svg)](spec/AQL_SPEC_v0.5.md)
[![Tests](https://img.shields.io/badge/tests-150%20conformance-green.svg)](tests/)

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
      FOLLOW LINKS TYPE "implements" INTO PROCEDURAL
      RETURN pattern, confidence

-- Aggregation
RECALL FROM EPISODIC WHERE outcome = "success"
      AGGREGATE COUNT(*) AS total, AVG(confidence) AS avg_conf
      HAVING avg_conf > 0.8
      RETURN total, avg_conf

-- Pipelines (Time-Bounded)
PIPELINE rtb_decision TIMEOUT 50ms
    RECALL FROM SEMANTIC WHERE category = $category LIMIT 10
    THEN FOLLOW LINKS TYPE "triggers" INTO PROCEDURAL
    THEN AGGREGATE MAX(confidence) AS best
    RETURN best
```

---

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
│   └── AQL_SPEC_v0.5.md      # Formal specification
└── tests/
    ├── fixtures/seed.aql     # Canonical test data
    ├── suites/*.yaml         # 150 conformance tests
    └── coverage/             # Coverage matrix
```

---

## Test Suite

The conformance test suite ensures any AQL implementation matches the specification.

```
Coverage Summary
──────────────────────────────────────────
Grammar productions     ~93%   (T01-T21)
Semantic validation     100%   (T17-01 to T17-08)
Bug regressions         29/29  (All documented bugs)
Edge cases              ~90%   (T21-01 to T21-10)
──────────────────────────────────────────
Total                   150 tests
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

---

## Crates

### aql-parser

The parser crate is independent and can be used with any backend.

```toml
[dependencies]
aql-parser = { git = "https://github.com/yourorg/aql" }
```

```rust
use aql_parser::{parse_query, Statement};

let stmt = parse_query("RECALL FROM EPISODIC WHERE x = 1 RETURN x")?;
match stmt {
    Statement::Recall { memory_type, conditions, .. } => {
        // Build your own execution plan
    }
    _ => {}
}
```

### clawdb

Single-node reference implementation using LanceDB for vector storage.

```toml
[dependencies]
clawdb = { git = "https://github.com/yourorg/aql" }
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
import init, { parse_and_validate, execute_query } from './pkg/clawdb_wasm.js';

await init();
const result = parse_and_validate("RECALL FROM EPISODIC WHERE x = 1 RETURN x");
console.log(result);
```

---

## Building

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

The formal specification lives in `spec/AQL_SPEC_v0.5.md`. Key design principles:

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

1. Read `spec/AQL_SPEC_v0.5.md`
2. Run the test suite against your changes
3. Ensure coverage matrix stays complete

---

*AQL v0.5 · April 2026 · Formal grammar with design rationale*

*Sriram Reddy · The query language every AI agent has been missing.*
