# aql_db/__init__.py
"""AQL Reference Database - in-memory implementation."""

from .store import ADB
from .executor import TaskExecutor
from .merger import ResultMerger
from .errors import ADBError, BackendError, ExecutionError
from .backends import (
    BaseBackend,
    WorkingBackend,
    ToolsBackend,
    ProceduralBackend,
    SemanticBackend,
    EpisodicBackend,
)

__all__ = [
    # Main class
    "ADB",
    # Components
    "TaskExecutor",
    "ResultMerger",
    # Errors
    "ADBError",
    "BackendError",
    "ExecutionError",
    # Backends
    "BaseBackend",
    "WorkingBackend",
    "ToolsBackend",
    "ProceduralBackend",
    "SemanticBackend",
    "EpisodicBackend",
]
