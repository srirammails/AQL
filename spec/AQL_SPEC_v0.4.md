# AQL — Agent Query Language
## Formal Grammar Specification v0.4

---

## 1. Design Principles

1. **AQL expresses intent, not implementation.** ADB handles storage, decay, conflict resolution.
2. **Readable without a spec.** If a non-expert can't understand a query, cut the feature.
3. **Every statement is scoped to a memory type.** Episodic, Semantic, Procedural, Working, or Tools.
4. **Working memory is the assembly layer.** The prepared context package for the next reasoning step.
5. **Tool Registry is a first-class memory type.** Queryable like any other memory.
6. **Pipelines are first-class.** Chain queries with a time budget.
7. **Multi-agent support is simple.** SCOPE and NAMESPACE — nothing more.

---

## 2. Top-Level Grammar (BNF)

```bnf
program         ::= statement+

statement       ::= read_stmt
                  | write_stmt
                  | forget_stmt
                  | pipeline_stmt
                  | reflect_stmt
```

---

## 3. Read Statements

```bnf
read_stmt       ::= verb memory_type? predicate modifier*

verb            ::= "LOOKUP"
                  | "RECALL"
                  | "SCAN"
                  | "LOAD"

memory_type     ::= "EPISODIC"
                  | "SEMANTIC"
                  | "PROCEDURAL"
                  | "WORKING"
                  | "TOOLS"

predicate       ::= exact_pred
                  | similarity_pred
                  | scan_pred

exact_pred      ::= "KEY" key_expr
                  | "WHERE" condition

key_expr        ::= identifier "=" value

similarity_pred ::= "LIKE" expression
                  | "PATTERN" expression

scan_pred       ::= "ALL"
                  | "WHERE" condition

condition       ::= field comparator value
                  | condition "AND" condition
                  | condition "OR" condition
                  | "(" condition ")"

comparator      ::= "=" | "!=" | ">" | "<" | ">=" | "<="
```

### Read Verb Semantics

| Verb | Memory Types | What it does |
|------|-------------|--------------|
| LOOKUP | SEMANTIC, PROCEDURAL, TOOLS | Find by exact key or pattern |
| RECALL | EPISODIC, SEMANTIC | Find by similarity or context |
| SCAN | WORKING | Get current assembled context |
| LOAD | TOOLS | Select tools into working memory |

---

## 4. Write Statements

```bnf
write_stmt      ::= store_stmt
                  | update_stmt
                  | link_stmt

store_stmt      ::= "STORE" memory_type payload
                    scope_mod?
                    namespace_mod?
                    ttl_mod?

update_stmt     ::= "UPDATE" memory_type "WHERE" condition payload

link_stmt       ::= "LINK" memory_type key_expr
                    "TO" memory_type key_expr

payload         ::= "(" field_value_list ")"

field_value_list ::= field_value ("," field_value)*
field_value     ::= field "=" value

scope_mod       ::= "SCOPE" scope_value
scope_value     ::= "private" | "shared" | "cluster"

namespace_mod   ::= "NAMESPACE" string_literal

ttl_mod         ::= "TTL" duration
```

### Write Verb Semantics

| Verb | What it does |
|------|--------------|
| STORE | Write new memory |
| UPDATE | Modify existing memory |
| LINK | Connect two memories |

---

## 5. Forget Statement

```bnf
forget_stmt     ::= "FORGET" memory_type predicate
```

That's it. No decay parameters. No strategy. ADB decides how to forget.

---

## 6. Modifiers

