# AQL — Agent Query Language
## Formal Grammar Specification v0.5
## With Design Rationale

---

## Preface

This document is the authoritative AQL v0.5 specification.
It includes the complete grammar AND the reasoning behind every
design decision. Read the reasoning before implementing.

**Core thesis:**
> Agent learning is the accumulation of two variables:
> a dynamic ontology of relationship types between memory records,
> and tuned execution parameters within procedural memory.
> AQL provides the query interface for both.
> Agents improve through use without retraining.

**The one rule:**
> AQL expresses what the agent wants.
> ADB handles how memory works.
> FlowR shapes context for the LLM.
> The LLM decides what happens next.

**The intelligence stack:**
> River/statistical models → fast numerical signals (ms)
> AQL ontology → structured domain knowledge with evidence (ms)
> ART/RL fine-tuning → better base reasoning (periodic, offline)
> Each layer makes the others more effective.
> The ontology is the persistent, queryable, auditable layer
> that no other approach provides.

---

## 1. Design Principles

1. **FROM and INTO express direction explicitly.**
   Read operations use FROM. Write operations use INTO.
   Direction is never ambiguous.

2. **AQL expresses intent, not implementation.**
   ADB handles storage, decay, conflict resolution, tiering.
   AQL never describes how memory works internally.

3. **Readable without a spec.**
   If a non-expert cannot read a query and understand it,
   the feature is too complex. Cut it.

4. **Every statement is scoped to a memory type or ALL.**
   Episodic, Semantic, Procedural, Working, Tools, or ALL.

5. **Working memory is the assembly layer.**
   The prepared context package for the next LLM reasoning step.
   WINDOW is working memory's primary access pattern.

6. **Tool Registry is a first-class memory type.**
   Tools are queryable, rankable, and learnable like any memory.

7. **Pipelines are first-class.**
   Chained queries with a hard time budget are a native concept.

8. **Multi-agent support is simple.**
   SCOPE and NAMESPACE only. Nothing more.

9. **FlowR owns transformation and context shaping.**
   AQL retrieves raw records. FlowR transforms them.
   The LLM receives shaped context, not raw queries.

10. **Ontology is dynamic and agent-owned.**
    LINK TYPE is an arbitrary string. ADB stores it.
    The LLM defines what TYPE strings mean.
    Ontology emerges from experience, not from a fixed schema.

11. **Agent learning lives in two places only:**
    The ontology (LINK relationships between memories) and
    the variables inside procedural memory records.
    Both are writable by the LLM via AQL.

12. **FOLLOW LINKS crosses memory type boundaries.**
    A query starting in SEMANTIC can follow links into PROCEDURAL.
    The result set contains records from the target memory type.
    This is how the agent traverses its own knowledge graph.

---

## 2. Why FROM and INTO

**Problem with v0.4 syntax:**
```
RECALL EPISODIC WHERE pod = "payments"
STORE EPISODIC (event = "oom")
```
The memory type floats after the verb. Direction is implicit.
New users must learn which verbs read vs write.

**v0.5 solution:**
```
RECALL FROM EPISODIC WHERE pod = "payments"
STORE INTO EPISODIC (event = "oom")
```
FROM = reading. INTO = writing. One rule. Never broken.
Any engineer reads this without a spec.

**ALL is a valid memory target for reads and deletes:**
```
RECALL FROM ALL WHERE url = "example.com"
FORGET FROM ALL WHERE temp = "true"
```
Searches or clears across all backends simultaneously.
ADB handles fan-out and result merging internally.
STORE INTO ALL is invalid — writes must target a specific type.

---

## 3. Top-Level Grammar (BNF)

```bnf
program      ::= statement+

statement    ::= read_stmt
               | write_stmt
               | forget_stmt
               | link_stmt
               | reflect_stmt
               | pipeline_stmt
```

---

## 4. Memory Types

```bnf
memory_type   ::= "EPISODIC"
                | "SEMANTIC"
                | "PROCEDURAL"
                | "WORKING"
                | "TOOLS"

memory_target ::= memory_type | "ALL"
```

| Memory Type | What it stores | Latency | Backend |
|-------------|---------------|---------|---------|
| WORKING | Current task context, active state | < 1ms | DashMap |
| TOOLS | Available tools, rankings, schemas | < 2ms | DashMap + scores |
| PROCEDURAL | How-to knowledge, runbooks, learned variables | < 5ms | Graph |
| SEMANTIC | Facts, concepts, world knowledge | < 20ms | Vector |
| EPISODIC | Past events, outcomes, history | < 50ms | Time-series |

**Why this order matters:**
Latency tiers reflect access frequency during agent reasoning.
Working memory is checked on every step. Episodic is checked
when historical context is needed. The agent's hot path touches
WORKING and TOOLS on every cycle; SEMANTIC, PROCEDURAL, and
EPISODIC are queried situationally.

---

## 5. Read Statements

```bnf
read_stmt    ::= read_verb "FROM" memory_target predicate modifier*

read_verb    ::= "SCAN"
               | "RECALL"
               | "LOOKUP"
               | "LOAD"
```

### Read Verb Semantics

| Verb | Valid memory targets | Retrieval mode |
|------|---------------------|----------------|
| SCAN | WORKING only | Full scan or window |
| RECALL | EPISODIC, SEMANTIC, ALL | Similarity or condition |
| LOOKUP | PROCEDURAL, TOOLS, SEMANTIC | Exact key or pattern |
| LOAD | TOOLS only | Ranked selection |

**Why SCAN is WORKING only:**
SCAN means "give me current state." Only working memory has
a meaningful current state. Other memory types use RECALL.

