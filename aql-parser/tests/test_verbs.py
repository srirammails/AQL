# tests/test_verbs.py
"""Unit tests for each AQL verb (v0.5 syntax with FROM/INTO)."""

import pytest
from aql_parser import parse, Verb, MemoryType, AqlError


class TestLookup:
    def test_lookup_semantic_key(self):
        plan = parse('LOOKUP FROM SEMANTIC KEY url = "sports.example.com"')
        assert plan.verb == Verb.LOOKUP
        assert plan.memory_type == MemoryType.SEMANTIC
        assert plan.predicate.type == "key"
        assert plan.predicate.key_expr.field == "url"
        assert plan.predicate.key_expr.value == "sports.example.com"

    def test_lookup_procedural_pattern(self):
        plan = parse('LOOKUP FROM PROCEDURAL PATTERN $log_event THRESHOLD 0.85')
        assert plan.verb == Verb.LOOKUP
        assert plan.memory_type == MemoryType.PROCEDURAL
        assert plan.predicate.type == "pattern"

    def test_lookup_procedural_where(self):
        plan = parse('LOOKUP FROM PROCEDURAL WHERE pattern_id = {matched}')
        assert plan.verb == Verb.LOOKUP
        assert plan.predicate.type == "where"


class TestRecall:
    def test_recall_episodic_where(self):
        plan = parse('RECALL FROM EPISODIC WHERE pod = "payments-api" LIMIT 5')
        assert plan.verb == Verb.RECALL
        assert plan.memory_type == MemoryType.EPISODIC

    def test_recall_semantic_like(self):
        plan = parse('RECALL FROM SEMANTIC LIKE $page_context LIMIT 10')
        assert plan.verb == Verb.RECALL
        assert plan.memory_type == MemoryType.SEMANTIC
        assert plan.predicate.type == "like"

    def test_recall_with_confidence(self):
        plan = parse(
            'RECALL FROM EPISODIC WHERE pod = "x" MIN_CONFIDENCE 0.7 LIMIT 5'
        )
        assert plan.verb == Verb.RECALL

    def test_recall_from_all(self):
        """v0.5: RECALL FROM ALL for cross-memory search."""
        plan = parse('RECALL FROM ALL WHERE url = "example.com" LIMIT 10')
        assert plan.verb == Verb.RECALL
        assert plan.memory_type is None  # ALL represented as None


class TestScan:
    def test_scan_working_all(self):
        plan = parse('SCAN FROM WORKING ALL')
        assert plan.verb == Verb.SCAN
        assert plan.memory_type == MemoryType.WORKING
        assert plan.predicate.type == "all"

    def test_scan_with_return(self):
        plan = parse('SCAN FROM WORKING ALL RETURN active_tools, context')
        assert plan.verb == Verb.SCAN


class TestLoad:
    def test_load_tools_where(self):
        plan = parse('LOAD FROM TOOLS WHERE relevance > 0.8 LIMIT 3')
        assert plan.verb == Verb.LOAD
        assert plan.memory_type == MemoryType.TOOLS

    def test_load_tools_with_task(self):
        plan = parse('LOAD FROM TOOLS WHERE task = "bidding" LIMIT 3')
        assert plan.verb == Verb.LOAD


class TestStore:
    def test_store_episodic(self):
        plan = parse('''
            STORE INTO EPISODIC (
                incident_id = "inc-001",
                resolved = true
            )
        ''')
        assert plan.verb == Verb.STORE
        assert plan.memory_type == MemoryType.EPISODIC
        assert plan.payload.fields["incident_id"] == "inc-001"
        assert plan.payload.fields["resolved"] is True

    def test_store_with_scope(self):
        plan = parse('''
            STORE INTO SEMANTIC (concept = "k8s_oom")
            SCOPE shared
            NAMESPACE "platform-agents"
        ''')
        assert plan.scope.value.value == "shared"
        assert plan.namespace.value == "platform-agents"

    def test_store_with_ttl(self):
        plan = parse('STORE INTO WORKING (task_id = "x") TTL 5m')
        assert plan.ttl.value == 5
        assert plan.ttl.unit == "m"


class TestUpdate:
    def test_update_episodic(self):
        plan = parse('UPDATE INTO EPISODIC WHERE id = "x" (resolved = true)')
        assert plan.verb == Verb.UPDATE
        assert plan.memory_type == MemoryType.EPISODIC
        assert plan.payload.fields["resolved"] is True


