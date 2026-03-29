# ADB · Agent Database

**The unified in-memory memory layer every AI agent has been missing.**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/AQL%20spec-v0.2-teal.svg)](AQL_Grammar_Specification.md)
[![Status](https://img.shields.io/badge/status-specification%20%2B%20reference%20implementation-orange.svg)]()

---

## The Problem

Every team building AI agents hits the same wall.

Agents need to remember across four dimensions simultaneously:

- **What is happening now** — active task state, loaded tools, current context
- **What happened before** — past decisions, outcomes, episode history
- **What they know about the world** — domain knowledge, entity relationships, concepts
- **How to do things** — runbooks, procedures, action patterns

No single database handles all four. Every agent framework today duct-tapes Pinecone, Redis, Neo4j, and Postgres together and calls it memory. The agent code becomes tangled with storage logic. There is no standard query pattern. Every team reinvents this differently.

## The Solution

ADB is a unified in-memory multimodal database designed for agents, not humans. It implements all four cognitive memory types in a single process, with a single query interface: **AQL — Agent Query Language**.

```
Agent Container
  ├── ADB Process
  │     ├── Working Memory    — DashMap          — < 1ms
  │     ├── Procedural Memory — petgraph          — < 5ms
  │     ├── Semantic Memory   — usearch           — < 20ms
  │     ├── Episodic Memory   — Arrow + DataFusion — < 50ms
  │     └── Tool Registry     — ranked, dynamic   — < 5ms
  │
  │     AQL Query Planner
  │     Arrow Flight IPC
  │
  ├── Agent Runtime
  │     AQL query → assembled context → LLM → decision → write-back
```

ADB runs as a sidecar process in the same isolated container as the agent. No network hops. No cross-contamination between agents. Each agent is cognitively sovereign.

## AQL — Agent Query Language

AQL is an open specification for querying agent memory. Its verbs encode agentic intent, not just predicates.

```sql
-- What do I know about this URL?
LOOKUP SEMANTIC KEY url="sports.example.com"
  RETURN context, categories, historical_signal

-- What happened last time?
RECALL EPISODIC WHERE url="sports.example.com"
  RETURN bid_price, impression, click, conversion
  ORDER BY time DESC LIMIT 10

-- Does this log event match a known pattern?
LOOKUP PROCEDURAL PATTERN $log_event
  THRESHOLD 0.85
  RETURN pattern_id, severity, action_steps

-- Assemble full context for LLM decision
REFLECT incident_id={current}
  INCLUDE EPISODIC WHERE incident_id={current}
  INCLUDE PROCEDURAL WHERE pattern_id={matched}
  INCLUDE WORKING

-- Full pipeline with hard timeout
PIPELINE bid_decision TIMEOUT 80ms
  LOOKUP SEMANTIC KEY url={url}
  | RECALL SEMANTIC LIKE $page_context FROM creatives LIMIT 5
  | RECALL EPISODIC WHERE url={url} LIMIT 10
  | REFLECT url={url}

-- Agent learns from outcome
STORE EPISODIC (
  incident_id = "inc-2026-03-28",
  action      = "scaled memory to 512Mi",
  resolved    = true
)

-- LLM rewrites agent strategy at runtime
UPDATE PROCEDURAL WHERE pattern_id="oom-kill-001" (
  steps      = [new_step_1, new_step_2],
  confidence = 0.97
)
```

**AQL's core design principle:** AQL retrieves and stores. It does not decide. The LLM owns what happens next.

→ [Full AQL v0.2 Specification](AQL_Grammar_Specification.md)

## Key Design Decisions

**Four memory types — not arbitrary, cognitively grounded**

The taxonomy is derived from cognitive science (Tulving, Baddeley) and independently validated through engineering necessity. Every agent building on multiple backends converges on these four types.

| Memory Type | Storage Backend | Retrieval Mode | Latency |
|-------------|----------------|----------------|---------|
| Working | DashMap | Direct scan | < 1ms |
| Tools | Ranked registry | Task relevance | < 2ms |
| Procedural | petgraph | Pattern / goal match | < 5ms |
| Semantic | usearch | Vector similarity | < 20ms |
| Episodic | Arrow + DataFusion | Context / time | < 50ms |

**Bidirectional memory — agents that self-improve**

The LLM can push knowledge into semantic memory and rewrite procedural memory at runtime. Agents get smarter through use — no retraining, no redeployment.

**ADB as MCP server — instant ecosystem reach**

ADB exposes itself as a Model Context Protocol (MCP) server. Any MCP-compatible LLM — Claude, GPT, any future runtime — can query agent memory directly via AQL. No custom integration required.

**Apache Arrow throughout**

ADB stores data in Apache Arrow columnar format internally. Query results are returned via Arrow Flight IPC — zero-copy transport between ADB and the agent runtime.

**CNCF aligned**

FlowR — the companion workflow runtime built on CNCF Serverless Workflow specification — provides long-running execution for agents. FlowR gives agents time. ADB gives agents memory.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Agent Container                    │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │                 ADB Process                  │   │
│  │                                              │   │
│  │  Working      Procedural  Semantic  Episodic │   │
│  │  DashMap      petgraph    usearch   Arrow    │   │
│  │                                              │   │
│  │  Tool Registry — ranked, dynamic loading     │   │
│  │                                              │   │
│  │  AQL Query Planner                           │   │
│  │  Arrow Flight IPC                            │   │
│  └──────────────────────────────────────────────┘   │
│                       ↕ Unix socket                 │
│  ┌──────────────────────────────────────────────┐   │
│  │              Agent Runtime                   │   │
│  │                                              │   │
│  │  AQL query → assembled context → LLM         │   │
│  │  LLM decision → action → write-back to ADB  │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

## Use Cases

**Real-Time Bidding** — Agent evaluates bid requests in < 80ms using a pipeline across semantic, episodic, and procedural memory.

**Kubernetes Log Analysis** — Agent matches log events to known runbooks, checks incident history, acts, and learns from outcomes. Unresolved incidents are stored as new procedural patterns automatically.

**Compliance and Audit** — Every agent decision is stored with full context — what the agent knew, what procedure it followed, what outcome resulted. Fully queryable by regulators and auditors.

## Repository Structure

```
AQL/
├── LICENSE                       — Apache 2.0
├── README.md                     — this file
├── AQL_Grammar_Specification.md  — AQL v0.2 formal BNF grammar specification
├── reference/                    — reference implementation (Rust) [planned]
└── docs/                         — architecture, design docs [planned]
```

## Status

| Component | Status |
|-----------|--------|
| AQL v0.2 spec | Published |
| Reference implementation | In progress |
| ADB MCP server | Planned — week of March 31 |
| FlowR integration | Planned |
| Arrow Flight IPC | Planned |
| CNCF submission | Planned — Q3 2026 |

## What's New in v0.2

- **Working memory as assembly layer** — not just active state, but the complete prepared context package
- **Tool Registry as fifth memory type** — schemas, rankings, token costs queryable via AQL
- **LOAD verb** — `LOAD TOOLS WHERE relevance > 0.8` selects tools into working memory

## Roadmap — Open Questions

1. **Embedding literals** — how does a caller pass a live embedding vector into a `LIKE` predicate?
2. **Streaming RECALL** — should episodic recall support streaming for long histories?
3. **Conditional STORE** — `IF NOT EXISTS` semantics
4. **Memory versioning** — optimistic concurrency control for `UPDATE`
5. **Cross-agent memory** — namespace for shared vs. private memory
6. **Tool ranking updates** — should STORE TOOLS auto-update rankings?
7. **Tool eviction** — when working memory is full, which tools get evicted first?

## Contributing

AQL is an open specification. Contributions to the grammar, use cases, and reference implementation are welcome.

- Open an issue to discuss a grammar change or new use case
- Submit a PR against the spec with your proposed BNF addition
- Share your agent memory scenarios — real use cases drive the grammar

## Related Work

| Project | What it does | How ADB differs |
|---------|-------------|-----------------|
| Mem0 | Personal assistant memory | Single modality, no query language |
| Zep | Conversation memory | Episodic only, no unified interface |
| Chroma / Pinecone | Vector search | Semantic only, no cognitive taxonomy |
| mnemory | MCP memory server | Facts about users, not agent operations |
| LangChain Memory | Conversation buffer | No persistent multimodal store |

## License

Apache 2.0 — see [LICENSE](LICENSE)

AQL specification and ADB architecture are free to use, implement, and build upon.

---

*AQL v0.2 · March 2026 · Sriram Reddy*

*The unified memory layer every AI agent has been missing.*
