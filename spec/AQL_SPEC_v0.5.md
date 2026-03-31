# AQL — Agent Query Language
## Formal Grammar Specification v0.5

---

## 1. Design Principles

1. **AQL expresses intent, not implementation.** ADB handles storage, decay, conflict resolution.
2. **Readable without a spec.** If a non-expert can't understand a query, cut the feature.
3. **Every statement is scoped to a memory type.** Episodic, Semantic, Procedural, Working, or Tools.
4. **Working memory is the assembly layer.** The prepared context package for the next reasoning step.
5. **Tool Registry is a first-class memory type.** Queryable like any other memory.
6. **Pipelines are first-class.** Chain queries with a time budget.
7. **Multi-agent support is simple.** SCOPE and NAMESPACE — nothing more.
8. **FlowR owns transformation.** AQL retrieves raw context. FlowR shapes it for the LLM.

---

## 2. Top-Level Grammar (BNF)

```
program         ::= statement+

statement       ::= read_stmt
                  | write_stmt
                  | forget_stmt
                  | pipeline_stmt
                  | reflect_stmt
```

---

## 3. Read Statements

```
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
                  | window_pred

window_pred     ::= "WINDOW" window_type

window_type     ::= "LAST" integer
                  | "LAST" duration
                  | "TOP" integer "BY" field
                  | "SINCE" key_expr

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

### WINDOW — Working Memory Only

WINDOW is only valid on SCAN WORKING.
All other memory types reject WINDOW with a clear error.

| Window type | Meaning |
|-------------|---------|
| LAST N | Most recent N items added to working memory |
| LAST duration | Items added within last N ms/s/m/h |
| TOP N BY field | Top N items ranked by field value |
| SINCE key_expr | All items added since a specific event |

---

## 4. Write Statements

```
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
|------|-------------|
| STORE | Write new memory |
| UPDATE | Modify existing memory |
| LINK | Connect two memories |

---

## 5. Forget Statement

```
forget_stmt     ::= "FORGET" memory_type predicate
```

ADB decides how to forget. AQL just says what to forget.

---

## 6. Modifiers

```
modifier        ::= return_mod
                  | limit_mod
                  | weight_mod
                  | threshold_mod
                  | timeout_mod
                  | order_mod
                  | confidence_mod
                  | source_mod
                  | aggregate_mod
                  | having_mod

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

aggregate_mod   ::= "AGGREGATE" aggregate_func ("," aggregate_func)*

aggregate_func  ::= agg_op "(" field ")" "AS" identifier
                  | "COUNT" "(" "*" ")" "AS" identifier

agg_op          ::= "COUNT" | "AVG" | "SUM" | "MIN" | "MAX"

having_mod      ::= "HAVING" condition

duration        ::= integer time_unit
time_unit       ::= "ms" | "s" | "m" | "h" | "d"
```

### AGGREGATE — Valid on RECALL and LOOKUP

AGGREGATE summarises retrieved records before returning.
Five functions only: COUNT, AVG, SUM, MIN, MAX.

HAVING filters on the aggregated result.
Must follow AGGREGATE — invalid without it.

---

## 7. Reflect Statement

REFLECT assembles context from multiple memory types.
ADB handles consistency internally.
FlowR transforms the assembled context for the LLM.

```
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

```
pipeline_stmt   ::= "PIPELINE" identifier?
                    "TIMEOUT" duration
                    pipeline_stage+

pipeline_stage  ::= (read_stmt | reflect_stmt) "|"?
```

---

## 9. Primitives

```
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

### WINDOW — Working Memory

```sql
-- Last 10 items added to working memory
SCAN WORKING WINDOW LAST 10

-- Everything added in last 30 seconds
SCAN WORKING WINDOW LAST 30s

-- Top 3 items by attention weight
SCAN WORKING WINDOW TOP 3 BY attention_weight
  RETURN event, status, attention_weight

-- Everything since this incident started
SCAN WORKING WINDOW SINCE event_id = "inc-001"
  RETURN event, status, timestamp

-- Common pattern: what has my agent been doing?
-- Called before every LLM reasoning step
SCAN WORKING WINDOW LAST 10
  RETURN event, status, timestamp
```

