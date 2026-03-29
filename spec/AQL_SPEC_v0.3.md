# AQL — Agent Query Language
## Formal Grammar Specification v0.3

---

## 1. Design Principles

1. **AQL is a retrieval and storage language, not a decision language.** It populates agent context. The LLM reasons over that context.
2. **Every statement is scoped to a memory type.** Episodic, Semantic, Procedural, Working, or Tools.
3. **Working memory is the assembly layer.** The prepared context package — everything the agent needs for the next reasoning step, pre-loaded and ready. Sub-millisecond. Never blocks.
4. **Tool Registry is a first-class memory type.** Available tools, rankings, and token costs are queryable like any other memory.
5. **Retrieval mode is explicit.** The verb encodes intent, not just predicate.
6. **Pipelines are first-class.** Agentic queries chain across memory types with a time budget.
7. **Multi-agent support is native.** Namespace and scope isolate or share memory across agents.
8. **Forgetting is explicit.** Decay models and strategies control memory lifecycle.

---

## 2. Top-Level Grammar (BNF)

```bnf
program         ::= statement+

statement       ::= read_stmt
                  | write_stmt
                  | forget_stmt
                  | tool_stmt
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
                  | "NOT" condition

comparator      ::= "=" | "!=" | ">" | "<" | ">=" | "<=" | "~" | "IN"
```

### Read Verb Semantics

| Verb | Memory Types | Retrieval Mode |
|------|-------------|----------------|
| LOOKUP | SEMANTIC, PROCEDURAL, TOOLS | Exact key or pattern match |
| RECALL | EPISODIC, SEMANTIC | Similarity / context match |
| SCAN | WORKING | Full scan of assembled context |

---

## 4. Write Statements

```bnf
write_stmt      ::= store_stmt
                  | update_stmt
                  | link_stmt

store_stmt      ::= "STORE" memory_type payload
                    strength_mod?
                    ttl_mod?
                    scope_mod?
                    namespace_mod?
                    condition_mod?
                    modifier*

update_stmt     ::= "UPDATE" memory_type "WHERE" condition payload
                    concurrency_mod?
                    namespace_mod?

link_stmt       ::= "LINK" memory_type key_expr
                    "TO" memory_type key_expr
                    weight_mod?

payload         ::= "(" field_value_list ")"
                  | "FROM" identifier
                  | "FROM" "THIS_EPISODE"

field_value_list ::= field_value ("," field_value)*
field_value     ::= field "=" value

strength_mod    ::= "STRENGTH" float

ttl_mod         ::= "TTL" duration

scope_mod       ::= "SCOPE" scope_value
scope_value     ::= "private" | "shared" | "cluster"

namespace_mod   ::= "NAMESPACE" namespace_expr
namespace_expr  ::= identifier "=" string_literal

condition_mod   ::= "IF" "NOT" "EXISTS"

concurrency_mod ::= "VERSION" integer
                  | "LOCK" lock_mode
lock_mode       ::= "optimistic" | "pessimistic"
```

### Write Verb Semantics

| Verb | Purpose |
|------|---------|
| STORE | Write new memory into a memory type |
| UPDATE | Modify existing memory entry with optional concurrency control |
| LINK | Create relationship between two memory entries |

---

## 5. Forget Statements

```bnf
forget_stmt     ::= "FORGET" memory_type predicate
                    decay_clause?
                    strategy_clause?
                    namespace_mod?

decay_clause    ::= "DECAY" decay_params
decay_params    ::= "lambda" "=" float "offset" "=" float

strategy_clause ::= "STRATEGY" strategy_value
strategy_value  ::= "soft_delete" | "compress" | "hard_delete" | "archive"
```

### Forget Semantics

| Strategy | Behavior |
|----------|----------|
| soft_delete | Mark as deleted, retain for audit |
| compress | Summarize and reduce storage footprint |
| hard_delete | Permanently remove from storage |
| archive | Move to cold storage tier |

---

## 6. Tool Statements