**Why LOAD is TOOLS only:**
LOAD means "select tools into active use." It implies ranking
and token-budget awareness. Only the tool registry supports this.

**Why RECALL supports ALL:**
Cross-memory search is a common agent need:
"Tell me everything you know about this URL."
Without ALL, every cross-memory query requires a full REFLECT.
RECALL FROM ALL is the shorthand.

**Why LOOKUP does not support ALL:**
LOOKUP is exact-key or pattern-match. The semantics of
"exact match across all memory types" are ambiguous —
an exact key in SEMANTIC means something different from
an exact key in PROCEDURAL. Use RECALL FROM ALL instead.

---

## 6. Predicates

```bnf
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

condition       ::= simple_cond
                  | condition "AND" condition
                  | condition "OR" condition
                  | "(" condition ")"

simple_cond     ::= field comparator value

comparator      ::= "=" | "!=" | ">" | "<" | ">=" | "<="

window_pred     ::= "WINDOW" window_type

window_type     ::= "LAST" integer
                  | "LAST" duration
                  | "TOP" integer "BY" field
                  | "SINCE" key_expr
```

### Why WINDOW exists

**Problem:** Agents repeat the same query on every reasoning step:
"what have I been tracking recently?"
This is always a window over working memory, not a full scan.

**Solution:** WINDOW as a first-class predicate on SCAN FROM WORKING.

```sql
-- Last 10 items
SCAN FROM WORKING WINDOW LAST 10

-- Last 30 seconds
SCAN FROM WORKING WINDOW LAST 30s

-- Top 3 by attention weight
SCAN FROM WORKING WINDOW TOP 3 BY attention_weight

-- Everything since this incident started
SCAN FROM WORKING WINDOW SINCE event_id = "inc-001"
```

**Why WINDOW is WORKING only:**
Other memory types already have ORDER BY + LIMIT for recency.
WINDOW captures the concept of "current active context" which
only working memory represents. Episodic memory uses
ORDER BY time DESC LIMIT N for time-based access.

---

## 7. Write Statements

```bnf
write_stmt   ::= store_stmt | update_stmt

store_stmt   ::= "STORE" "INTO" memory_type payload
                 scope_mod?
                 namespace_mod?
                 ttl_mod?

update_stmt  ::= "UPDATE" "INTO" memory_type
                 "WHERE" condition
                 payload

payload      ::= "(" field_value_list ")"

field_value_list ::= field_value ("," field_value)*
field_value      ::= field "=" value
```

### Write Verb Semantics

| Verb | What it does |
|------|-------------|
| STORE INTO | Write new memory record |
| UPDATE INTO | Modify existing memory record |

### Why UPDATE INTO not UPDATE FROM

UPDATE writes new values. INTO expresses writing direction.
FROM would imply reading. INTO is consistent with STORE INTO.

### Why STORE INTO ALL is invalid

A write must target a specific memory type. The agent must
know whether it is storing a fact (SEMANTIC), an event
(EPISODIC), or a procedure (PROCEDURAL). Ambiguous writes
lead to ambiguous retrieval.

---

## 8. Forget Statement

```bnf
forget_stmt  ::= "FORGET" "FROM" memory_target predicate
```

**Rules:**
- predicate is required — FORGET FROM ALL with no WHERE is invalid
- FORGET FROM ALL WHERE condition clears matching records from all backends
- ADB decides the deletion strategy (soft delete, hard delete, compression)

**Why predicate is required:**
An accidental FORGET FROM ALL with no predicate would destroy
all agent memory. The grammar enforces intentionality.

**Why no decay parameters:**
v0.3 had `DECAY lambda=0.1 offset=7d` and
`STRATEGY soft_delete|compress|hard_delete`.
These are implementation details. ADB decides how to forget.
AQL only says what to forget.

---

## 9. Link Statement

```bnf
link_stmt    ::= "LINK" "FROM" memory_type "WHERE" condition
                 "TO" memory_type "WHERE" condition
                 type_mod?
                 weight_mod?

type_mod     ::= "TYPE" string_literal
weight_mod   ::= "WEIGHT" float
```

### Why LINK and dynamic ontology

**The procedural memory insight:**

A procedure stored without links to episodes is a definition,
not operational knowledge.

```
Without links:
  pattern = "OOMKilled"
  steps   = "scale memory"
  → just a cookbook recipe
  → no evidence it works
  → no record of when to apply it

With links:
  LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
    TO EPISODIC WHERE incident_id = "inc-001"
    TYPE "applied_to"
    WEIGHT 0.97
  → grounded in evidence
  → knows it worked (weight 0.97)
  → knows when it was applied
```

### Why TYPE is a dynamic string, not a fixed vocabulary

A static ontology (ADB defines "learned_from", "applies_to" etc)
is wrong for three reasons:

1. **Different agents have different relationship vocabularies.**
   A medical agent's ontology differs from a K8s agent's.
   An RTB agent invents "weakened_by_content_mismatch".
   A trading agent invents "correlated_during_volatility".

2. **Agents discover new relationship types through experience.**
   The LLM is better at naming relationships than we are.
   A K8s agent that discovers Friday OOM patterns invents
   TYPE "recurring_pattern_friday" — no one predicted this.

3. **One ontology for all agents is a design constraint, not a feature.**
   Constraining TYPE to a fixed set limits what agents can learn.

**Dynamic ontology rules:**
```
TYPE is any string_literal
ADB stores it, indexes it, returns it when queried
ADB never interprets what TYPE means
LLM defines and interprets TYPE values
Ontology is internal to each agent
```