### AGGREGATE — Summarise Before Returning

```sql
-- How many times has this pod OOMKilled?
RECALL EPISODIC WHERE pod = "payments-api"
  AGGREGATE COUNT(*) AS total_incidents
  RETURN total_incidents

-- What was the average bid price for this URL?
RECALL EPISODIC WHERE url = "sports.example.com"
  AGGREGATE AVG(bid_price) AS avg_cpm,
            COUNT(*) AS impressions
  RETURN avg_cpm, impressions

-- What is the highest confidence procedure?
LOOKUP PROCEDURAL WHERE category = "k8s"
  AGGREGATE MAX(confidence) AS best_confidence
  RETURN pattern_id, best_confidence
  ORDER BY best_confidence DESC
  LIMIT 1

-- Which URLs had more than 5 incidents?
RECALL EPISODIC WHERE campaign = "summer"
  AGGREGATE COUNT(*) AS incidents
  HAVING incidents > 5
  RETURN url, incidents
  ORDER BY incidents DESC
```

### WINDOW + AGGREGATE together

```sql
-- How many events in last 60 seconds?
SCAN WORKING WINDOW LAST 60s
  AGGREGATE COUNT(*) AS recent_count
  RETURN recent_count

-- What is average attention in recent window?
SCAN WORKING WINDOW LAST 5
  AGGREGATE AVG(attention_weight) AS avg_attention
  RETURN avg_attention
```

### Full RTB Pipeline with AGGREGATE

```sql
PIPELINE bid_decision TIMEOUT 80ms
  LOAD TOOLS WHERE task = "bidding" AND ad_format = "display"
    ORDER BY ranking DESC LIMIT 3
  | LOOKUP SEMANTIC KEY url = {url}
      RETURN iab_category, brand_safety, avg_cpm
  | RECALL EPISODIC WHERE url = {url}
      AGGREGATE AVG(bid_price) AS avg_cpm,
                COUNT(*) AS impressions,
                AVG(ctr) AS avg_ctr
      MIN_CONFIDENCE 0.7
      RETURN avg_cpm, impressions, avg_ctr
  | LOOKUP PROCEDURAL PATTERN $bid_features
      THRESHOLD 0.85
      RETURN pattern, action, priority
  | REFLECT url = {url}
      INCLUDE SEMANTIC
      INCLUDE EPISODIC
      INCLUDE PROCEDURAL
```

### Full K8s Pipeline with WINDOW

```sql
PIPELINE incident TIMEOUT 200ms
  -- What is agent currently tracking?
  SCAN WORKING WINDOW LAST 10
    RETURN event, status
  | LOOKUP PROCEDURAL PATTERN $log_event
      THRESHOLD 0.85
      RETURN pattern_id, severity, action_steps
  | RECALL EPISODIC WHERE pod = {pod}
      AGGREGATE COUNT(*) AS total_incidents,
                AVG(resolution_time) AS avg_resolution
      MIN_CONFIDENCE 0.7
      LIMIT 5
      RETURN incident_id, action, resolved,
             total_incidents, avg_resolution
  | LOAD TOOLS WHERE category = "kubernetes"
      LIMIT 3
  | REFLECT incident_id = {current}
      INCLUDE WORKING
      INCLUDE EPISODIC
      INCLUDE PROCEDURAL
```

### FlowR + AQL Together

```
FlowR flow receives K8s event
  ↓
Step 1: AQL SCAN WORKING WINDOW LAST 5
        FlowR checks: is this pod already being handled?
  ↓
Step 2: AQL RECALL EPISODIC WHERE pod={pod}
        AGGREGATE COUNT(*) AS incidents
        FlowR checks: incidents > 3 → escalate path
  ↓
Step 3: AQL LOOKUP PROCEDURAL PATTERN $event
        FlowR extracts action_steps for LLM context
  ↓
Step 4: FlowR shapes context:
        "Pod OOMKilled 4 times. Avg resolution: 8min.
         Runbook: scale + notify. 3 recent steps in working memory."
  ↓
Step 5: Shaped context → LLM → decision
  ↓
Step 6: AQL STORE EPISODIC (outcome)
        AQL FORGET WORKING WHERE pod={pod}  ← clear after resolution
```