class TestForget:
    def test_forget_episodic(self):
        plan = parse('FORGET FROM EPISODIC WHERE last_accessed > 30')
        assert plan.verb == Verb.FORGET
        assert plan.memory_type == MemoryType.EPISODIC

    def test_forget_working(self):
        plan = parse('FORGET FROM WORKING WHERE task_id = "completed-001"')
        assert plan.verb == Verb.FORGET
        assert plan.memory_type == MemoryType.WORKING

    def test_forget_from_all(self):
        """v0.5: FORGET FROM ALL with WHERE clause."""
        plan = parse('FORGET FROM ALL WHERE temp = "true"')
        assert plan.verb == Verb.FORGET
        assert plan.memory_type is None  # ALL represented as None


class TestLink:
    def test_link_memories(self):
        """v0.5: LINK FROM memory_type WHERE ... TO memory_type WHERE ... TYPE? WEIGHT?"""
        plan = parse('''
            LINK FROM EPISODIC WHERE episode_id = "evt-001"
            TO SEMANTIC WHERE concept_id = "payment_failure"
            TYPE "evidence_for"
            WEIGHT 0.95
        ''')
        assert plan.verb == Verb.LINK
        assert plan.link_from_type == MemoryType.EPISODIC
        assert plan.link_from_predicate.field == "episode_id"
        assert plan.link_to_type == MemoryType.SEMANTIC
        assert plan.link_to_predicate.field == "concept_id"
        assert plan.link_type == "evidence_for"
        assert plan.link_weight == 0.95

    def test_link_without_type_weight(self):
        plan = parse('''
            LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
            TO EPISODIC WHERE incident_id = "inc-001"
        ''')
        assert plan.verb == Verb.LINK
        assert plan.link_type is None
        assert plan.link_weight is None


class TestReflect:
    def test_reflect_basic(self):
        """v0.5: REFLECT FROM memory_type, FROM memory_type ..."""
        plan = parse('''
            REFLECT FROM EPISODIC,
                    FROM PROCEDURAL
        ''')
        assert plan.verb == Verb.REFLECT
        assert len(plan.sources) == 2

    def test_reflect_with_predicates(self):
        plan = parse('''
            REFLECT FROM EPISODIC WHERE incident_id = {current},
                    FROM PROCEDURAL WHERE pattern_id = {matched}
        ''')
        assert plan.verb == Verb.REFLECT
        assert plan.sources[0].predicate is not None
        assert plan.sources[1].predicate is not None

    def test_reflect_with_then(self):
        plan = parse('''
            REFLECT FROM SEMANTIC
            THEN STORE INTO PROCEDURAL (pattern = "x")
        ''')
        assert plan.then_stmt is not None
        assert plan.then_stmt.verb == Verb.STORE

    def test_reflect_from_all(self):
        """v0.5: REFLECT FROM ALL predicate?"""
        plan = parse('''
            REFLECT FROM ALL WHERE incident_id = {current}
        ''')
        assert plan.verb == Verb.REFLECT
        assert len(plan.sources) == 1
        assert plan.sources[0].is_all is True


class TestPipeline:
    def test_pipeline_basic(self):
        plan = parse('''
            PIPELINE bid_decision TIMEOUT 80ms
            LOOKUP FROM SEMANTIC KEY url = {url}
            | RECALL FROM EPISODIC WHERE url = {url} LIMIT 10
        ''')
        assert plan.verb == Verb.PIPELINE
        assert plan.pipeline_name == "bid_decision"
        assert plan.timeout.value == 80
        assert plan.timeout.unit == "ms"
        assert len(plan.stages) == 2

    def test_pipeline_without_name(self):
        plan = parse('''
            PIPELINE TIMEOUT 200ms
            SCAN FROM WORKING ALL
        ''')
        assert plan.pipeline_name is None
        assert len(plan.stages) == 1

    def test_pipeline_with_reflect(self):
        plan = parse('''
            PIPELINE test TIMEOUT 100ms
            LOOKUP FROM SEMANTIC KEY url = {url}
            | REFLECT FROM SEMANTIC,
                      FROM EPISODIC
        ''')
        assert plan.verb == Verb.PIPELINE
        assert len(plan.stages) == 2
        assert plan.stages[1].verb == Verb.REFLECT
