# ADB · Agent Database

**The unified in-memory memory layer every AI agent has been missing.**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/AQL%20spec-v0.3-teal.svg)](spec/AQL_SPEC_v0.3.md)
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
-- Load relevant tools for current task
LOAD TOOLS WHERE relevance > 0.8
  ORDER BY ranking DESC
  LIMIT 3
  RETURN tool_id, schema, token_cost

-- Recall with quality filter and storage tier
RECALL EPISODIC WHERE pod="payments-api"
  MIN_CONFIDENCE 0.7
  TIER hot, warm
  RETURN incident_id, action, resolved, confidence
  ORDER BY time DESC LIMIT 5

-- Store with strength, scope, and namespace
STORE SEMANTIC (
  concept   = "k8s_oom_pattern",
  knowledge = "payments-api OOMs every Friday after batch job"
)
  STRENGTH 0.87
  TTL 90d
  SCOPE shared
  NAMESPACE cluster="platform-agents"
  IF NOT EXISTS

-- Forget with decay model
FORGET EPISODIC
  WHERE activation < 0.3
  DECAY lambda=0.5 offset=0.1
  STRATEGY soft_delete

-- Multi-agent REFLECT with consistency checks
REFLECT incident_id={current}
  INCLUDE EPISODIC WHERE incident_id={current}
  INCLUDE PROCEDURAL WHERE pattern_id={matched}
  INCLUDE WORKING
  CHECK temporal, factual, logical
  RESOLVE CONFLICTS = merge
  NAMESPACE cluster="platform-agents"

-- Full pipeline with hard timeout
PIPELINE bid_decision TIMEOUT 80ms
  LOAD TOOLS WHERE task="bid_evaluation" LIMIT 3
  | LOOKUP SEMANTIC KEY url={url}
      MIN_CONFIDENCE 0.8
      TIER hot
  | RECALL EPISODIC WHERE url={url}
      TIER hot, warm
      LIMIT 10
  | REFLECT url={url}
      INCLUDE EPISODIC
      INCLUDE SEMANTIC
      CHECK temporal, factual
```

**AQL's core design principle:** AQL retrieves and stores. It does not decide. The LLM owns what happens next.

→ [Full AQL v0.3 Specification](spec/AQL_SPEC_v0.3.md)

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
├── spec/
│   ├── AQL_SPEC_v0.2.md          — AQL v0.2 specification
│   └── AQL_SPEC_v0.3.md          — AQL v0.3 specification (current)
├── grammar/
│   └── aql.pest                  — PEG grammar for pest parser
├── crates/
│   └── aql-parser/               — Rust parser crate [in progress]
│       ├── src/
│       └── tests/
└── docs/                         — architecture, design docs [planned]
```

## Status

| Component | Status |
|-----------|--------|
| AQL v0.3 spec | Published |
| PEG grammar (pest) | Published |
| aql-parser crate | In progress |
| ADB reference implementation | Planned |
| ADB MCP server | Planned — week of March 31 |
| FlowR integration | Planned |
| Arrow Flight IPC | Planned |
| CNCF submission | Planned — Q3 2026 |

## What's New in v0.3

- **FORGET statement** — explicit memory lifecycle with decay models (`DECAY lambda=0.5 offset=0.1`) and strategies (`soft_delete`, `compress`, `hard_delete`, `archive`)
- **LOAD TOOLS / UPDATE TOOLS** — first-class verbs for tool registry
- **STRENGTH modifier** — importance weighting on STORE (0.0 - 1.0)
- **TTL modifier** — time-to-live for automatic expiration
- **SCOPE modifier** — `private | shared | cluster` for multi-agent isolation
- **NAMESPACE modifier** — agent identity for memory isolation
- **MIN_CONFIDENCE modifier** — quality filter on RECALL
- **TIER modifier** — `hot | warm | cold | archive` storage tier selection
- **IF NOT EXISTS** — conditional STORE
- **VERSION / LOCK** — optimistic and pessimistic concurrency control
- **CHECK clause** — reflection consistency dimensions (`temporal`, `factual`, `logical`, `causal`)
- **RESOLVE CONFLICTS** — conflict resolution strategies (`merge`, `replace`, `flag`, `newest`, `highest_confidence`)

## Roadmap — Open Questions

1. **Embedding literals** — how does a caller pass a live embedding vector into a `LIKE` predicate?
2. **Streaming RECALL** — should episodic recall support streaming for long histories?
3. **Tool eviction** — when working memory is full, which tools get evicted first?
4. **Cross-cluster memory** — federation across multiple ADB instances?
5. **Encryption** — per-agent encryption keys for regulated industries?
6. **Compression strategies** — what summarization algorithm for `STRATEGY compress`?

## Contributing

AQL is an open specification. Contributions to the grammar, use cases, and reference implementation are welcome.

- Open an issue to discuss a grammar change or new use case
- Submit a PR against the spec with your proposed BNF addition
- Share your agent memory scenarios — real use cases drive the grammar

## Related Work

| Project | What it does | How ADB differs |
|---------|-------------|-----------------|
| Mem0 | Personal assistant memory | Single modality, no query language |
| MemoryBear | Personal assistant memory, graph-based | 5 external services required, no query language, no cognitive taxonomy — ADB: one process, AQL, agent-native |
| Zep | Conversation memory | Episodic only, no unified interface |
| Chroma / Pinecone | Vector search | Semantic only, no cognitive taxonomy |
| mnemory | MCP memory server | Facts about users, not agent operations |
| LangChain Memory | Conversation buffer | No persistent multimodal store |

## License

Apache 2.0 — see [LICENSE](LICENSE)

AQL specification and ADB architecture are free to use, implement, and build upon.

---

*AQL v0.1 · March 2026 · Initial specification*
*AQL v0.2 · March 2026 · Working memory as assembly layer, Tool Registry*
*AQL v0.3 · March 2026 · FORGET, DECAY, SCOPE, NAMESPACE, CHECK, RESOLVE*

*Sriram Reddy · The unified memory layer every AI agent has been missing.*