**How ontology grows:**
```
Agent starts with no ontology
  ↓
Episode happens, LLM creates link with TYPE string
  ↓
More episodes, more links, more TYPE strings
  ↓
Agent's TYPE vocabulary becomes consistent through use
  ↓
Multi-agent: shared ontology emerges via SCOPE shared
  ↓
Community ontology grows bottom-up from experience
```

### LINK direction and cross-memory semantics

LINK FROM specifies the source memory type and record.
TO specifies the target memory type and record.
Links cross memory type boundaries — this is intentional.

Common cross-memory link patterns:
```
SEMANTIC → PROCEDURAL  (concept triggers a runbook)
PROCEDURAL → EPISODIC  (runbook was applied to this incident)
EPISODIC → PROCEDURAL  (incident contradicts a runbook)
SEMANTIC → EPISODIC    (concept was confirmed by this event)
PROCEDURAL → PROCEDURAL (one runbook supersedes another)
EPISODIC → EPISODIC    (incidents form a recurring pattern)
```

Every link pattern above was invented by an LLM through
experience. None were designed upfront.

---

## 10. WITH LINKS and FOLLOW LINKS

```bnf
with_links_mod ::= "WITH" "LINKS" links_target

links_target   ::= "ALL"
                 | "TYPE" string_literal

follow_mod     ::= "FOLLOW" "LINKS" "TYPE" string_literal
```

### WITH LINKS — inspecting evidence

WITH LINKS returns the link metadata for a record:
what types of links exist, how many, and to what.

```sql
-- What evidence does this runbook have?
LOOKUP FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  WITH LINKS ALL
  RETURN pattern_id, steps, link_type, count

-- Returns:
-- pattern_id: "oom-fix"
-- links:
--   type: "applied_to", count: 3, avg_weight: 0.93
--   type: "learned_from", count: 1
--   type: "superseded_by", count: 1
```

WITH LINKS TYPE narrows to a specific relationship:

```sql
-- Only show episodes where this runbook was applied
LOOKUP FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  WITH LINKS TYPE "applied_to"
  RETURN linked_episodes
```

### FOLLOW LINKS — traversing the knowledge graph

FOLLOW LINKS crosses memory type boundaries.
It starts from the source query results and follows
links of the specified TYPE to their targets.
The result set contains records from the TARGET memory type.

```sql
-- Start in SEMANTIC, follow "triggers" links to PROCEDURAL
RECALL FROM SEMANTIC WHERE concept = "kubernetes_oom"
  FOLLOW LINKS TYPE "triggers"
  RETURN procedures

-- This query does two things:
-- 1. Finds semantic records matching "kubernetes_oom"
-- 2. Follows all links of TYPE "triggers" from those records
-- 3. Returns the PROCEDURAL records at the other end
```

**Why FOLLOW LINKS is single-hop only (v0.5):**
Multi-hop traversal (DEPTH 2, DEPTH 3) is deferred to v0.6.
Single-hop covers 90% of agent queries. Multi-hop adds
complexity to the query planner and latency budget management.
The agent can chain multiple FOLLOW LINKS in a pipeline
for explicit multi-hop when needed.

### Why WITH EVIDENCE was removed

WITH EVIDENCE (proposed in v0.4 discussions) would have had
ADB auto-compute success_rate and last_outcome from link history.

Problem: ADB cannot compute these without knowing what TYPE
strings mean. "applied_to" with WEIGHT 0.97 means success to
the agent, but ADB has no basis for that interpretation.

Solution: FlowR computes derived metrics from link data.

```
Wrong (ADB interprets):
  LOOKUP FROM PROCEDURAL PATTERN $log_event
    WITH EVIDENCE
    RETURN success_rate    ← ADB cannot compute this

Right (FlowR computes):
  FlowR Step 1: AQL get procedure
  FlowR Step 2: AQL get linked episodes WITH LINKS TYPE "applied_to"
  FlowR Step 3: FlowR computes success_rate from outcome weights
  FlowR Step 4: Pass enriched context to LLM
```

---

## 11. Reflect Statement

```bnf
reflect_stmt    ::= "REFLECT" reflect_sources modifier* then_clause?

reflect_sources ::= reflect_source ("," reflect_source)*
                  | "FROM" "ALL" predicate?

reflect_source  ::= "FROM" memory_type predicate?

then_clause     ::= "THEN" write_stmt
```

### Why REFLECT

REFLECT is the context assembly statement. It pulls from multiple
memory types simultaneously and produces a unified context package.
ADB handles consistency checking internally.

FlowR receives the assembled context and shapes it before
passing to the LLM.

**REFLECT FROM ALL is valid:**
```sql
-- Assemble everything about this incident
REFLECT FROM ALL WHERE incident_id = {current}
```
Equivalent to REFLECT from every memory type with the same predicate.
Useful for comprehensive context assembly when the agent needs
everything it knows about a topic.

**THEN clause enables learn-on-read:**
```sql
-- Assemble context and store what was learned
REFLECT FROM EPISODIC WHERE incident_id = {current},
         FROM PROCEDURAL WHERE pattern_id = {matched}
  THEN STORE INTO PROCEDURAL (
    pattern    = "learned_pattern",
    steps      = "step1, step2",
    confidence = 0.6
  )
```

---

## 12. Pipeline Statement

```bnf
pipeline_stmt  ::= "PIPELINE" identifier?
                   "TIMEOUT" duration
                   pipeline_stage+

pipeline_stage ::= (read_stmt | reflect_stmt) "|"?
```

### Pipeline semantics

TIMEOUT is a hard constraint on total pipeline execution.
ADB allocates budget across stages proportionally.
If a stage cannot complete within budget it returns
partial results rather than blocking.

