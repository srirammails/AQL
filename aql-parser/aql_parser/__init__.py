# aql_parser/__init__.py
"""
AQL Parser — Agent Query Language v0.4

A parser for AQL, the query language for ADB (Agent Database).

Usage:
    from aql_parser import parse, ExecutionPlan, Verb, MemoryType

    plan = parse('RECALL EPISODIC WHERE pod = "payments" LIMIT 5')
    print(plan.verb)        # Verb.RECALL
    print(plan.memory_type) # MemoryType.EPISODIC
"""

from .parser import parse
from .types import (
    ExecutionPlan,
    Verb,
    MemoryType,
    Comparator,
    ScopeValue,
    Predicate,
    Payload,
    ReflectSource,
    ReturnMod,
    LimitMod,
    WeightMod,
    ThresholdMod,
    TimeoutMod,
    OrderMod,
    ConfidenceMod,
    SourceMod,
    ScopeMod,
    NamespaceMod,
    TtlMod,
    KeyExpr,
    Condition,
)
from .errors import AqlError

__version__ = "0.4.0"
__all__ = [
    "parse",
    "ExecutionPlan",
    "Verb",
    "MemoryType",
    "Comparator",
    "ScopeValue",
    "Predicate",
    "Payload",
    "ReflectSource",
    "ReturnMod",
    "LimitMod",
    "WeightMod",
    "ThresholdMod",
    "TimeoutMod",
    "OrderMod",
    "ConfidenceMod",
    "SourceMod",
    "ScopeMod",
    "NamespaceMod",
    "TtlMod",
    "KeyExpr",
    "Condition",
    "AqlError",
]