```bnf
tool_stmt       ::= load_tools_stmt
                  | update_tools_stmt

load_tools_stmt ::= "LOAD" "TOOLS" predicate? modifier*

update_tools_stmt ::= "UPDATE" "TOOLS" "WHERE" condition payload
                      namespace_mod?
```

### Tool Registry Schema

```
Tool Entry
├── tool_id           ← unique identifier
├── schema            ← JSON schema for parameters
├── description       ← natural language description
├── token_cost        ← estimated tokens for schema + typical response
├── ranking           ← learned effectiveness score (0.0 - 1.0)
├── category          ← classification for filtering
├── last_used         ← timestamp
├── call_count        ← total invocations
└── success_rate      ← historical success ratio (0.0 - 1.0)
```

---

## 7. Modifiers

```bnf
modifier        ::= return_mod
                  | limit_mod
                  | weight_mod
                  | threshold_mod
                  | depth_mod
                  | decay_mod
                  | timeout_mod
                  | priority_mod
                  | source_mod
                  | order_mod
                  | confidence_mod
                  | tier_mod

return_mod      ::= "RETURN" field_list
field_list      ::= field ("," field)*

limit_mod       ::= "LIMIT" integer

weight_mod      ::= "WEIGHT" field
                  | "WEIGHT" float

threshold_mod   ::= "THRESHOLD" float

depth_mod       ::= "DEPTH" integer

decay_mod       ::= "DECAY" duration

timeout_mod     ::= "TIMEOUT" duration

priority_mod    ::= "PRIORITY" priority_value
priority_value  ::= "HIGH" | "NORMAL" | "LOW" | "CRITICAL"

source_mod      ::= "FROM" source_list
source_list     ::= source ("," source)*
source          ::= identifier

order_mod       ::= "ORDER" "BY" field sort_dir?
sort_dir        ::= "ASC" | "DESC"

confidence_mod  ::= "MIN_CONFIDENCE" float

tier_mod        ::= "TIER" tier_list
tier_list       ::= tier_name ("," tier_name)*
tier_name       ::= "hot" | "warm" | "cold" | "archive"

duration        ::= integer time_unit
time_unit       ::= "ms" | "s" | "m" | "h" | "d"
```

---

## 8. Reflect Statement

REFLECT is a cross-modal statement that pulls from multiple memory types simultaneously and assembles a unified context payload. It supports consistency checking and conflict resolution for multi-agent scenarios.

```bnf
reflect_stmt    ::= "REFLECT" context_expr
                    reflect_source*
                    check_clause?
                    resolve_clause?
                    modifier*
                    then_clause?

context_expr    ::= key_expr
                  | identifier

reflect_source  ::= "INCLUDE" memory_type predicate?

check_clause    ::= "CHECK" check_list
check_list      ::= check_dim ("," check_dim)*
check_dim       ::= "temporal" | "factual" | "logical" | "causal"

resolve_clause  ::= "RESOLVE" "CONFLICTS" "=" resolve_strategy
resolve_strategy ::= "merge" | "replace" | "flag" | "newest" | "highest_confidence"

then_clause     ::= "THEN" write_stmt
```

### Reflection Dimensions

| Dimension | What it checks |
|-----------|---------------|
| temporal | Timeline consistency across episodes |
| factual | Semantic facts don't contradict |
| logical | Procedural steps are coherent |
| causal | Cause-effect relationships hold |

---

## 9. Pipeline Statement

Pipelines chain multiple statements with a shared timeout budget. Each stage feeds context into the next.

```bnf
pipeline_stmt   ::= "PIPELINE" pipeline_name?
                    "TIMEOUT" duration
                    pipeline_stage+

pipeline_stage  ::= stage_stmt pipeline_op?

stage_stmt      ::= read_stmt
                  | tool_stmt
                  | reflect_stmt

pipeline_op     ::= "|"

pipeline_name   ::= identifier
```

---

## 10. Primitives

