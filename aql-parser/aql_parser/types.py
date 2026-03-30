# aql_parser/types.py
"""AQL v0.4 Type Definitions"""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional, List, Any, Dict
from enum import Enum
import dataclasses


class Verb(str, Enum):
    LOOKUP = "LOOKUP"
    RECALL = "RECALL"
    SCAN = "SCAN"
    LOAD = "LOAD"
    STORE = "STORE"
    UPDATE = "UPDATE"
    LINK = "LINK"
    FORGET = "FORGET"
    REFLECT = "REFLECT"
    PIPELINE = "PIPELINE"


class MemoryType(str, Enum):
    EPISODIC = "EPISODIC"
    SEMANTIC = "SEMANTIC"
    PROCEDURAL = "PROCEDURAL"
    WORKING = "WORKING"
    TOOLS = "TOOLS"


class Comparator(str, Enum):
    EQ = "="
    NEQ = "!="
    GT = ">"
    LT = "<"
    GTE = ">="
    LTE = "<="


class ScopeValue(str, Enum):
    PRIVATE = "private"
    SHARED = "shared"
    CLUSTER = "cluster"


@dataclass
class Condition:
    """Represents a WHERE condition."""
    field: str
    op: Comparator
    value: Any
    # For AND/OR compound conditions
    left: Optional[Condition] = None
    right: Optional[Condition] = None
    logical_op: Optional[str] = None  # "AND" | "OR"


@dataclass
class KeyExpr:
    """Represents a KEY field = value expression."""
    field: str
    value: Any


@dataclass
class Predicate:
    """Represents a query predicate (KEY, WHERE, LIKE, PATTERN, ALL)."""
    type: str  # "key" | "where" | "like" | "pattern" | "all"
    key_expr: Optional[KeyExpr] = None
    condition: Optional[Condition] = None
    expression: Optional[Any] = None  # for LIKE / PATTERN


@dataclass
class ReturnMod:
    """RETURN field1, field2, ..."""
    fields: List[str]


@dataclass
class LimitMod:
    """LIMIT n"""
    value: int


@dataclass
class WeightMod:
    """WEIGHT field"""
    field: str


@dataclass
class ThresholdMod:
    """THRESHOLD 0.85"""
    value: float


@dataclass
class TimeoutMod:
    """TIMEOUT 80ms"""
    value: int
    unit: str  # ms | s | m | h | d


@dataclass
class OrderMod:
    """ORDER BY field ASC|DESC"""
    field: str
    direction: str = "ASC"


@dataclass
class ConfidenceMod:
    """MIN_CONFIDENCE 0.7"""
    value: float


@dataclass
class SourceMod:
    """FROM source1, source2"""
    sources: List[str]


@dataclass
class ScopeMod:
    """SCOPE private|shared|cluster"""
    value: ScopeValue


@dataclass
class NamespaceMod:
    """NAMESPACE "agent-id" """
    value: str


@dataclass
class TtlMod:
    """TTL 90d"""
    value: int
    unit: str


@dataclass
class Payload:
    """Represents (field = value, ...) payload."""
    fields: Dict[str, Any]


@dataclass
class ReflectSource:
    """INCLUDE memory_type predicate?"""
    memory_type: MemoryType
    predicate: Optional[Predicate] = None


@dataclass
class ExecutionPlan:
    """The parsed AQL statement as an execution plan."""
    verb: Verb
    memory_type: Optional[MemoryType] = None
    predicate: Optional[Predicate] = None
    payload: Optional[Payload] = None
    modifiers: List[Any] = field(default_factory=list)

    # REFLECT specific
    context_expr: Optional[Any] = None
    sources: List[ReflectSource] = field(default_factory=list)
    then_stmt: Optional[ExecutionPlan] = None

    # PIPELINE specific
    pipeline_name: Optional[str] = None
    timeout: Optional[TimeoutMod] = None
    stages: List[ExecutionPlan] = field(default_factory=list)

    # LINK specific
    link_from: Optional[KeyExpr] = None
    link_to_type: Optional[MemoryType] = None
    link_to: Optional[KeyExpr] = None

    # Multi-agent
    scope: Optional[ScopeMod] = None
    namespace: Optional[NamespaceMod] = None
    ttl: Optional[TtlMod] = None

    def get_modifier(self, mod_type: type) -> Optional[Any]:
        """Get a modifier by type."""
        for m in self.modifiers:
            if isinstance(m, mod_type):
                return m
        return None

    def to_dict(self) -> dict:
        """Convert to dictionary for serialization."""
        def convert(obj):
            if dataclasses.is_dataclass(obj) and not isinstance(obj, type):
                return {k: convert(v) for k, v in dataclasses.asdict(obj).items()}
            if isinstance(obj, Enum):
                return obj.value
            if isinstance(obj, list):
                return [convert(i) for i in obj]
            if isinstance(obj, dict):
                return {k: convert(v) for k, v in obj.items()}
            return obj
        return convert(self)
