# AQL — Agent Query Language
## Formal Grammar Specification v0.2

---

## 1. Design Principles

1. **AQL is a retrieval and storage language, not a decision language.** It populates agent context. The LLM reasons over that context.
2. **Every statement is scoped to a memory type.** Episodic, Semantic, Procedural, Working, or Tools.
3. **Working memory is the assembly layer.** It's the prepared context package — everything the agent needs for the next reasoning step, pre-loaded and ready. Sub-millisecond. Never blocks.
4. **Tool Registry is a first-class memory type.** Available tools, rankings, and token costs are queryable like any other memory.
5. **Retrieval mode is explicit.** The verb encodes intent, not just predicate.
6. **Pipelines are first-class.** Agentic queries chain across memory types with a time budget.

---

## 2. Top-Level Grammar (BNF)

```bnf
program         ::= statement+

statement       ::= read_stmt
                  | write_stmt
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

exact_pred      ::= "KEY" identifier
                  | "WHERE" condition

similarity_pred ::= "LIKE" expression
                  | "PATTERN" expression

scan_pred       ::= "ALL"
                  | "WHERE" condition

condition       ::= field comparator value
                  | condition "AND" condition
                  | condition "OR" condition
                  | "(" condition ")"

comparator      ::= "=" | "!=" | ">" | "<" | ">=" | "<=" | "~"

```

### Read Verb Semantics

| Verb | Memory Types | Retrieval Mode |
|------|-------------|----------------|
| LOOKUP | SEMANTIC, PROCEDURAL, TOOLS | Exact key or pattern match |
| RECALL | EPISODIC, SEMANTIC | Similarity / context match |
| SCAN | WORKING | Full scan of assembled context |
| LOAD | TOOLS | Select and activate tools into working memory |

---

## 4. Write Statements

```bnf
write_stmt      ::= write_verb memory_type payload modifier*

write_verb      ::= "STORE"
                  | "UPDATE"
                  | "FORGET"
                  | "LINK"

payload         ::= "(" field_value_list ")"
                  | "FROM" identifier
                  | "FROM" "THIS_EPISODE"

field_value_list ::= field_value ("," field_value)*
field_value     ::= field "=" value

```

### Write Verb Semantics

| Verb | Purpose |
|------|---------|
| STORE | Write new memory into a memory type |
| UPDATE | Modify existing memory entry |
| FORGET | Expire or decay a memory entry |
| LINK | Create relationship between two memory entries |

---

## 5. Modifiers

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

return_mod      ::= "RETURN" field_list
field_list      ::= field ("," field)*

limit_mod       ::= "LIMIT" integer

weight_mod      ::= "WEIGHT" field
                  | "WEIGHT" float

threshold_mod   ::= "THRESHOLD" float

depth_mod       ::= "DEPTH" integer

decay_mod       ::= "DECAY" duration

timeout_mod     ::= "TIMEOUT" duration

priority_mod    ::= "PRIORITY" ("HIGH" | "NORMAL" | "LOW")

source_mod      ::= "FROM" source_list
source_list     ::= source ("," source)*
source          ::= identifier

order_mod       ::= "ORDER BY" field ("ASC" | "DESC")?

duration        ::= integer time_unit
time_unit       ::= "ms" | "s" | "m" | "h"

```

---

## 6. Reflect Statement

REFLECT is a special cross-modal statement. It pulls from multiple memory types simultaneously and assembles a unified context payload for the LLM. It does not decide — it synthesizes.

```bnf
reflect_stmt    ::= "REFLECT" context_expr
                    reflect_source*
                    modifier*
                    then_clause?

context_expr    ::= field "=" value
                  | identifier

reflect_source  ::= "INCLUDE" memory_type predicate?

then_clause     ::= "THEN" write_stmt

```

REFLECT is the only statement that can span all four memory types in a single call. The `THEN` clause allows a write-back when reflection produces a new insight — for example storing a newly learned pattern back into procedural memory.

---

## 7. Pipeline Statement

Pipelines chain multiple statements with a shared timeout budget. Each stage feeds context into the next.

```bnf
pipeline_stmt   ::= "PIPELINE" pipeline_name?
                    "TIMEOUT" duration
                    pipeline_stage+

pipeline_stage  ::= (read_stmt | reflect_stmt) pipeline_op?

pipeline_op     ::= "|"

pipeline_name   ::= identifier

```

---

## 8. Primitives

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

string_literal  ::= '"' [^"]* '"'
integer         ::= [0-9]+
float           ::= [0-9]+ "." [0-9]+
boolean         ::= "true" | "false"
embedding_ref   ::= "$" identifier
variable        ::= "{" identifier "}"

```

