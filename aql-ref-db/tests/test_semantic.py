# tests/test_semantic.py
"""Tests for semantic memory backend."""

import pytest
from aql_db.backends import SemanticBackend


class TestSemanticBackend:
    def setup_method(self):
        self.backend = SemanticBackend()

    def test_store_concept(self):
        record = self.backend.store(
            "payments-api",
            {
                "concept": "payments-api",
                "knowledge": "critical service for payment processing"
            }
        )
        assert record["data"]["concept_id"] == "payments-api"
        assert "payment" in record["data"]["knowledge"]

    def test_lookup_by_concept(self):
        self.backend.store("c1", {"concept": "payments", "knowledge": "..."})
        self.backend.store("c2", {"concept": "auth", "knowledge": "..."})

        results = self.backend.lookup({"key_value": "payments"})
        assert len(results) >= 1

    def test_recall_similarity(self):
        self.backend.store(
            "k8s",
            {"concept": "kubernetes", "knowledge": "container orchestration platform pods"}
        )
        self.backend.store(
            "docker",
            {"concept": "docker", "knowledge": "container runtime images"}
        )
        self.backend.store(
            "python",
            {"concept": "python", "knowledge": "programming language"}
        )

        # Search for container-related concepts
        results = self.backend.recall(
            {"expression": "container pods orchestration"},
            {"min_confidence": 0.1}
        )
        assert len(results) >= 1
        # k8s should rank higher due to more word overlap
        assert results[0]["data"]["concept"] == "kubernetes"

    def test_recall_with_min_confidence(self):
        self.backend.store("c1", {"concept": "test", "knowledge": "exact match words"})
        self.backend.store("c2", {"concept": "other", "knowledge": "different content"})

        # High confidence should filter
        results = self.backend.recall(
            {"expression": "exact match words"},
            {"min_confidence": 0.5}
        )
        assert all(r.get("_similarity", 0) >= 0.5 for r in results)

    def test_forget_concept(self):
        self.backend.store("c1", {"concept": "test"})
        self.backend.store("c2", {"concept": "test2"})

        count = self.backend.forget({"key_value": "test"})
        assert count == 1

    def test_update_knowledge(self):
        self.backend.store("c1", {"concept": "test", "knowledge": "old"})

        count = self.backend.update(
            {"key_value": "c1"},
            {"knowledge": "updated knowledge"}
        )
        assert count == 1

        results = self.backend.lookup({"key_value": "c1"})
        assert "updated" in results[0]["data"]["knowledge"]

    def test_limit_modifier(self):
        for i in range(10):
            self.backend.store(f"c{i}", {"concept": f"concept{i}", "knowledge": "test"})

        results = self.backend.recall(
            {"expression": "test"},
            {"limit": 3}
        )
        assert len(results) == 3
