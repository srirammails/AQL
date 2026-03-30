# tests/test_router.py
"""Tests for the routing module."""

import pytest
from aql_parser import Verb, MemoryType
from aql_planner import route, RoutingError


class TestRouter:
    def test_lookup_semantic_routes_to_semantic_backend(self):
        assert route(Verb.LOOKUP, MemoryType.SEMANTIC) == "semantic"

    def test_lookup_procedural_routes_to_procedural_backend(self):
        assert route(Verb.LOOKUP, MemoryType.PROCEDURAL) == "procedural"

    def test_lookup_tools_routes_to_tools_backend(self):
        assert route(Verb.LOOKUP, MemoryType.TOOLS) == "tools"

    def test_recall_episodic_routes_to_episodic_backend(self):
        assert route(Verb.RECALL, MemoryType.EPISODIC) == "episodic"

    def test_recall_semantic_routes_to_semantic_backend(self):
        assert route(Verb.RECALL, MemoryType.SEMANTIC) == "semantic"

    def test_scan_working_routes_to_working_backend(self):
        assert route(Verb.SCAN, MemoryType.WORKING) == "working"

    def test_load_tools_routes_to_tools_backend(self):
        assert route(Verb.LOAD, MemoryType.TOOLS) == "tools"

    def test_store_episodic_routes_to_episodic_backend(self):
        assert route(Verb.STORE, MemoryType.EPISODIC) == "episodic"

    def test_store_semantic_routes_to_semantic_backend(self):
        assert route(Verb.STORE, MemoryType.SEMANTIC) == "semantic"

    def test_forget_working_routes_to_working_backend(self):
        assert route(Verb.FORGET, MemoryType.WORKING) == "working"

    def test_reflect_routes_to_merger(self):
        assert route(Verb.REFLECT, None) == "merger"

    def test_pipeline_routes_to_pipeline(self):
        assert route(Verb.PIPELINE, None) == "pipeline"


class TestRouterErrors:
    def test_recall_working_raises_invalid_combination(self):
        with pytest.raises(RoutingError, match="Use SCAN for WORKING"):
            route(Verb.RECALL, MemoryType.WORKING)

    def test_scan_episodic_raises_invalid_combination(self):
        with pytest.raises(RoutingError, match="SCAN ALL only supported for WORKING"):
            route(Verb.SCAN, MemoryType.EPISODIC)

    def test_scan_semantic_raises_invalid_combination(self):
        with pytest.raises(RoutingError, match="SCAN ALL only supported for WORKING"):
            route(Verb.SCAN, MemoryType.SEMANTIC)