The pipe `|` operator chains stages sequentially.
Each stage's output is available to subsequent stages.
Variables from earlier stages can be referenced using
`{identifier}` syntax in later stages.

```sql
PIPELINE bid_decision TIMEOUT 80ms
  LOAD FROM TOOLS WHERE task = "bidding" LIMIT 3
  | LOOKUP FROM SEMANTIC KEY url = {url}
  | RECALL FROM EPISODIC WHERE url = {url} LIMIT 10
  | REFLECT FROM SEMANTIC, FROM EPISODIC, FROM PROCEDURAL
```

---

## 13. Aggregate and Having

```bnf
aggregate_mod  ::= "AGGREGATE" aggregate_func ("," aggregate_func)*

aggregate_func ::= agg_op "(" field ")" "AS" identifier
                 | "COUNT" "(" "*" ")" "AS" identifier

agg_op         ::= "COUNT" | "AVG" | "SUM" | "MIN" | "MAX"

having_mod     ::= "HAVING" condition
```

### Why AGGREGATE belongs in AQL

**The question:** Is aggregation intent or implementation?

**The answer:** Aggregation is intent when the LLM needs a summary
signal, not raw records.

```
"How many times did this pod OOMKill?"
→ the LLM needs a count, not 50 raw episode records
→ AGGREGATE COUNT(*) AS total is pure retrieval intent

"What was the average bid price?"
→ the LLM needs the average, not 200 raw bid records
→ AGGREGATE AVG(bid_price) AS avg_cpm is pure retrieval intent
```

**Why only COUNT, AVG, SUM, MIN, MAX:**
These five cover every summarisation an agent needs before
a decision. Computed fields (bid_price * 1.1) belong in FlowR.
GROUP BY deferred to v0.6 — needs careful semantics.

**HAVING requires AGGREGATE:**
HAVING filters on aggregate results. Invalid without AGGREGATE.

```sql
-- Which domains had more than 5 incidents?
RECALL FROM EPISODIC WHERE campaign = "summer"
  AGGREGATE COUNT(*) AS incidents
  HAVING incidents > 5
  RETURN url, incidents
  ORDER BY incidents DESC

-- Which strategies are underperforming?
RECALL FROM EPISODIC WHERE strategy = "tech_news_premium"
  AGGREGATE COUNT(*) AS uses, AVG(ctr) AS avg_ctr
  HAVING uses > 10 AND avg_ctr < 0.01
  RETURN url, uses, avg_ctr
```

---

## 14. All Modifiers

```bnf
modifier       ::= return_mod
                 | limit_mod
                 | weight_mod
                 | threshold_mod
                 | timeout_mod
                 | order_mod
                 | confidence_mod
                 | source_mod
                 | aggregate_mod
                 | having_mod
                 | with_links_mod
                 | follow_mod

return_mod     ::= "RETURN" field_list
field_list     ::= field ("," field)*

limit_mod      ::= "LIMIT" integer

weight_mod     ::= "WEIGHT" field

threshold_mod  ::= "THRESHOLD" float

timeout_mod    ::= "TIMEOUT" duration

order_mod      ::= "ORDER" "BY" field ("ASC" | "DESC")?

confidence_mod ::= "MIN_CONFIDENCE" float

source_mod     ::= "FROM" identifier_list
identifier_list ::= identifier ("," identifier)*

scope_mod      ::= "SCOPE" scope_value
scope_value    ::= "private" | "shared" | "cluster"

namespace_mod  ::= "NAMESPACE" string_literal

ttl_mod        ::= "TTL" duration

duration       ::= integer time_unit
time_unit      ::= "ms" | "s" | "m" | "h" | "d"
```

---

## 15. Primitives

```bnf
identifier     ::= [a-zA-Z_][a-zA-Z0-9_]*

field          ::= identifier ("." identifier)?

value          ::= string_literal
                 | integer
                 | float
                 | boolean
                 | embedding_ref
                 | variable

string_literal ::= '"' [^"]* '"'
integer        ::= [0-9]+
float          ::= [0-9]+ "." [0-9]+
boolean        ::= "true" | "false"
embedding_ref  ::= "$" identifier
variable       ::= "{" identifier "}"
```

---

## 16. Example Queries by Domain

### RTB Bidding — Complete Learning Cycle