---

## 9. Example Statements

### RTB Scenario

```aql
-- Query 1: Do I know this URL and what does it mean?
LOOKUP SEMANTIC KEY url="sports.example.com"
  RETURN context, categories, historical_signal

-- Query 2: What creatives match this page context?
RECALL SEMANTIC LIKE $page_context
  FROM creatives
  WHERE budget > 0
  LIMIT 5
  WEIGHT relevance_score

-- Query 3: What happened last time on this URL?
RECALL EPISODIC WHERE url="sports.example.com"
  RETURN bid_price, impression, click, conversion
  ORDER BY time DESC
  LIMIT 10

-- Full pipeline with timeout budget
PIPELINE bid_decision TIMEOUT 80ms
  LOOKUP SEMANTIC KEY url={url}
  | RECALL SEMANTIC LIKE $page_context FROM creatives LIMIT 5
  | RECALL EPISODIC WHERE url={url} LIMIT 10
  | REFLECT url={url}
      INCLUDE EPISODIC
      INCLUDE SEMANTIC
      INCLUDE PROCEDURAL
```

---

### Kubernetes Log Analysis Scenario

```aql
-- Query 1: Does this event match a known pattern?
LOOKUP PROCEDURAL PATTERN $log_event
  THRESHOLD 0.85
  RETURN pattern_id, severity, confidence

-- Query 2: What actions are defined for this pattern?
LOOKUP PROCEDURAL WHERE pattern_id={matched}
  FROM confluence, jira
  RETURN action_steps, priority, owner
  DEPTH 2

-- Query 3: Did my actions resolve the incident?
RECALL EPISODIC WHERE incident_id={current}
  RETURN actions_taken, outcomes, timeline

-- Reflect and write back if unresolved
REFLECT incident_id={current}
  INCLUDE EPISODIC WHERE incident_id={current}
  INCLUDE PROCEDURAL WHERE pattern_id={matched}
  THEN STORE PROCEDURAL (
    pattern = this_episode,
    source  = "learned",
    weight  = 0.6
  )
```

---

### Working Memory Operations

```aql
-- Scan current active state
SCAN WORKING ALL
  RETURN context, active_tasks, attention_weights

-- Store current task into working memory
STORE WORKING (
  task_id    = "bid_eval_001",
  url        = "sports.example.com",
  started_at = now()
)
  DECAY 5m

-- Clear working memory after reasoning cycle
FORGET WORKING WHERE task_id="bid_eval_001"
```

---

### Memory Linking

```aql
-- Link an episode to a semantic concept
LINK EPISODIC episode_id="evt_001"
  TO SEMANTIC concept_id="payment_failure"
  WEIGHT 0.9

-- Link a pattern to its action procedure
LINK SEMANTIC concept_id="k8s_oom"
  TO PROCEDURAL procedure_id="restart_pod"
  WEIGHT 1.0
```

---

## 10. Memory Type Characteristics

```
Memory Type  │ Retrieval Trigger    │ Return Type           │ Latency Target
─────────────┼──────────────────────┼───────────────────────┼────────────────
WORKING      │ Direct scan          │ Assembled context     │ < 1ms
TOOLS        │ Task relevance       │ Tool schemas + costs  │ < 2ms
PROCEDURAL   │ Pattern / goal match │ Executable steps      │ < 5ms
SEMANTIC     │ Concept similarity   │ Structured facts      │ < 20ms
EPISODIC     │ Context / time cue   │ Event sequences       │ < 50ms
```

---

## 10.1 Working Memory Structure

Working memory is not just "active state" — it's the **assembled context package** for the next reasoning step:

```
Working Memory
├── Current task state        ← what am I doing right now
├── Active tool set           ← which tools are loaded
├── Relevant episodes         ← pulled from episodic, pre-loaded
├── Active procedures         ← the runbook I'm currently executing
├── Semantic context          ← concepts relevant to current task
└── Attention weights         ← what matters most right now
```

Working memory becomes the **assembly layer** — AQL populates it from all five memory types:

```aql
PIPELINE prepare_context TIMEOUT 10ms
  SCAN WORKING ALL
  | RECALL EPISODIC WHERE task={current} LIMIT 3
  | LOOKUP PROCEDURAL WHERE goal={current}
  | RECALL SEMANTIC LIKE $current_context LIMIT 5
  | LOAD TOOLS WHERE relevance > 0.8
```

---

## 10.2 Tool Registry

Tool Registry is a first-class memory type. It stores available tools, their schemas, rankings, and token costs.