```bnf
identifier      ::= [a-zA-Z_][a-zA-Z0-9_]*

field           ::= identifier
                  | identifier "." identifier

value           ::= string_literal
                  | integer
                  | float
                  | boolean
                  | embedding_ref
                  | variable
                  | array_literal
                  | "now()"

string_literal  ::= '"' [^"]* '"'
                  | "'" [^']* "'"

integer         ::= "-"? [0-9]+

float           ::= "-"? [0-9]+ "." [0-9]+

boolean         ::= "true" | "false"

embedding_ref   ::= "$" identifier

variable        ::= "{" identifier "}"

array_literal   ::= "[" value_list? "]"
value_list      ::= value ("," value)*
```

---

## 11. Memory Type Characteristics

```
Memory Type  │ Storage Backend    │ Retrieval Trigger    │ Latency
─────────────┼────────────────────┼──────────────────────┼─────────
WORKING      │ DashMap            │ Direct scan          │ < 1ms
TOOLS        │ DashMap + ranking  │ Task relevance       │ < 2ms
PROCEDURAL   │ petgraph           │ Pattern / goal match │ < 5ms
SEMANTIC     │ usearch (HNSW)     │ Vector similarity    │ < 20ms
EPISODIC     │ Arrow + DataFusion │ Context / time cue   │ < 50ms
```

---

## 12. Working Memory Structure

Working memory is the **assembly layer** — the prepared context package:

```
Working Memory
├── Current task state        ← what am I doing right now
├── Active tool set           ← which tools are loaded
├── Relevant episodes         ← pulled from episodic, pre-loaded
├── Active procedures         ← the runbook I'm currently executing
├── Semantic context          ← concepts relevant to current task
└── Attention weights         ← what matters most right now
```

---

## 13. Example Statements

### Tool Registry Operations

```aql
-- Load relevant tools for current task
LOAD TOOLS WHERE relevance > 0.8
  ORDER BY ranking DESC
  LIMIT 3
  RETURN tool_id, schema, token_cost

-- Update tool ranking after successful execution
UPDATE TOOLS WHERE tool_id="github_list_repos" (
  ranking    = ranking + 0.1,
  call_count = call_count + 1
)
  NAMESPACE agent_id="agent_001"
```

### Forgetting with Decay

```aql
-- Soft delete old episodes with decay
FORGET EPISODIC
  WHERE activation < 0.3
  DECAY lambda=0.5 offset=0.1
  STRATEGY soft_delete

-- Hard delete expired working memory
FORGET WORKING
  WHERE ttl_expired = true
  STRATEGY hard_delete

-- Archive old semantic knowledge
FORGET SEMANTIC
  WHERE last_accessed < "2026-01-01"
  STRATEGY archive
```

### Store with Strength and Scope

```aql
-- Store shared knowledge across agent cluster
STORE SEMANTIC (
  concept   = "k8s_oom_pattern",
  knowledge = "payments-api OOMs every Friday after batch job",
  source    = "agent_observation"
)
  STRENGTH 0.87
  TTL 90d
  SCOPE shared
  NAMESPACE cluster="platform-agents"

-- Store private episode with conditional
STORE EPISODIC (
  incident_id = "inc-2026-03-28",
  action      = "scaled memory to 512Mi",
  resolved    = true
)
  STRENGTH 0.95
  SCOPE private
  NAMESPACE agent_id="agent_001"
  IF NOT EXISTS
```

### Recall with Quality Filter

```aql
-- Only recall high confidence memories
RECALL EPISODIC WHERE pod="payments-api"
  MIN_CONFIDENCE 0.7
  TIER hot, warm
  RETURN incident_id, action, resolved, confidence
  ORDER BY time DESC
  LIMIT 5

-- Recall semantic with threshold
RECALL SEMANTIC LIKE $current_context
  MIN_CONFIDENCE 0.8
  TIER hot
  THRESHOLD 0.85
  RETURN concept, knowledge, confidence
  LIMIT 10
```

### Multi-Agent REFLECT