AQL retrieves. FlowR shapes. LLM decides.

---

## 12. What AQL Does NOT Do

| Concern | Owner |
|---------|-------|
| Deciding next action | LLM |
| Executing actions | Agent Runtime / FlowR |
| Transforming retrieved data | FlowR |
| Decay algorithms | ADB |
| Storage tiers | ADB |
| Conflict resolution | ADB |
| Compression | ADB |
| Concurrency control | ADB |

**The rule:**
- If it is about *how* memory works → ADB's job
- If it is about *shaping* data for the LLM → FlowR's job
- If it is about *what* the agent wants to know → AQL's job

---

## 13. Changes from v0.4

### Added

**WINDOW predicate** — working memory only
```
SCAN WORKING WINDOW LAST 10
SCAN WORKING WINDOW LAST 30s
SCAN WORKING WINDOW TOP 3 BY attention_weight
SCAN WORKING WINDOW SINCE event_id = "inc-001"
```

Why: agents repeat "what have I been tracking recently"
on every reasoning step. WINDOW makes this a first-class
pattern instead of a workaround.

**AGGREGATE modifier** — RECALL and LOOKUP only
```
AGGREGATE COUNT(*) AS total
AGGREGATE AVG(bid_price) AS avg_cpm, COUNT(*) AS impressions
```

**HAVING modifier** — only valid after AGGREGATE
```
AGGREGATE COUNT(*) AS incidents
HAVING incidents > 5
```

Why: agents need summarised signals not raw records.
"How many times did this happen?" is retrieval intent.
The LLM should receive the answer, not the raw rows.

### Principle clarified

FlowR owns data transformation and context shaping.
AQL owns retrieval and storage.
This removes any need for computed fields or complex
manipulation in the query language.

### Grammar size

| Version | Rules | Modifiers | Complexity |
|---------|-------|-----------|------------|
| v0.1 | 25 | 10 | Baseline |
| v0.2 | 28 | 12 | +12% |
| v0.3 | 45 | 22 | +80% |
| v0.4 | 30 | 8 | +20% |
| v0.5 | 35 | 10 | +40% |

v0.5 adds WINDOW and AGGREGATE.
Still 30% smaller than v0.3.
Every addition has a real use case.

---

## 14. The Readability Test

Every query must pass: *Can a non-expert read this and know what it does?*

```sql
SCAN WORKING WINDOW LAST 10
```
✓ Scan working memory, return last 10 items

```sql
RECALL EPISODIC WHERE pod = "payments-api"
  AGGREGATE COUNT(*) AS total_incidents
```
✓ Recall episodes about payments-api, count them

```sql
SCAN WORKING WINDOW TOP 3 BY attention_weight
```
✓ Scan working memory, return top 3 by attention weight

```sql
RECALL EPISODIC WHERE campaign = "summer"
  AGGREGATE COUNT(*) AS incidents
  HAVING incidents > 5
```
✓ Recall summer campaign episodes, count them,
  only return where count exceeds 5

---

## 15. Open Questions for v0.6

1. **GROUP BY** — group episodic records by field before aggregating
   ```sql
   RECALL EPISODIC WHERE campaign = "summer"
     GROUP BY url
     AGGREGATE COUNT(*) AS impressions, AVG(ctr) AS avg_ctr
   ```

2. **WINDOW on EPISODIC** — time window on past episodes
   ```sql
   RECALL EPISODIC WHERE pod = "payments-api"
     WINDOW LAST 7d
   ```
   Currently achieved with ORDER BY time DESC + LIMIT.
   Explicit WINDOW may be cleaner.

3. **MERGE statement** — explicitly merge two memory types
   ```sql
   MERGE EPISODIC WHERE url = {url}
     WITH SEMANTIC KEY url = {url}
     ON concept = url
   ```
   Currently handled by REFLECT + FlowR.

4. **Streaming RECALL** — for long episodic histories
   ```sql
   RECALL EPISODIC WHERE agent_id = "agent-001"
     STREAM BATCH 100
   ```

---

*AQL v0.5 — Specification*
*Changes: WINDOW predicate + AGGREGATE modifier + HAVING modifier*
*March 2026 · Sriram Reddy*
*github.com/srirammails/AQL*