```sql
-- Store incoming bid request into working memory
STORE INTO WORKING (
  bid_id = "br-20260330-8a3f",
  url = "https://techcrunch.com/2026/03/ai-agents",
  ad_slot = "300x250",
  floor_price = 2.50,
  device = "mobile",
  geo = "IE"
) TTL 500ms

-- Full bid decision pipeline
PIPELINE bid_decision TIMEOUT 80ms
  -- What tools do I have?
  LOAD FROM TOOLS WHERE task = "bidding"
    AND ad_format = "display"
    ORDER BY ranking DESC LIMIT 3
    RETURN tool_id, schema, token_cost

  -- What do we know about this page?
  | LOOKUP FROM SEMANTIC KEY url = {url}
      RETURN iab_category, brand_safety, avg_cpm

  -- If unknown URL, find similar content and follow to strategies
  | RECALL FROM SEMANTIC LIKE $page_embedding
      MIN_CONFIDENCE 0.8
      FOLLOW LINKS TYPE "triggers"
      RETURN procedures, link_type, weight

  -- Check triggered strategy's track record
  | LOOKUP FROM PROCEDURAL WHERE pattern_id = {matched}
      WITH LINKS TYPE "applied_to"
      RETURN pattern_id, steps, linked_episodes

  -- Aggregate past performance on similar URLs
  | RECALL FROM EPISODIC WHERE iab_category = {category}
      AGGREGATE AVG(cpm_paid) AS avg_cpm,
                AVG(ctr) AS avg_ctr,
                COUNT(*) AS sample_size
      HAVING sample_size > 5

  -- Assemble for LLM decision
  | REFLECT FROM SEMANTIC,
             FROM PROCEDURAL,
             FROM EPISODIC

-- After bid outcome: store episode
STORE INTO EPISODIC (
  bid_id = "br-20260330-8a3f",
  url = "https://techcrunch.com/2026/03/ai-agents",
  bid_price = 3.20,
  won = true,
  cpm_paid = 2.85,
  ctr = 0.034,
  viewability = 0.91
) SCOPE private NAMESPACE "pubcontext-bidder"

-- Build ontology: link strategy to outcome
LINK FROM PROCEDURAL WHERE pattern_id = "tech_news_premium"
  TO EPISODIC WHERE bid_id = "br-20260330-8a3f"
  TYPE "applied_to"
  WEIGHT 0.91

-- LLM discovers content mismatch on a different bid
LINK FROM EPISODIC WHERE bid_id = "bid-0087"
  TO PROCEDURAL WHERE pattern_id = "tech_news_premium"
  TYPE "weakened_by_content_mismatch"
  WEIGHT 0.22

-- LLM creates semantic-to-procedural trigger
LINK FROM SEMANTIC WHERE concept = "ai_enterprise_content"
  TO PROCEDURAL WHERE pattern_id = "tech_news_premium"
  TYPE "triggers"
  WEIGHT 0.85

-- Update strategy confidence from accumulated evidence
UPDATE INTO PROCEDURAL WHERE pattern_id = "tech_news_premium" (
  confidence = 0.72,
  success_count = 48,
  action = "bid floor + 22%"
)

-- Find underperforming strategies using HAVING
RECALL FROM EPISODIC WHERE strategy = "tech_news_premium"
  AGGREGATE COUNT(*) AS uses, AVG(ctr) AS avg_ctr
  HAVING uses > 10 AND avg_ctr < 0.01
  RETURN url, uses, avg_ctr

-- LLM marks strategy as ineffective on gadget content
LINK FROM PROCEDURAL WHERE pattern_id = "tech_news_premium"
  TO SEMANTIC WHERE concept = "gadget_reviews"
  TYPE "ineffective_on"
  WEIGHT 0.15

-- Clear working memory after bid
FORGET FROM WORKING WHERE bid_id = "br-20260330-8a3f"
```

### K8s Incident Response — Noise Filtering and Pattern Discovery

```sql
-- Alert storm: 7 events in 35 seconds
STORE INTO WORKING (
  event_id = "evt-003", source = "kubelet",
  alert = "OOMKilled", pod = "payments-api-7f8b9",
  namespace = "production"
) TTL 300s

-- Triage pipeline
PIPELINE incident_triage TIMEOUT 200ms
  -- What is happening right now?
  SCAN FROM WORKING WINDOW LAST 60s
    RETURN event_id, source, alert, pod

  -- Match against known patterns with evidence
  | LOOKUP FROM PROCEDURAL PATTERN $log_events
      THRESHOLD 0.7
      WITH LINKS TYPE "applied_to"
      RETURN pattern_id, steps, severity, linked_episodes

  -- Get noise filter (learned from past incidents)
  | RECALL FROM SEMANTIC LIKE $alert_combination
      FOLLOW LINKS TYPE "noise_filter_for"
      RETURN procedures, noise_alerts, root_signal

  -- How many times has this happened?
  | RECALL FROM EPISODIC WHERE pod = "payments-api"
      AND root_cause = "OOMKilled"
      AGGREGATE COUNT(*) AS past_incidents,
                AVG(resolution_time) AS avg_resolution
      HAVING past_incidents > 0

  -- What do we know about this service?
  | LOOKUP FROM SEMANTIC KEY concept = "payments-api"
      RETURN service_type, owner, memory_limit, dependencies

  -- Assemble for LLM
  | REFLECT FROM WORKING,
             FROM PROCEDURAL,
             FROM SEMANTIC,
             FROM EPISODIC

-- After resolution: store what happened
STORE INTO EPISODIC (
  incident_id = "inc-001",
  pod = "payments-api",
  root_cause = "OOMKilled",
  root_event = "evt-003",
  symptom_events = "evt-001,evt-002,evt-004,evt-005,evt-006,evt-007",
  action = "scaled memory to 768Mi",
  resolved = true,
  resolution_time = 180
)

-- Build ontology: runbook applied successfully
LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  TO EPISODIC WHERE incident_id = "inc-001"
  TYPE "applied_to"
  WEIGHT 0.95

-- Build ontology: root cause triggers runbook
LINK FROM SEMANTIC WHERE concept = "kubernetes_oom"
  TO PROCEDURAL WHERE pattern_id = "oom-fix"
  TYPE "triggers"
  WEIGHT 0.90

-- LLM learns noise filter from experience
STORE INTO SEMANTIC (
  concept = "oom_symptom_pattern",
  description = "When OOMKilled fires, expect: HighLatency,
                 5xxErrorRate, ConsumerLagHigh within 30s",
  noise_alerts = "HighLatency, 5xxErrorRate, ConsumerLagHigh,
                  ConnectionPoolExhausted, PodRestarting",
  root_signal = "OOMKilled"
)

LINK FROM SEMANTIC WHERE concept = "oom_symptom_pattern"
  TO PROCEDURAL WHERE pattern_id = "oom-fix"
  TYPE "noise_filter_for"

-- After 3 Friday incidents: LLM discovers recurrence
LINK FROM EPISODIC WHERE incident_id = "inc-001"
  TO EPISODIC WHERE incident_id = "inc-005"
  TYPE "recurring_pattern_friday"

-- LLM creates preventive runbook
STORE INTO PROCEDURAL (
  pattern_id = "friday-oom-prevention",
  trigger = "day = friday AND hour > 12 AND pod = payments-api",
  steps = "1. Scale memory to 1Gi before 13:00
           2. Monitor through batch window
           3. Scale back after 15:00",
  severity = "preventive",
  confidence = 0.80
)

-- New runbook supersedes reactive one on Fridays
LINK FROM PROCEDURAL WHERE pattern_id = "friday-oom-prevention"
  TO PROCEDURAL WHERE pattern_id = "oom-fix"
  TYPE "supersedes"

-- Prediction link: pattern predicts failure
LINK FROM SEMANTIC WHERE concept = "friday_batch_oom"
  TO PROCEDURAL WHERE pattern_id = "oom-fix"
  TYPE "predicts"
  WEIGHT 0.92
```

