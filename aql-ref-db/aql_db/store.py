# aql_db/store.py
"""ADB - Agent Database main entry point."""

from typing import Any, Dict, Optional

from .backends import (
    BaseBackend,
    WorkingBackend,
    ToolsBackend,
    ProceduralBackend,
    SemanticBackend,
    EpisodicBackend,
)
from .merger import ResultMerger
from .executor import TaskExecutor
from .errors import ADBError


class ADB:
    """
    Agent Database — reference implementation.

    In-memory. Python. Not production.
    Use this to learn, test, and demo AQL.

    Usage:
        db = ADB()
        result = db.execute('SCAN WORKING ALL')
        result = db.execute('RECALL EPISODIC WHERE pod = "x" LIMIT 5')
    """

    def __init__(self):
        self.backends: Dict[str, BaseBackend] = {
            "working": WorkingBackend(),
            "tools": ToolsBackend(),
            "procedural": ProceduralBackend(),
            "semantic": SemanticBackend(),
            "episodic": EpisodicBackend(),
        }
        self.merger = ResultMerger()
        self._executor: Optional[TaskExecutor] = None

    @property
    def executor(self) -> TaskExecutor:
        """Lazy-load executor to avoid circular import."""
        if self._executor is None:
            self._executor = TaskExecutor(self)
        return self._executor

    def execute(self, query: str) -> Dict[str, Any]:
        """
        Main entry point.
        Takes AQL string, returns result dict.

        Args:
            query: AQL query string

        Returns:
            Result dictionary from execution

        Raises:
            ADBError: On parse or execution error
        """
        try:
            from aql_parser import parse, AqlError
            from aql_planner import plan, PlannerError
        except ImportError as e:
            raise ADBError(f"Required packages not installed: {e}")

        try:
            execution_plan = parse(query)
        except Exception as e:
            raise ADBError(f"Parse error: {e}")

        try:
            task_list = plan(execution_plan)
        except Exception as e:
            raise ADBError(f"Planning error: {e}")

        try:
            return self.executor.execute(task_list)
        except Exception as e:
            raise ADBError(f"Execution error: {e}")

    def get_backend(self, name: str) -> BaseBackend:
        """
        Get a backend by name.

        Args:
            name: Backend name (working, tools, procedural, semantic, episodic)

        Returns:
            Backend instance

        Raises:
            ADBError: If backend not found
        """
        if name == "merger":
            # Special case - merger is not a backend but handle gracefully
            raise ADBError("Use db.merger directly for merge operations")

        if name not in self.backends:
            raise ADBError(f"Unknown backend: {name}")

        return self.backends[name]

    def reset(self):
        """Reset all backends to empty state."""
        self.backends = {
            "working": WorkingBackend(),
            "tools": ToolsBackend(),
            "procedural": ProceduralBackend(),
            "semantic": SemanticBackend(),
            "episodic": EpisodicBackend(),
        }
        self._executor = None
