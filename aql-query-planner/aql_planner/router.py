# aql_planner/router.py
"""Routes verb + memory_type to backend."""

from typing import Dict, Optional, Tuple, Union

from aql_parser import Verb, MemoryType
from .errors import RoutingError

# Routing table: (verb, memory_type) -> backend
# None for memory_type means "any" or "not applicable"
ROUTING_TABLE: Dict[Tuple[Verb, Optional[MemoryType]], str] = {
    # LOOKUP routes
    (Verb.LOOKUP, MemoryType.SEMANTIC): "semantic",
    (Verb.LOOKUP, MemoryType.PROCEDURAL): "procedural",
    (Verb.LOOKUP, MemoryType.TOOLS): "tools",
    (Verb.LOOKUP, MemoryType.EPISODIC): "episodic",
    (Verb.LOOKUP, MemoryType.WORKING): "working",

    # RECALL routes
    (Verb.RECALL, MemoryType.EPISODIC): "episodic",
    (Verb.RECALL, MemoryType.SEMANTIC): "semantic",
    (Verb.RECALL, MemoryType.PROCEDURAL): "procedural",

    # SCAN routes (only WORKING supports SCAN ALL)
    (Verb.SCAN, MemoryType.WORKING): "working",

    # LOAD routes (only TOOLS)
    (Verb.LOAD, MemoryType.TOOLS): "tools",

    # STORE routes to matching backend
    (Verb.STORE, MemoryType.WORKING): "working",
    (Verb.STORE, MemoryType.EPISODIC): "episodic",
    (Verb.STORE, MemoryType.SEMANTIC): "semantic",
    (Verb.STORE, MemoryType.PROCEDURAL): "procedural",
    (Verb.STORE, MemoryType.TOOLS): "tools",

    # UPDATE routes to matching backend
    (Verb.UPDATE, MemoryType.WORKING): "working",
    (Verb.UPDATE, MemoryType.EPISODIC): "episodic",
    (Verb.UPDATE, MemoryType.SEMANTIC): "semantic",
    (Verb.UPDATE, MemoryType.PROCEDURAL): "procedural",
    (Verb.UPDATE, MemoryType.TOOLS): "tools",

    # FORGET routes to matching backend
    (Verb.FORGET, MemoryType.WORKING): "working",
    (Verb.FORGET, MemoryType.EPISODIC): "episodic",
    (Verb.FORGET, MemoryType.SEMANTIC): "semantic",
    (Verb.FORGET, MemoryType.PROCEDURAL): "procedural",
    (Verb.FORGET, MemoryType.TOOLS): "tools",

    # LINK is cross-backend (graph operations)
    (Verb.LINK, MemoryType.EPISODIC): "graph",
    (Verb.LINK, MemoryType.SEMANTIC): "graph",
    (Verb.LINK, MemoryType.PROCEDURAL): "graph",

    # REFLECT has no memory type - goes to merger
    (Verb.REFLECT, None): "merger",

    # PIPELINE has no memory type - handled specially
    (Verb.PIPELINE, None): "pipeline",
}

# Invalid combinations that should raise clear errors
INVALID_COMBINATIONS = {
    (Verb.RECALL, MemoryType.WORKING): "Use SCAN for WORKING memory, not RECALL",
    (Verb.SCAN, MemoryType.EPISODIC): "SCAN ALL only supported for WORKING memory",
    (Verb.SCAN, MemoryType.SEMANTIC): "SCAN ALL only supported for WORKING memory",
    (Verb.SCAN, MemoryType.PROCEDURAL): "SCAN ALL only supported for WORKING memory",
    (Verb.SCAN, MemoryType.TOOLS): "SCAN ALL only supported for WORKING memory",
}


def route(verb: Verb, memory_type: Optional[MemoryType]) -> str:
    """
    Route a verb + memory_type combination to the appropriate backend.

    Args:
        verb: The AQL verb (LOOKUP, RECALL, STORE, etc.)
        memory_type: The memory type (EPISODIC, SEMANTIC, etc.) or None

    Returns:
        Backend name string

    Raises:
        RoutingError: If the combination is invalid
    """
    # Check for explicitly invalid combinations
    key = (verb, memory_type)
    if key in INVALID_COMBINATIONS:
        raise RoutingError(INVALID_COMBINATIONS[key])

    # Look up in routing table
    if key in ROUTING_TABLE:
        return ROUTING_TABLE[key]

    # Handle unknown combinations
    raise RoutingError(
        f"No route for verb={verb.value}, memory_type={memory_type.value if memory_type else None}"
    )