```aql
-- Reflect with consistency check
REFLECT incident_id={current}
  INCLUDE EPISODIC WHERE incident_id={current}
  INCLUDE PROCEDURAL WHERE pattern_id={matched}
  INCLUDE WORKING
  CHECK temporal, factual, logical
  RESOLVE CONFLICTS = merge
  NAMESPACE cluster="platform-agents"

-- Reflect and learn
REFLECT incident_id={current}
  INCLUDE EPISODIC WHERE incident_id={current}
  INCLUDE SEMANTIC WHERE domain="kubernetes"
  CHECK factual, causal
  RESOLVE CONFLICTS = highest_confidence
  THEN STORE PROCEDURAL (
    pattern   = "oom_friday_batch",
    steps     = ["check batch job", "scale memory", "verify"],
    confidence = 0.85
  )
```

### RTB Pipeline v0.3

```aql
PIPELINE bid_decision TIMEOUT 80ms
  LOAD TOOLS WHERE task="bid_evaluation" LIMIT 3
  | LOOKUP SEMANTIC KEY url={url}
      MIN_CONFIDENCE 0.8
      TIER hot
  | RECALL SEMANTIC LIKE $page_context
      FROM creatives
      WHERE budget > 0
      LIMIT 5
      WEIGHT relevance_score
  | RECALL EPISODIC WHERE url={url}
      TIER hot, warm
      LIMIT 10
  | REFLECT url={url}
      INCLUDE EPISODIC
      INCLUDE SEMANTIC
      INCLUDE PROCEDURAL
      CHECK temporal, factual
```

### Kubernetes Log Analysis v0.3

```aql
-- Match pattern with confidence filter
LOOKUP PROCEDURAL PATTERN $log_event
  THRESHOLD 0.85
  MIN_CONFIDENCE 0.7
  RETURN pattern_id, severity, action_steps, confidence

-- Full incident pipeline
PIPELINE incident_response TIMEOUT 200ms
  LOOKUP PROCEDURAL PATTERN $log_event
      THRESHOLD 0.85
  | RECALL EPISODIC WHERE pattern_id={matched}
      TIER hot, warm
      LIMIT 10
  | LOAD TOOLS WHERE category="kubernetes" LIMIT 5
  | REFLECT incident_id={current}
      INCLUDE EPISODIC WHERE incident_id={current}
      INCLUDE PROCEDURAL WHERE pattern_id={matched}
      INCLUDE WORKING
      CHECK temporal, factual, causal
      RESOLVE CONFLICTS = merge
      THEN STORE EPISODIC (
        incident_id = {current},
        pattern_id  = {matched},
        action      = {action_taken},
        resolved    = {outcome}
      )
        STRENGTH 0.9
        SCOPE shared
        NAMESPACE cluster="platform-agents"
```

### Working Memory Operations

```aql
-- Scan current assembled context
SCAN WORKING ALL
  RETURN task_state, active_tools, attention_weights

-- Store task into working memory with TTL
STORE WORKING (
  task_id    = "bid_eval_001",
  url        = "sports.example.com",
  started_at = now()
)
  TTL 5m
  PRIORITY HIGH

-- Clear completed task
FORGET WORKING
  WHERE task_id="bid_eval_001"
  STRATEGY hard_delete
```

### Memory Linking

```aql
-- Link episode to semantic concept
LINK EPISODIC episode_id="evt_001"
  TO SEMANTIC concept_id="payment_failure"
  WEIGHT 0.9

-- Link pattern to procedure
LINK SEMANTIC concept_id="k8s_oom"
  TO PROCEDURAL procedure_id="restart_pod"
  WEIGHT 1.0
```

### Update with Concurrency Control

```aql
-- Optimistic update with version check
UPDATE PROCEDURAL WHERE pattern_id="oom-kill-001" (
  steps      = ["check memory", "scale up", "verify"],
  confidence = 0.97
)
  VERSION 3
  NAMESPACE agent_id="agent_001"

-- Pessimistic lock for critical update
UPDATE SEMANTIC WHERE concept_id="pricing_model" (
  value = 0.15,
  updated_by = "agent_001"
)
  LOCK pessimistic
```

---

## 14. ADB Container Architecture

