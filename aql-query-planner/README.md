# aql-query-planner

**Query planner for AQL — converts ExecutionPlan to TaskList**

[![PyPI](https://img.shields.io/pypi/v/aql-query-planner.svg)](https://pypi.org/project/aql-query-planner/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## Overview

The query planner takes an `ExecutionPlan` from `aql-parser` and produces a `TaskList` for the data layer. It handles:

1. **Routing** — Which backend to call (working, semantic, episodic, procedural, tools)
2. **Scheduling** — Task dependencies and execution order
3. **Budget allocation** — Time budget distribution for PIPELINE queries

## Installation

```bash
pip install aql-query-planner
```

## Quick Start

```python
from aql_parser import parse
from aql_planner import plan

# Parse and plan a simple query
execution_plan = parse('RECALL EPISODIC WHERE pod = "payments" LIMIT 5')
task_list = plan(execution_plan)

print(task_list.tasks[0].backend)    # "episodic"
print(task_list.tasks[0].task_type)  # TaskType.RECALL
```

## Routing Rules

| Verb | Memory Type | Backend |
|------|-------------|---------|
| LOOKUP | SEMANTIC | semantic |
| LOOKUP | PROCEDURAL | procedural |
| LOOKUP | TOOLS | tools |
| RECALL | EPISODIC | episodic |
| RECALL | SEMANTIC | semantic |
| SCAN | WORKING | working |
| LOAD | TOOLS | tools |
| STORE | * | (matches memory type) |
| REFLECT | — | merger |

## Latency Estimates

Default estimates used for budget allocation:

| Backend | Estimated Latency |
|---------|------------------|
| working | 1ms |
| tools | 2ms |
| procedural | 5ms |
| semantic | 20ms |
| episodic | 50ms |
| merger | 5ms |

## Pipeline Planning

```python
from aql_parser import parse
from aql_planner import plan

execution_plan = parse('''
    PIPELINE bid_decision TIMEOUT 80ms
    LOAD TOOLS WHERE task = "bidding" LIMIT 3
    | LOOKUP SEMANTIC KEY url = {url}
    | RECALL EPISODIC WHERE url = {url} LIMIT 10
''')

task_list = plan(execution_plan)

print(f"Tasks: {len(task_list.tasks)}")           # 3
print(f"Budget: {task_list.total_budget_ms}ms")   # 80
print(f"Pipeline: {task_list.pipeline_name}")     # "bid_decision"
```

## API

### `plan(execution_plan) -> TaskList`

Main entry point. Routes any AQL query to the appropriate planning strategy.

### `Task`

```python
@dataclass
class Task:
    id: str                    # Unique task identifier
    task_type: TaskType        # LOOKUP, RECALL, SCAN, etc.
    backend: str               # Target backend name
    predicate: Optional[dict]  # Search conditions
    payload: Optional[dict]    # Data for STORE/UPDATE
    modifiers: dict            # LIMIT, ORDER BY, etc.
    depends_on: list[str]      # Task IDs that must complete first
    budget_ms: int             # Time budget
    scope: str                 # "private" | "shared" | "cluster"
    namespace: Optional[str]   # Agent identity
```

### `TaskList`

```python
@dataclass
class TaskList:
    tasks: list[Task]          # Ordered tasks
    total_budget_ms: int       # Total time budget
    pipeline_name: Optional[str]
    merge_strategy: str        # "sequential" | "reflect"
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

*AQL Query Planner v0.1 · March 2026*
