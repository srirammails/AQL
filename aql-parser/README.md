# aql-parser

**Parser for AQL — Agent Query Language**

[![PyPI](https://img.shields.io/pypi/v/aql-parser.svg)](https://pypi.org/project/aql-parser/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

AQL is the query language for ADB (Agent Database) — a unified in-memory multimodal database for AI agents.

## Installation

```bash
pip install aql-parser
```

## Quick Start

```python
from aql_parser import parse, Verb, MemoryType

# Parse a query
plan = parse('RECALL EPISODIC WHERE pod = "payments" LIMIT 5')

print(plan.verb)        # Verb.RECALL
print(plan.memory_type) # MemoryType.EPISODIC

# Access modifiers
from aql_parser import LimitMod
limit = plan.get_modifier(LimitMod)
print(limit.value)      # 5
```

## AQL Examples

```sql
-- Load relevant tools
LOAD TOOLS WHERE relevance > 0.8
  ORDER BY ranking DESC
  LIMIT 3

-- Recall with quality filter
RECALL EPISODIC WHERE pod = "payments-api"
  MIN_CONFIDENCE 0.7
  ORDER BY time DESC
  LIMIT 5

-- Store shared knowledge
STORE SEMANTIC (
  concept = "k8s_oom_pattern",
  knowledge = "payments-api OOMs every Friday"
)
  SCOPE shared
  NAMESPACE "platform-agents"

-- Forget old memories
FORGET EPISODIC WHERE last_accessed > 30

-- Assemble context for LLM
REFLECT incident_id = {current}
  INCLUDE EPISODIC
  INCLUDE PROCEDURAL
  INCLUDE WORKING

-- Pipeline with timeout
PIPELINE bid_decision TIMEOUT 80ms
  LOAD TOOLS WHERE task = "bidding" LIMIT 3
  | LOOKUP SEMANTIC KEY url = {url}
  | RECALL EPISODIC WHERE url = {url} LIMIT 10
```

## API

### `parse(query: str) -> ExecutionPlan`

Parse an AQL query string and return an `ExecutionPlan`.

```python
from aql_parser import parse, AqlError

try:
    plan = parse(query)
except AqlError as e:
    print(f"Parse error: {e}")
```

### `ExecutionPlan`

The parsed query as a dataclass:

```python
@dataclass
class ExecutionPlan:
    verb: Verb                           # LOOKUP, RECALL, SCAN, LOAD, STORE, etc.
    memory_type: Optional[MemoryType]    # EPISODIC, SEMANTIC, PROCEDURAL, WORKING, TOOLS
    predicate: Optional[Predicate]       # WHERE, KEY, LIKE, PATTERN, ALL
    payload: Optional[Payload]           # For STORE/UPDATE
    modifiers: List[Any]                 # LIMIT, RETURN, ORDER BY, etc.

    # REFLECT specific
    sources: List[ReflectSource]
    then_stmt: Optional[ExecutionPlan]

    # PIPELINE specific
    pipeline_name: Optional[str]
    timeout: Optional[TimeoutMod]
    stages: List[ExecutionPlan]

    # Multi-agent
    scope: Optional[ScopeMod]            # private | shared | cluster
    namespace: Optional[NamespaceMod]
    ttl: Optional[TtlMod]

    def get_modifier(self, mod_type: type) -> Optional[Any]:
        """Get a modifier by type."""

    def to_dict(self) -> dict:
        """Convert to dictionary for serialization."""
```

### Enums

```python
class Verb(str, Enum):
    LOOKUP, RECALL, SCAN, LOAD, STORE, UPDATE, LINK, FORGET, REFLECT, PIPELINE

class MemoryType(str, Enum):
    EPISODIC, SEMANTIC, PROCEDURAL, WORKING, TOOLS

class ScopeValue(str, Enum):
    PRIVATE, SHARED, CLUSTER
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest tests/ -v

# Run with coverage
pytest tests/ --cov=aql_parser --cov-report=term-missing
```

## Specification

See the full AQL v0.4 specification: [AQL_SPEC_v0.4.md](https://github.com/srirammails/AQL/blob/main/spec/AQL_SPEC_v0.4.md)

## License

Apache 2.0

---

*AQL v0.4 · March 2026 · Sriram Reddy*
