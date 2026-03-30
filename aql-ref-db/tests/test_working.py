# tests/test_working.py
"""Tests for working memory backend."""

import pytest
import time
from aql_db.backends import WorkingBackend


class TestWorkingBackend:
    def setup_method(self):
        self.backend = WorkingBackend()

    def test_store_and_lookup(self):
        self.backend.store("task1", {"status": "active"})
        results = self.backend.lookup(
            {"conditions": [{"field": "status", "op": "=", "value": "active"}]}
        )
        assert len(results) == 1
        assert results[0]["data"]["status"] == "active"

    def test_scan_returns_all(self):
        self.backend.store("t1", {"x": 1})
        self.backend.store("t2", {"x": 2})
        self.backend.store("t3", {"x": 3})
        results = self.backend.scan()
        assert len(results) == 3

    def test_forget_removes_matching(self):
        self.backend.store("t1", {"status": "done"})
        self.backend.store("t2", {"status": "active"})
        count = self.backend.forget(
            {"conditions": [{"field": "status", "op": "=", "value": "done"}]}
        )
        assert count == 1
        assert len(self.backend.scan()) == 1

    def test_update_modifies_records(self):
        self.backend.store("t1", {"status": "active", "count": 0})
        count = self.backend.update(
            {"conditions": [{"field": "status", "op": "=", "value": "active"}]},
            {"count": 5}
        )
        assert count == 1
        results = self.backend.scan()
        assert results[0]["data"]["count"] == 5

    def test_ttl_expiration(self):
        self.backend.store("temp", {"x": 1}, ttl=1)
        results = self.backend.scan()
        assert len(results) == 1

        time.sleep(1.1)
        results = self.backend.scan()
        assert len(results) == 0

    def test_recall_falls_back_to_lookup(self):
        self.backend.store("t1", {"status": "active"})
        results = self.backend.recall(
            {"conditions": [{"field": "status", "op": "=", "value": "active"}]}
        )
        assert len(results) == 1

    def test_comparison_operators(self):
        self.backend.store("t1", {"value": 10})
        self.backend.store("t2", {"value": 20})
        self.backend.store("t3", {"value": 30})

        # Greater than
        results = self.backend.lookup(
            {"conditions": [{"field": "value", "op": ">", "value": 15}]}
        )
        assert len(results) == 2

        # Less than
        results = self.backend.lookup(
            {"conditions": [{"field": "value", "op": "<", "value": 25}]}
        )
        assert len(results) == 2

    def test_scope_and_namespace(self):
        record = self.backend.store(
            "t1", {"x": 1},
            scope="shared",
            namespace="agent-001"
        )
        assert record["metadata"]["scope"] == "shared"
        assert record["metadata"]["namespace"] == "agent-001"
