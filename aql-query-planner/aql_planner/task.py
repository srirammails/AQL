# aql_planner/task.py
"""Core types for AQL query planning."""

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional
import uuid


class TaskType(str, Enum):
    """Operation types for tasks."""
    LOOKUP = "LOOKUP"
    RECALL = "RECALL"
    SCAN = "SCAN"
    LOAD = "LOAD"
    STORE = "STORE"
    UPDATE = "UPDATE"
    FORGET = "FORGET"
    LINK = "LINK"
    REFLECT = "REFLECT"
    MERGE = "MERGE"


@dataclass
class Task:
    """A single task in the execution plan."""
    task_type: TaskType
    backend: str
    predicate: Optional[Dict] = None
    payload: Optional[Dict] = None
    modifiers: Dict = field(default_factory=dict)
    depends_on: List[str] = field(default_factory=list)
    budget_ms: int = 0
    scope: str = "private"
    namespace: Optional[str] = None
    id: str = field(default_factory=lambda: str(uuid.uuid4())[:8])

    def to_dict(self) -> Dict:
        """Convert to dictionary for serialization."""
        return {
            "id": self.id,
            "task_type": self.task_type.value,
            "backend": self.backend,
            "predicate": self.predicate,
            "payload": self.payload,
            "modifiers": self.modifiers,
            "depends_on": self.depends_on,
            "budget_ms": self.budget_ms,
            "scope": self.scope,
            "namespace": self.namespace,
        }


@dataclass
class TaskList:
    """Ordered list of tasks to execute."""
    tasks: List[Task] = field(default_factory=list)
    total_budget_ms: int = 0
    pipeline_name: Optional[str] = None
    merge_strategy: str = "sequential"

    def to_dict(self) -> Dict:
        """Convert to dictionary for serialization."""
        return {
            "tasks": [t.to_dict() for t in self.tasks],
            "total_budget_ms": self.total_budget_ms,
            "pipeline_name": self.pipeline_name,
            "merge_strategy": self.merge_strategy,
        }