### Trading Agent — Strategy Evolution

```sql
-- Pipeline for trade decision
PIPELINE trade_decision TIMEOUT 10ms
  SCAN FROM WORKING WINDOW TOP 5 BY signal_strength
    RETURN instrument, signal, price, volume

  | LOOKUP FROM SEMANTIC KEY instrument = {instrument}
      RETURN sector, correlation_group, avg_daily_volume

  | RECALL FROM EPISODIC WHERE instrument = {instrument}
      AGGREGATE AVG(slippage) AS avg_slippage,
                COUNT(*) AS trade_count,
                AVG(pnl) AS avg_pnl
      HAVING trade_count > 10

  | LOOKUP FROM PROCEDURAL PATTERN $market_signal
      THRESHOLD 0.80
      WITH LINKS TYPE "profitable_in"
      RETURN strategy, parameters, linked_episodes

  | REFLECT FROM WORKING, FROM SEMANTIC,
             FROM EPISODIC, FROM PROCEDURAL

-- LLM discovers correlation between volatility and strategy
LINK FROM EPISODIC WHERE trade_id = "t-2026-0401"
  TO PROCEDURAL WHERE pattern_id = "mean-reversion-v3"
  TYPE "profitable_during_high_vol"
  WEIGHT 0.88

-- Strategy adapts its parameters
UPDATE INTO PROCEDURAL WHERE pattern_id = "mean-reversion-v3" (
  entry_threshold = 2.1,
  stop_loss = 0.015,
  confidence = 0.81
)
```

### Gaming NPC — Adaptive Behavior

```sql
-- NPC decision pipeline (16ms budget = one frame)
PIPELINE npc_decision TIMEOUT 16ms
  SCAN FROM WORKING WINDOW LAST 5
    RETURN player_action, npc_state, threat_level

  | RECALL FROM EPISODIC WHERE player_id = {player}
      AGGREGATE COUNT(*) AS encounter_count,
                AVG(damage_taken) AS avg_damage
      HAVING encounter_count > 3

  | LOOKUP FROM PROCEDURAL PATTERN $player_behavior
      THRESHOLD 0.75
      FOLLOW LINKS TYPE "effective_against"
      RETURN strategy, counter_moves

  | REFLECT FROM WORKING, FROM EPISODIC, FROM PROCEDURAL

-- NPC learns player always flanks left
LINK FROM EPISODIC WHERE encounter_id = "enc-042"
  TO SEMANTIC WHERE concept = "player_flanks_left"
  TYPE "evidence_for"
  WEIGHT 0.85

-- NPC creates counter-strategy
STORE INTO PROCEDURAL (
  pattern_id = "anti-left-flank",
  trigger = "player approaches from west",
  steps = "pre-position to east, bait approach, ambush",
  confidence = 0.70
)

LINK FROM SEMANTIC WHERE concept = "player_flanks_left"
  TO PROCEDURAL WHERE pattern_id = "anti-left-flank"
  TYPE "triggers"
  WEIGHT 0.85
```

### Multi-Agent — Shared Ontology

```sql
-- Agent 1 stores shared knowledge
STORE INTO SEMANTIC (
  concept = "k8s_oom_pattern",
  knowledge = "payments-api OOMs every Friday after batch job"
) SCOPE shared NAMESPACE "platform-agents"

-- Agent 2 reads it via same namespace
RECALL FROM SEMANTIC WHERE concept = "k8s_oom_pattern"
  SCOPE shared
  NAMESPACE "platform-agents"

-- Agent 2 adds its own link to the shared concept
LINK FROM SEMANTIC WHERE concept = "k8s_oom_pattern"
  TO PROCEDURAL WHERE pattern_id = "agent2-oom-fix"
  TYPE "handled_by"
  SCOPE shared
  NAMESPACE "platform-agents"

-- Shared ontology grows as agents contribute links
-- Each agent's TYPE vocabulary merges into community ontology
```

---

## 17. The Learning Loop — Complete AQL

The complete agent learning cycle in five steps:

```sql
-- Step 1: Agent acts on incident
-- (LLM decides, FlowR executes via tools)

-- Step 2: Store what happened
STORE INTO EPISODIC (
  incident_id = "inc-2026-03-28",
  pod = "payments-api",
  action = "scaled to 768Mi",
  resolved = true,
  resolution_time = 420
)

-- Step 3: Build the ontology
LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  TO EPISODIC WHERE incident_id = "inc-2026-03-28"
  TYPE "applied_to"
  WEIGHT 0.97

-- Step 4: Tune procedural variables
UPDATE INTO PROCEDURAL WHERE pattern_id = "oom-fix" (
  scale_to = "768Mi",
  confidence = 0.94,
  success_count = 8
)

-- Step 5: Clear working memory
FORGET FROM WORKING WHERE pod = "payments-api"
```

