# tests/test_episodic.py
"""Tests for episodic memory backend."""

import pytest
import time
from aql_db.backends import EpisodicBackend


class TestEpisodicBackend:
    def setup_method(self):
        self.backend = EpisodicBackend()

    def test_store_episode(self):
        record = self.backend.store(
            "inc-001",
            {
                "incident_id": "inc-001",
                "pod": "payments-api",
                "action": "scaled memory"
            }
        )
        assert record["data"]["pod"] == "payments-api"

    def test_lookup_by_condition(self):
        self.backend.store("e1", {"pod": "payments-api"})
        self.backend.store("e2", {"pod": "auth-service"})

        results = self.backend.lookup(
            {"conditions": [{"field": "pod", "op": "=", "value": "payments-api"}]}
        )
        assert len(results) == 1
        assert results[0]["data"]["pod"] == "payments-api"

    def test_recall_default_time_order(self):
        self.backend.store("e1", {"order": 1})
        time.sleep(0.01)
        self.backend.store("e2", {"order": 2})
        time.sleep(0.01)
        self.backend.store("e3", {"order": 3})

        results = self.backend.recall({})
        # Should be ordered by time DESC (most recent first)
        assert results[0]["data"]["order"] == 3
        assert results[2]["data"]["order"] == 1

    def test_recall_with_limit(self):
        for i in range(10):
            self.backend.store(f"e{i}", {"idx": i})

        results = self.backend.recall({}, {"limit": 3})
        assert len(results) == 3

    def test_forget_old_episodes(self):
        self.backend.store("e1", {"status": "old"})
        self.backend.store("e2", {"status": "new"})

        count = self.backend.forget(
            {"conditions": [{"field": "status", "op": "=", "value": "old"}]}
        )
        assert count == 1

    def test_update_episode(self):
        self.backend.store("e1", {"status": "active", "resolved": False})

        count = self.backend.update(
            {"conditions": [{"field": "status", "op": "=", "value": "active"}]},
            {"resolved": True}
        )
        assert count == 1

    def test_order_by_modifier(self):
        self.backend.store("e1", {"priority": 3})
        self.backend.store("e2", {"priority": 1})
        self.backend.store("e3", {"priority": 2})

        results = self.backend.recall(
            {},
            {"order_by": "priority", "order_dir": "ASC"}
        )
        priorities = [r["data"]["priority"] for r in results]
        assert priorities == [1, 2, 3]

    def test_scope_metadata(self):
        record = self.backend.store(
            "e1", {"x": 1},
            scope="shared",
            namespace="agent-001"
        )
        assert record["metadata"]["scope"] == "shared"
        assert record["metadata"]["namespace"] == "agent-001"