```bnf
modifier        ::= return_mod
                  | limit_mod
                  | weight_mod
                  | threshold_mod
                  | timeout_mod
                  | order_mod
                  | confidence_mod
                  | source_mod

return_mod      ::= "RETURN" field_list
field_list      ::= field ("," field)*

limit_mod       ::= "LIMIT" integer

weight_mod      ::= "WEIGHT" field

threshold_mod   ::= "THRESHOLD" float

timeout_mod     ::= "TIMEOUT" duration

order_mod       ::= "ORDER" "BY" field ("ASC" | "DESC")?

confidence_mod  ::= "MIN_CONFIDENCE" float

source_mod      ::= "FROM" identifier_list
identifier_list ::= identifier ("," identifier)*

duration        ::= integer time_unit
time_unit       ::= "ms" | "s" | "m" | "h" | "d"
```

---

## 7. Reflect Statement

REFLECT assembles context from multiple memory types. ADB handles consistency internally.

```bnf
reflect_stmt    ::= "REFLECT" context_expr
                    reflect_source+
                    modifier*
                    then_clause?

context_expr    ::= key_expr | identifier

reflect_source  ::= "INCLUDE" memory_type predicate?

then_clause     ::= "THEN" write_stmt
```

---

## 8. Pipeline Statement

```bnf
pipeline_stmt   ::= "PIPELINE" identifier?
                    "TIMEOUT" duration
                    pipeline_stage+

pipeline_stage  ::= (read_stmt | reflect_stmt) "|"?
```

---

## 9. Primitives

```bnf
identifier      ::= [a-zA-Z_][a-zA-Z0-9_]*

field           ::= identifier ("." identifier)?

value           ::= string_literal
                  | integer
                  | float
                  | boolean
                  | embedding_ref
                  | variable

string_literal  ::= '"' [^"]* '"'

integer         ::= [0-9]+

float           ::= [0-9]+ "." [0-9]+

boolean         ::= "true" | "false"

embedding_ref   ::= "$" identifier

variable        ::= "{" identifier "}"
```

---

## 10. Memory Types

```
Memory Type  │ What it stores              │ Latency
─────────────┼─────────────────────────────┼─────────
WORKING      │ Current task context        │ < 1ms
TOOLS        │ Available tools + rankings  │ < 2ms
PROCEDURAL   │ How-to knowledge, runbooks  │ < 5ms
SEMANTIC     │ Facts, concepts, entities   │ < 20ms
EPISODIC     │ Past events, history        │ < 50ms
```

---

## 11. Example Queries

### Tool Selection
```sql
-- Load relevant tools
LOAD TOOLS WHERE relevance > 0.8
  ORDER BY ranking DESC
  LIMIT 3
  RETURN tool_id, schema

-- What tools am I using?
SCAN WORKING ALL
  RETURN active_tools
```

### Multi-Agent Memory
```sql
-- Store shared knowledge
STORE SEMANTIC (
  concept = "k8s_oom_pattern",
  knowledge = "payments-api OOMs every Friday"
)
  SCOPE shared
  NAMESPACE "platform-agents"

-- Store private episode
STORE EPISODIC (
  incident_id = "inc-001",
  action = "scaled memory",
  resolved = true
)
  SCOPE private
```

### Quality-Filtered Recall
```sql
-- Only high confidence memories
RECALL EPISODIC WHERE pod = "payments-api"
  MIN_CONFIDENCE 0.7
  ORDER BY time DESC
  LIMIT 5
  RETURN incident_id, action, resolved

-- Similar concepts
RECALL SEMANTIC LIKE $current_context
  MIN_CONFIDENCE 0.8
  LIMIT 10
```

### Forgetting
```sql
-- Forget old episodes
FORGET EPISODIC WHERE last_accessed > 30d

-- Clear working memory
FORGET WORKING WHERE task_id = "completed-001"
```

### Pattern Matching
```sql
-- Match log to known pattern
LOOKUP PROCEDURAL PATTERN $log_event
  THRESHOLD 0.85
  RETURN pattern_id, severity, action_steps

-- Get actions for pattern
LOOKUP PROCEDURAL WHERE pattern_id = {matched}
  RETURN steps, priority
```