After this loop the agent is smarter. The ontology grew by one link.
The procedure variables were tuned. No retraining. No deployment.
Just AQL writes.

**The intelligence compounds:**
```
Episode 1:  ontology has 1 link, confidence 0.50
Episode 10: ontology has 15 links, confidence 0.75
Episode 50: ontology has 80+ links across 6 TYPE strings,
            confidence 0.94, noise filters learned,
            recurrence patterns discovered,
            preventive runbooks created by the LLM

Each episode makes the next one faster and more accurate.
The ontology IS the agent's accumulated intelligence.
```

**Periodic RL fine-tuning (ART) uses ADB data:**
```
Export episodes + link weights from ADB
  → structured reward signals for GRPO training
  → train improved base model (Mistral, Qwen, Llama)
  → deploy new checkpoint
  → better reasoning produces better links
  → better links produce better training data
  → cycle compounds
```

---

## 18. What AQL Does NOT Do

| Concern | Owner |
|---------|-------|
| Deciding next action | LLM |
| Executing actions | Agent Runtime / FlowR |
| Transforming retrieved data | FlowR |
| Computing success_rate from links | FlowR |
| Statistical feature computation | River / FlowR |
| RL fine-tuning | ART / external trainer |
| Decay algorithms | ADB |
| Storage tier placement | ADB |
| Conflict resolution | ADB |
| Compression | ADB |
| Concurrency control | ADB |
| Ontology interpretation | LLM |
| Procedural variable tuning logic | LLM + FlowR |

**The rule:** If it's about *how* memory works → ADB's job.
If it's about *what* the agent wants → AQL.
If it's about *transforming* data → FlowR.
If it's about *deciding* what to do → LLM.

---

## 19. Validation Rules

These are semantic rules the grammar cannot enforce.
The parser must validate these after parsing.

```
SCAN only valid on WORKING:
  SCAN FROM EPISODIC → error: "SCAN only valid on WORKING.
                               Use RECALL FROM EPISODIC."

LOAD only valid on TOOLS:
  LOAD FROM EPISODIC → error: "LOAD only valid on TOOLS."

WINDOW only valid on SCAN FROM WORKING:
  RECALL FROM EPISODIC WINDOW LAST 10 → error

FORGET predicate required:
  FORGET FROM ALL (no WHERE) → error:
    "FORGET FROM ALL requires WHERE clause."

STORE INTO ALL invalid:
  STORE INTO ALL (...) → error:
    "STORE requires specific memory type. ALL not valid for writes."

HAVING requires AGGREGATE:
  HAVING incidents > 5 without AGGREGATE → error

FOLLOW LINKS returns target memory type:
  RECALL FROM SEMANTIC ... FOLLOW LINKS TYPE "triggers"
  → result set contains PROCEDURAL records (the link targets)
  → this is correct, not an error

WITH LINKS on ALL:
  LOOKUP FROM ALL WITH LINKS → warn: supported but expensive

LINK requires specific memory types (not ALL):
  LINK FROM ALL → error: "LINK requires specific memory types."
```

---

## 20. Changes from v0.4

### Syntax changes

**FROM and INTO added as explicit direction markers.**
FROM on all reads and deletes.
INTO on all writes.
Old: `RECALL EPISODIC`, `STORE EPISODIC`
New: `RECALL FROM EPISODIC`, `STORE INTO EPISODIC`
Reason: unambiguous data flow direction.

**ALL added as memory target.**
`RECALL FROM ALL`, `FORGET FROM ALL`, `REFLECT FROM ALL`.
Reason: cross-memory search without requiring full REFLECT.

**UPDATE INTO added.**
Consistent with STORE INTO.

### New features

**WINDOW predicate on SCAN FROM WORKING.**
LAST N, LAST duration, TOP N BY field, SINCE key_expr.
Reason: formalises the most common agent query pattern.

**AGGREGATE modifier.**
COUNT, AVG, SUM, MIN, MAX with AS alias.
Reason: agents need summary signals not raw records.

**HAVING modifier.**
Filters on aggregate results.
Reason: "which had more than N" is a real agent query.

**WITH LINKS modifier.**
WITH LINKS ALL — return all link metadata.
WITH LINKS TYPE string — return links of specific type.
Reason: procedural memory without evidence is incomplete.

**FOLLOW LINKS modifier.**
Single-hop graph traversal across memory types.
Reason: "what does this concept trigger" requires
crossing from SEMANTIC to PROCEDURAL.

**Dynamic ontology model.**
LINK TYPE is any string_literal. ADB stores without interpretation.
Reason: agents must discover their own relationship vocabulary.

### Removed from v0.4

**Implicit memory type (verb without FROM).**
Removed: non-deterministic behaviour is hard to test.
Use RECALL FROM ALL for cross-memory search.

**WITH EVIDENCE.**
Removed: ADB cannot compute success metrics without
knowing TYPE semantics. FlowR computes these.

### Grammar size

| Version | Rules | Modifiers | Notes |
|---------|-------|-----------|-------|
| v0.1 | 25 | 10 | Baseline |
| v0.2 | 28 | 12 | Tool registry |
| v0.3 | 45 | 22 | Over-engineered |
| v0.4 | 30 | 8 | Clean simplification |
| v0.5 | 40 | 12 | FROM/INTO + WINDOW + AGGREGATE + ontology |

