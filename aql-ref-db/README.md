# aql-ref-db

**Reference implementation of ADB — Agent Database**

[![PyPI](https://img.shields.io/pypi/v/aql-ref-db.svg)](https://pypi.org/project/aql-ref-db/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## Overview

ADB is an in-memory reference implementation of the Agent Database.
Like SQLite for SQL, it proves the concept and lets you experiment.

Five memory backends:
- **Working** — dict-based, supports SCAN ALL, TTL expiration
- **Tools** — tool registry with rankings that improve over time
- **Procedural** — networkx graph for patterns and procedures
- **Semantic** — numpy cosine similarity for concept recall
- **Episodic** — pandas DataFrame for time-series episodes

## Installation

```bash
pip install aql-ref-db

# With all backends (pandas, numpy, networkx)
pip install aql-ref-db[full]
```

## Quick Start

```python
from aql_db import ADB

db = ADB()

# Store a memory
db.execute('''
    STORE EPISODIC (
        incident_id = "inc-001",
        pod = "payments-api",
        action = "scaled memory"
    )
''')

# Recall memories
result = db.execute('''
    RECALL EPISODIC WHERE pod = "payments-api"
    ORDER BY time DESC
    LIMIT 5
''')

for record in result["records"]:
    print(record["data"])
```

## Full Pipeline Example

```python
from aql_db import ADB

db = ADB()

# Seed knowledge
db.execute('''
    STORE SEMANTIC (
        concept = "payments-api",
        knowledge = "critical service, handles all payment processing"
    )
''')

db.execute('''
    STORE PROCEDURAL (
        pattern_id = "oom-kill-001",
        pattern = "OOMKilled memory limit exceeded",
        steps = "check limits,scale memory,notify team"
    )
''')

# Run a pipeline
result = db.execute('''
    PIPELINE incident TIMEOUT 200ms
    LOOKUP PROCEDURAL PATTERN $log_event THRESHOLD 0.5
    | RECALL EPISODIC WHERE pod = "payments-api" LIMIT 5
''')

print(result)
```

## REFLECT — Context Assembly

```python
result = db.execute('''
    REFLECT incident_id = {current}
    INCLUDE EPISODIC
    INCLUDE SEMANTIC
    INCLUDE WORKING
''')

# Get LLM-ready context string
print(result["llm_context"])
```

## API

### `ADB`

```python
class ADB:
    def execute(self, query: str) -> dict:
        """Execute AQL query, return results."""

    def get_backend(self, name: str) -> BaseBackend:
        """Get a backend directly."""

    def reset(self):
        """Reset all backends to empty state."""
```

### Result Format

All backends return records in standard format:

```python
{
    "id": "unique-id",
    "memory_type": "EPISODIC",
    "data": { ... },
    "metadata": {
        "created_at": 1234567890.0,
        "accessed_at": 1234567890.0,
        "scope": "private",
        "namespace": None,
    }
}
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest tests/ -v
```

## License

Apache 2.0

---

*AQL Reference Database v0.1 · March 2026*
