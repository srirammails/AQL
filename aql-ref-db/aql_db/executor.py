# aql_db/executor.py
"""Task executor - runs TaskList against backends."""

from typing import Any, Dict, List, Optional, TYPE_CHECKING
from datetime import datetime

from aql_planner import TaskList, Task, TaskType

from .errors import ExecutionError

if TYPE_CHECKING:
    from .store import ADB


class TaskExecutor:
    """
    Executes a TaskList against ADB backends.

    Handles dependencies, budget enforcement, and result collection.
    """

    def __init__(self, store: "ADB"):
        self.store = store
        self.results: Dict[str, Any] = {}

    def execute(self, task_list: TaskList) -> Dict[str, Any]:
        """
        Execute all tasks in the task list.

        Args:
            task_list: Ordered list of tasks with dependencies

        Returns:
            Result from the final task
        """
        self.results = {}

        for task in task_list.tasks:
            # Wait for dependencies (in this sync impl, just check they exist)
            for dep_id in task.depends_on:
                if dep_id not in self.results:
                    raise ExecutionError(
                        f"Dependency {dep_id} not complete for task {task.id}"
                    )

            # Execute the task
            result = self._execute_task(task)
            self.results[task.id] = result

        # Return final result
        if task_list.tasks:
            return self.results[task_list.tasks[-1].id]
        return {}

    def _execute_task(self, task: Task) -> Dict[str, Any]:
        """Execute a single task."""
        # Handle REFLECT specially - uses merger, not a backend
        if task.task_type == TaskType.REFLECT:
            source_results = {
                dep_id: self.results[dep_id]
                for dep_id in task.depends_on
            }
            return self.store.merger.merge(source_results)

        backend = self.store.get_backend(task.backend)

        if task.task_type == TaskType.LOOKUP:
            records = backend.lookup(task.predicate or {}, task.modifiers)
            return {
                "memory_type": self._backend_to_memory_type(task.backend),
                "records": records,
            }

        elif task.task_type == TaskType.RECALL:
            records = backend.recall(task.predicate or {}, task.modifiers)
            return {
                "memory_type": self._backend_to_memory_type(task.backend),
                "records": records,
            }

        elif task.task_type == TaskType.SCAN:
            records = backend.scan()
            # Filter by predicate if provided (SCAN WHERE ...)
            if task.predicate and task.predicate.get("conditions"):
                records = self._filter_records(records, task.predicate)
            records = backend._apply_modifiers(records, task.modifiers)
            return {
                "memory_type": "WORKING",
                "records": records,
            }

        elif task.task_type == TaskType.LOAD:
            # LOAD TOOLS uses lookup with ranking
            records = backend.recall(task.predicate or {}, task.modifiers)
            return {
                "memory_type": "TOOLS",
                "tools": records,
                "records": records,
            }

        elif task.task_type == TaskType.STORE:
            # Extract key from predicate or payload
            key = self._extract_key(task)
            data = task.payload or {}
            record = backend.store(
                key=key,
                data=data,
                scope=task.scope,
                namespace=task.namespace,
            )
            return {
                "memory_type": self._backend_to_memory_type(task.backend),
                "stored": record,
            }

        elif task.task_type == TaskType.UPDATE:
            count = backend.update(task.predicate or {}, task.payload or {})
            return {
                "memory_type": self._backend_to_memory_type(task.backend),
                "updated_count": count,
            }

        elif task.task_type == TaskType.FORGET:
            count = backend.forget(task.predicate or {})
            return {
                "memory_type": self._backend_to_memory_type(task.backend),
                "deleted_count": count,
            }

        elif task.task_type == TaskType.LINK:
            # LINK is graph operation - not fully implemented in reference
            return {
                "memory_type": "GRAPH",
                "linked": True,
            }

        else:
            raise ExecutionError(f"Unknown task type: {task.task_type}")

    def _extract_key(self, task: Task) -> str:
        """Extract or generate a key for store operations."""
        # Try to get key from predicate
        if task.predicate:
            key_value = task.predicate.get("key_value")
            if key_value:
                return str(key_value)

            conditions = task.predicate.get("conditions", [])
            for cond in conditions:
                if cond.get("field") in ("id", "key", "concept_id", "pattern_id", "tool_id", "task_id"):
                    return str(cond.get("value", ""))

        # Try to get from payload
        if task.payload:
            for key_field in ("id", "key", "concept_id", "pattern_id", "tool_id", "task_id", "concept", "incident_id"):
                if key_field in task.payload:
                    return str(task.payload[key_field])

        # Generate UUID
        import uuid
        return str(uuid.uuid4())[:8]

    def _backend_to_memory_type(self, backend: str) -> str:
        """Convert backend name to memory type."""
        mapping = {
            "working": "WORKING",
            "tools": "TOOLS",
            "procedural": "PROCEDURAL",
            "semantic": "SEMANTIC",
            "episodic": "EPISODIC",
            "merger": "MERGED",
            "graph": "GRAPH",
        }
        return mapping.get(backend, backend.upper())

    def _filter_records(
        self,
        records: List[Dict[str, Any]],
        predicate: Dict[str, Any],
    ) -> List[Dict[str, Any]]:
        """Filter records by predicate conditions."""
        conditions = predicate.get("conditions", [])
        if not conditions:
            return records

        filtered = []
        for record in records:
            match = True
            for cond in conditions:
                field = cond.get("field")
                op = cond.get("op")
                value = cond.get("value")

                record_value = record.get("data", {}).get(field)

                if op in ("=", "EQ"):
                    if record_value != value:
                        match = False
                        break
                elif op in ("!=", "NEQ"):
                    if record_value == value:
                        match = False
                        break
                elif op in (">", "GT"):
                    if not (record_value and record_value > value):
                        match = False
                        break
                elif op in ("<", "LT"):
                    if not (record_value and record_value < value):
                        match = False
                        break
                elif op in (">=", "GTE"):
                    if not (record_value and record_value >= value):
                        match = False
                        break
                elif op in ("<=", "LTE"):
                    if not (record_value and record_value <= value):
                        match = False
                        break

            if match:
                filtered.append(record)

        return filtered