v0.5 is 33% smaller than v0.3 while being significantly
more capable.

---

## 21. The Readability Test

Every query must pass: a non-expert can read it and know what it does.

```sql
SCAN FROM WORKING WINDOW LAST 10
```
✓ Scan from working memory, last 10 items

```sql
RECALL FROM EPISODIC WHERE pod = "payments-api"
  AGGREGATE COUNT(*) AS total_incidents
```
✓ Recall from episodic memory about this pod, count the incidents

```sql
STORE INTO SEMANTIC (concept = "oom_pattern", knowledge = "OOMs on Fridays")
  SCOPE shared NAMESPACE "platform-agents"
```
✓ Store into semantic memory, shared across the platform agents

```sql
LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  TO EPISODIC WHERE incident_id = "inc-001"
  TYPE "applied_to" WEIGHT 0.97
```
✓ Link this procedure to this episode, type applied_to, weight 0.97

```sql
FORGET FROM ALL WHERE temp = "true"
```
✓ Forget from all memory, anything marked temporary

```sql
RECALL FROM SEMANTIC LIKE $page_embedding
  FOLLOW LINKS TYPE "triggers"
  RETURN procedures
```
✓ Recall similar semantic records, follow trigger links, return procedures

```sql
LOOKUP FROM PROCEDURAL WHERE pattern_id = "oom-fix"
  WITH LINKS ALL
```
✓ Look up this procedure, show all its link relationships

```sql
RECALL FROM EPISODIC WHERE strategy = "tech_news_premium"
  AGGREGATE COUNT(*) AS uses, AVG(ctr) AS avg_ctr
  HAVING uses > 10 AND avg_ctr < 0.01
```
✓ Recall episodes for this strategy, count uses and average CTR,
  only where used more than 10 times with CTR below 1%

All pass. v0.5 is ready.

---

## 22. Open Questions for v0.6

1. **GROUP BY** — group before aggregating
   ```sql
   RECALL FROM EPISODIC WHERE campaign = "summer"
     GROUP BY url
     AGGREGATE COUNT(*) AS impressions
   ```

2. **WINDOW on EPISODIC** — explicit time window
   ```sql
   RECALL FROM EPISODIC WHERE pod = "payments-api"
     WINDOW LAST 7d
   ```

3. **FOLLOW LINKS depth** — multi-hop traversal
   ```sql
   RECALL FROM SEMANTIC WHERE concept = "kubernetes_oom"
     FOLLOW LINKS TYPE "triggers" DEPTH 2
   ```

4. **Streaming RECALL** — for very long histories
   ```sql
   RECALL FROM EPISODIC WHERE agent_id = "agent-001"
     STREAM BATCH 100
   ```

5. **MERGE statement** — explicit cross-memory join
   ```sql
   MERGE FROM EPISODIC WHERE url = {url}
     WITH SEMANTIC KEY url = {url}
     ON concept = url
   ```

6. **LINK WEIGHT update** — modify existing link weight
   ```sql
   UPDATE LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
     TO EPISODIC WHERE incident_id = "inc-001"
     WEIGHT 0.99
   ```

---

## 23. Stack Architecture

```
Agent Container (one container, zero network hops)
  │
  ├── FlowR Process
  │     CNCF Serverless Workflow runtime
  │     Receives world events (bid requests, alerts, signals)
  │     Orchestrates AQL queries against ADB
  │     Runs River/statistical models for numeric features
  │     Transforms and shapes context for LLM
  │     Acts on LLM decisions via tools
  │     Writes outcomes back to ADB via AQL
  │     Microsecond triggering via Tokio tasks
  │
  ├── ADB Process (sidecar, same container)
  │     ├── Working    DashMap              < 1ms
  │     ├── Tools      DashMap + ranking    < 2ms
  │     ├── Procedural petgraph + links     < 5ms
  │     ├── Semantic   usearch (vector)     < 20ms
  │     └── Episodic   Arrow + DataFusion   < 50ms
  │
  │     AQL Parser + Query Planner
  │     Link/Ontology Index
  │     Arrow Flight IPC (transport)
  │     MCP Server interface
  │
  ├── Local LLM (optional)
  │     Mistral / Qwen / Llama
  │     Runs in same container or adjacent
  │     Periodically improved via ART/GRPO
  │     using episodes + link weights from ADB
  │
  └── Agent Execution Loop
        Event → FlowR → AQL → ADB → context
        → FlowR transforms → LLM reasons → decision
        → FlowR executes → AQL stores outcome
        → AQL builds ontology links
        → AQL tunes procedural variables
        → Agent is smarter for next event
```

**Where agent learning lives:**

```
Ontology (LINK network in ADB)
  → agent's theory of how things relate
  → grows with every LINK statement
  → TYPE strings defined by the LLM
  → queryable via WITH LINKS and FOLLOW LINKS
  → auditable, reversible, inspectable

Procedural variables (records in ADB)
  → agent's tuned execution parameters
  → updated via UPDATE INTO PROCEDURAL
  → confidence, thresholds, timing, scale values
  → each UPDATE makes the agent more precise

Model weights (periodic, offline)
  → improved via ART/GRPO using ADB data
  → better reasoning over the same ontology
  → deployed as new checkpoint
  → compounds with ontology growth

Together:
  → complete agent intelligence stack
  → real-time learning via ontology (milliseconds)
  → periodic model improvement via RL (hours)
  → no retraining needed for domain knowledge
  → observable and auditable at every level
```

---

*AQL v0.5 — Complete Specification with Design Rationale*
*github.com/srirammails/AQL*
*March 2026 · Sriram Reddy*