```aql
-- Query available tools by task relevance
LOOKUP TOOLS WHERE task_relevance > 0.8
  ORDER BY ranking DESC
  LIMIT 3
  RETURN tool_id, schema, token_cost

-- Load minimum tools into working memory
LOAD TOOLS WHERE category = "file_operations"
  THRESHOLD 0.7
  LIMIT 5

-- Store tool usage outcome for ranking updates
STORE TOOLS (
  tool_id    = "read_file",
  success    = true,
  latency_ms = 12,
  task_id    = {current}
)
```

### Tool Registry Schema

```
Tool Entry
├── tool_id           ← unique identifier
├── schema            ← JSON schema for parameters
├── description       ← natural language description
├── token_cost        ← estimated tokens for schema + typical response
├── ranking           ← learned effectiveness score
├── category          ← classification for filtering
├── last_used         ← timestamp
└── success_rate      ← historical success ratio
```

---

## 11. ADB Container Architecture

```
┌───────────────────────────────────────────────────────┐
│                  Agent Container                      │
│                                                       │
│  ┌─────────────────────────────────────────────────┐  │
│  │                   ADB Process                   │  │
│  │                                                 │  │
│  │  ┌─────────────────────────────────────────┐    │  │
│  │  │           Working Memory                │    │  │
│  │  │         (Assembly Layer)                │    │  │
│  │  │  ┌─────────┬─────────┬───────────────┐  │    │  │
│  │  │  │ Task    │ Active  │ Attention     │  │    │  │
│  │  │  │ State   │ Tools   │ Weights       │  │    │  │
│  │  │  ├─────────┼─────────┼───────────────┤  │    │  │
│  │  │  │ Loaded  │ Loaded  │ Loaded        │  │    │  │
│  │  │  │Episodes │Concepts │ Procedures    │  │    │  │
│  │  │  └─────────┴─────────┴───────────────┘  │    │  │
│  │  └─────────────────────────────────────────┘    │  │
│  │                                                 │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐     │  │
│  │  │ Episodic │ │ Semantic │ │  Procedural  │     │  │
│  │  │(Time-srs │ │ (Vector  │ │  (Graph+KV)  │     │  │
│  │  │ +Vector) │ │  +Graph) │ │              │     │  │
│  │  └──────────┘ └──────────┘ └──────────────┘     │  │
│  │                                                 │  │
│  │  ┌──────────────────────────────────────────┐   │  │
│  │  │              Tool Registry               │   │  │
│  │  │  (Schemas + Rankings + Token Costs)      │   │  │
│  │  └──────────────────────────────────────────┘   │  │
│  │                                                 │  │
│  │              AQL Query Planner                  │  │
│  └───────────────────────┬─────────────────────────┘  │
│                          │ Unix socket / loopback     │
│  ┌───────────────────────┴─────────────────────────┐  │
│  │                 Agent Runtime                   │  │
│  │                                                 │  │
│  │     AQL query → assembled context (Working)    │  │
│  │                       ↓                        │  │
│  │                      LLM                       │  │
│  │                       ↓                        │  │
│  │                 decision/action                │  │
│  │                       ↓                        │  │
│  │                write-back to ADB               │  │
│  └─────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────┘
```

---

## 12. What AQL Is Not Responsible For

| Responsibility | Owner |
|---------------|-------|
| Deciding next action | LLM |
| Executing actions | Agent Runtime |
| Evaluating outcomes | LLM + Agent Runtime |
| Reasoning over context | LLM |
| Populating context | **AQL / ADB** |
| Storing outcomes | **AQL / ADB** (write-back) |

---

## 13. Open Grammar Questions

1. **Embedding literals** — how does a caller pass a live embedding vector into a LIKE predicate?
2. **Cross-agent memory** — does AQL need a namespace for shared vs. private memory?
3. **Streaming RECALL** — should episodic recall support streaming results for long histories?
4. **Conditional STORE** — should STORE support IF NOT EXISTS semantics?
5. **Memory versioning** — does UPDATE need optimistic concurrency control?
6. **Tool ranking updates** — should STORE TOOLS auto-update rankings, or require explicit UPDATE?
7. **Tool eviction** — when working memory is full, which tools get evicted first?

---

## 14. Resolved Questions (v0.2)

1. **Working memory scope** — Working memory is the assembled context package, not just active state. It contains pre-loaded episodes, concepts, procedures, and active tools ready for the next reasoning step.

2. **Tool selection** — Tool Registry is a first-class memory type. LOAD TOOLS selects and activates tools into working memory based on task relevance, rankings, and token budget.

---

*AQL v0.2 — Working Specification*
*ADB / AQL Project — Sriram*