```
┌───────────────────────────────────────────────────────────┐
│                    Agent Container                        │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐  │
│  │                    ADB Process                      │  │
│  │                                                     │  │
│  │  ┌───────────────────────────────────────────────┐  │  │
│  │  │              Working Memory                   │  │  │
│  │  │            (Assembly Layer)                   │  │  │
│  │  │  ┌──────────┬──────────┬────────────────┐     │  │  │
│  │  │  │ Task     │ Active   │ Attention      │     │  │  │
│  │  │  │ State    │ Tools    │ Weights        │     │  │  │
│  │  │  ├──────────┼──────────┼────────────────┤     │  │  │
│  │  │  │ Loaded   │ Loaded   │ Loaded         │     │  │  │
│  │  │  │ Episodes │ Concepts │ Procedures     │     │  │  │
│  │  │  └──────────┴──────────┴────────────────┘     │  │  │
│  │  └───────────────────────────────────────────────┘  │  │
│  │                                                     │  │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────────┐      │  │
│  │  │ Episodic  │ │ Semantic  │ │  Procedural   │      │  │
│  │  │ Arrow +   │ │ usearch   │ │  petgraph     │      │  │
│  │  │ DataFusion│ │ (HNSW)    │ │               │      │  │
│  │  └───────────┘ └───────────┘ └───────────────┘      │  │
│  │                                                     │  │
│  │  ┌───────────────────────────────────────────────┐  │  │
│  │  │               Tool Registry                   │  │  │
│  │  │    DashMap + Rankings + Token Costs           │  │  │
│  │  └───────────────────────────────────────────────┘  │  │
│  │                                                     │  │
│  │                AQL Query Planner                    │  │
│  │                Arrow Flight IPC                     │  │
│  └────────────────────────┬────────────────────────────┘  │
│                           │ Unix socket / loopback        │
│  ┌────────────────────────┴────────────────────────────┐  │
│  │                   Agent Runtime                     │  │
│  │                                                     │  │
│  │      AQL query → assembled context (Working)       │  │
│  │                        ↓                           │  │
│  │                       LLM                          │  │
│  │                        ↓                           │  │
│  │                  decision/action                   │  │
│  │                        ↓                           │  │
│  │                 write-back to ADB                  │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
```

---

## 15. What AQL Is Not Responsible For

| Responsibility | Owner |
|---------------|-------|
| Deciding next action | LLM |
| Executing actions | Agent Runtime |
| Evaluating outcomes | LLM + Agent Runtime |
| Reasoning over context | LLM |
| Populating context | **AQL / ADB** |
| Storing outcomes | **AQL / ADB** (write-back) |
| Tool execution | Agent Runtime |
| Tool selection ranking | **AQL / ADB** (Tool Registry) |

---

## 16. Changes from v0.2

### New Statements
- `FORGET` with decay model and strategy
- `LOAD TOOLS` as first-class verb
- `UPDATE TOOLS` for ranking updates

### New Modifiers
- `STRENGTH` — importance weighting on STORE
- `TTL` — time-to-live duration
- `SCOPE` — private / shared / cluster
- `NAMESPACE` — agent identity isolation
- `MIN_CONFIDENCE` — quality filter on RECALL
- `TIER` — hot / warm / cold / archive storage selection
- `IF NOT EXISTS` — conditional STORE
- `VERSION` / `LOCK` — concurrency control on UPDATE

### New REFLECT Capabilities
- `CHECK` clause — temporal, factual, logical, causal dimensions
- `RESOLVE CONFLICTS` — merge, replace, flag, newest, highest_confidence

### New Primitives
- `now()` function for timestamps
- Array literals `[value, value, ...]`
- Single-quoted strings
- `IN` comparator
- `NOT` condition prefix
- Day duration unit `d`

---

## 17. Open Questions

1. **Embedding literals** — how does a caller pass a live embedding vector into a LIKE predicate?
2. **Streaming RECALL** — should episodic recall support streaming for long histories?
3. **Tool eviction** — when working memory is full, which tools get evicted first?
4. **Cross-cluster memory** — federation across multiple ADB instances?
5. **Encryption** — per-agent encryption keys for regulated industries?
6. **Compression strategies** — what summarization algorithm for STRATEGY compress?

---

*AQL v0.3 — Working Specification*
*March 2026 · Sriram Reddy*
*github.com/srirammails/AQL*