### Context Assembly
```sql
-- Assemble context for decision
REFLECT incident_id = {current}
  INCLUDE EPISODIC WHERE incident_id = {current}
  INCLUDE PROCEDURAL WHERE pattern_id = {matched}
  INCLUDE WORKING

-- Reflect and learn
REFLECT url = {url}
  INCLUDE SEMANTIC
  INCLUDE EPISODIC
  THEN STORE PROCEDURAL (
    pattern = "learned_pattern",
    steps = ["step1", "step2"]
  )
```

### Full Pipelines
```sql
-- RTB pipeline
PIPELINE bid_decision TIMEOUT 80ms
  LOAD TOOLS WHERE task = "bidding" LIMIT 3
  | LOOKUP SEMANTIC KEY url = {url}
  | RECALL EPISODIC WHERE url = {url} LIMIT 10
  | REFLECT url = {url}
      INCLUDE SEMANTIC
      INCLUDE EPISODIC

-- Incident response
PIPELINE incident TIMEOUT 200ms
  LOOKUP PROCEDURAL PATTERN $log_event
      THRESHOLD 0.85
  | RECALL EPISODIC WHERE pattern_id = {matched}
      MIN_CONFIDENCE 0.7
      LIMIT 5
  | LOAD TOOLS WHERE category = "kubernetes"
  | REFLECT incident_id = {current}
      INCLUDE EPISODIC
      INCLUDE PROCEDURAL
```

---

## 12. What AQL Does NOT Do

| Concern | Owner |
|---------|-------|
| Deciding next action | LLM |
| Executing actions | Agent Runtime |
| Decay algorithms | ADB |
| Storage tiers | ADB |
| Conflict resolution | ADB |
| Compression | ADB |
| Concurrency control | ADB |

**The rule:** If it's about *how* memory works, it's ADB's job. AQL only expresses *what* the agent wants.

---

## 13. Changes from v0.3

### Removed (implementation details)
- `DECAY lambda=... offset=...` — ADB handles decay
- `STRATEGY soft_delete|compress|hard_delete` — ADB decides
- `TIER hot|warm|cold` — ADB manages storage placement
- `CHECK temporal|factual|logical` — ADB validates internally
- `RESOLVE CONFLICTS = ...` — ADB handles conflicts
- `STRENGTH` — ADB computes importance
- `VERSION` / `LOCK` — ADB handles concurrency
- `IF NOT EXISTS` — ADB can be configured for upsert

### Simplified
- `FORGET` — just the predicate, no parameters
- `NAMESPACE` — just a string, no key=value syntax

### Kept (express intent)
- `LOAD TOOLS` — new retrieval verb
- `SCOPE private|shared|cluster` — multi-agent isolation
- `NAMESPACE "..."` — agent identity
- `MIN_CONFIDENCE` — quality filter

---

## 14. Grammar Size

| Version | Rules | Modifiers | Complexity |
|---------|-------|-----------|------------|
| v0.1 | 25 | 10 | Baseline |
| v0.2 | 28 | 12 | +12% |
| v0.3 | 45 | 22 | +80% |
| v0.4 | 30 | 8 | +20% |

v0.4 is only 20% larger than v0.1, with full multi-agent and tool support.

---

## 15. The Readability Test

Every query should pass: *Can a non-expert read this and know what it does?*

```sql
LOAD TOOLS WHERE relevance > 0.8 LIMIT 3
```
✓ Load 3 tools where relevance is above 0.8

```sql
RECALL EPISODIC WHERE pod = "payments" MIN_CONFIDENCE 0.7
```
✓ Recall episodes about payments with at least 70% confidence

```sql
FORGET EPISODIC WHERE last_accessed > 30d
```
✓ Forget episodes not accessed in 30 days

```sql
STORE SEMANTIC (fact = "servers crash on Fridays")
  SCOPE shared
  NAMESPACE "ops-team"
```
✓ Store a shared fact for the ops-team

---

*AQL v0.4 — Simplified Specification*
*March 2026 · Sriram Reddy*
*github.com/srirammails/AQL*
