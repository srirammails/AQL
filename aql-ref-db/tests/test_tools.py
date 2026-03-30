# tests/test_tools.py
"""Tests for tools registry backend."""

import pytest
from aql_db.backends import ToolsBackend


class TestToolsBackend:
    def setup_method(self):
        self.backend = ToolsBackend()

    def test_store_tool(self):
        record = self.backend.store(
            "calc",
            {
                "description": "Calculator tool",
                "category": "math",
                "ranking": 0.8
            }
        )
        assert record["data"]["tool_id"] == "calc"
        assert record["data"]["ranking"] == 0.8

    def test_lookup_by_category(self):
        self.backend.store("calc", {"category": "math"})
        self.backend.store("search", {"category": "web"})

        results = self.backend.lookup(
            {"conditions": [{"field": "category", "op": "=", "value": "math"}]}
        )
        assert len(results) == 1
        assert results[0]["data"]["category"] == "math"

    def test_recall_sorted_by_ranking(self):
        self.backend.store("tool1", {"category": "test", "ranking": 0.3})
        self.backend.store("tool2", {"category": "test", "ranking": 0.9})
        self.backend.store("tool3", {"category": "test", "ranking": 0.6})

        results = self.backend.recall(
            {"conditions": [{"field": "category", "op": "=", "value": "test"}]}
        )
        # Should be sorted by ranking DESC
        rankings = [r["data"]["ranking"] for r in results]
        assert rankings == [0.9, 0.6, 0.3]

    def test_recall_with_threshold(self):
        self.backend.store("tool1", {"ranking": 0.3})
        self.backend.store("tool2", {"ranking": 0.9})
        self.backend.store("tool3", {"ranking": 0.6})

        results = self.backend.recall({}, {"threshold": 0.5})
        assert len(results) == 2

    def test_update_ranking(self):
        self.backend.store("tool1", {"ranking": 0.5})

        # Simulate success - ranking should increase
        new_ranking = self.backend.update_ranking("tool1", success=True)
        assert new_ranking > 0.5

        # Check call_count updated
        results = self.backend.lookup({})
        assert results[0]["data"]["call_count"] == 1

    def test_forget_tool(self):
        self.backend.store("tool1", {})
        self.backend.store("tool2", {})

        count = self.backend.forget(
            {"conditions": [{"field": "tool_id", "op": "=", "value": "tool1"}]}
        )
        assert count == 1
        results = self.backend.lookup({})
        assert len(results) == 1

    def test_limit_modifier(self):
        for i in range(10):
            self.backend.store(f"tool{i}", {"ranking": i / 10})

        results = self.backend.recall({}, {"limit": 3})
        assert len(results) == 3
