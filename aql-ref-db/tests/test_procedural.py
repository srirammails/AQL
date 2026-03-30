# tests/test_procedural.py
"""Tests for procedural memory backend."""

import pytest
from aql_db.backends import ProceduralBackend


class TestProceduralBackend:
    def setup_method(self):
        self.backend = ProceduralBackend()

    def test_store_procedure(self):
        record = self.backend.store(
            "oom-001",
            {
                "pattern": "OOMKilled memory limit",
                "steps": "check,scale,notify",
                "severity": "HIGH"
            }
        )
        assert record["data"]["pattern_id"] == "oom-001"
        assert record["data"]["severity"] == "HIGH"

    def test_lookup_by_pattern_id(self):
        self.backend.store("oom-001", {"pattern": "OOMKilled"})
        self.backend.store("cpu-001", {"pattern": "CPU throttle"})

        results = self.backend.lookup({"pattern_id": "oom-001"})
        assert len(results) == 1
        assert results[0]["data"]["pattern_id"] == "oom-001"

    def test_recall_pattern_matching(self):
        self.backend.store(
            "oom-001",
            {"pattern": "OOMKilled pod memory limit exceeded"}
        )
        self.backend.store(
            "cpu-001",
            {"pattern": "CPU throttling high utilization"}
        )

        # Search for memory-related patterns
        results = self.backend.recall(
            {"pattern": "memory limit OOMKilled"},
            {"threshold": 0.3}
        )
        # Should find oom-001 due to word overlap
        assert len(results) >= 1
        assert any("oom" in r["id"].lower() for r in results)

    def test_recall_with_threshold(self):
        self.backend.store("p1", {"pattern": "exact match words here"})
        self.backend.store("p2", {"pattern": "completely different text"})

        # High threshold should filter out non-matches
        results = self.backend.recall(
            {"pattern": "exact match words"},
            {"threshold": 0.5}
        )
        assert len(results) >= 1

    def test_forget_procedure(self):
        self.backend.store("p1", {"pattern": "test"})
        self.backend.store("p2", {"pattern": "test2"})

        count = self.backend.forget({"pattern_id": "p1"})
        assert count == 1

    def test_update_procedure(self):
        self.backend.store("p1", {"pattern": "test", "success_count": 0})

        count = self.backend.update(
            {"pattern_id": "p1"},
            {"success_count": 5}
        )
        assert count == 1

        results = self.backend.lookup({"pattern_id": "p1"})
        assert results[0]["data"]["success_count"] == 5

    def test_steps_as_list(self):
        record = self.backend.store(
            "p1",
            {"pattern": "test", "steps": ["step1", "step2", "step3"]}
        )
        assert isinstance(record["data"]["steps"], list)
        assert len(record["data"]["steps"]) == 3
