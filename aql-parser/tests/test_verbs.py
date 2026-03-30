# tests/test_verbs.py
"""Unit tests for each AQL verb."""

import pytest
from aql_parser import parse, Verb, MemoryType, AqlError


class TestLookup:
    def test_lookup_semantic_key(self):
        plan = parse('LOOKUP SEMANTIC KEY url = "sports.example.com"')
        assert plan.verb == Verb.LOOKUP
        assert plan.memory_type == MemoryType.SEMANTIC
        assert plan.predicate.type == "key"
        assert plan.predicate.key_expr.field == "url"
        assert plan.predicate.key_expr.value == "sports.example.com"

    def test_lookup_procedural_pattern(self):
        plan = parse('LOOKUP PROCEDURAL PATTERN $log_event THRESHOLD 0.85')
        assert plan.verb == Verb.LOOKUP
        assert plan.memory_type == MemoryType.PROCEDURAL
        assert plan.predicate.type == "pattern"

    def test_lookup_procedural_where(self):
        plan = parse('LOOKUP PROCEDURAL WHERE pattern_id = {matched}')
        assert plan.verb == Verb.LOOKUP
        assert plan.predicate.type == "where"


class TestRecall:
    def test_recall_episodic_where(self):
        plan = parse('RECALL EPISODIC WHERE pod = "payments-api" LIMIT 5')
        assert plan.verb == Verb.RECALL
        assert plan.memory_type == MemoryType.EPISODIC

    def test_recall_semantic_like(self):
        plan = parse('RECALL SEMANTIC LIKE $page_context LIMIT 10')
        assert plan.verb == Verb.RECALL
        assert plan.memory_type == MemoryType.SEMANTIC
        assert plan.predicate.type == "like"

    def test_recall_with_confidence(self):
        plan = parse(
            'RECALL EPISODIC WHERE pod = "x" MIN_CONFIDENCE 0.7 LIMIT 5'
        )
        assert plan.verb == Verb.RECALL


class TestScan:
    def test_scan_working_all(self):
        plan = parse('SCAN WORKING ALL')
        assert plan.verb == Verb.SCAN
        assert plan.memory_type == MemoryType.WORKING
        assert plan.predicate.type == "all"

    def test_scan_with_return(self):
        plan = parse('SCAN WORKING ALL RETURN active_tools, context')
        assert plan.verb == Verb.SCAN


class TestLoad:
    def test_load_tools_where(self):
        plan = parse('LOAD TOOLS WHERE relevance > 0.8 LIMIT 3')
        assert plan.verb == Verb.LOAD
        assert plan.memory_type == MemoryType.TOOLS

    def test_load_tools_with_task(self):
        plan = parse('LOAD TOOLS WHERE task = "bidding" LIMIT 3')
        assert plan.verb == Verb.LOAD


class TestStore:
    def test_store_episodic(self):
        plan = parse('''
            STORE EPISODIC (
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
            STORE SEMANTIC (concept = "k8s_oom")
            SCOPE shared
            NAMESPACE "platform-agents"
        ''')
        assert plan.scope.value.value == "shared"
        assert plan.namespace.value == "platform-agents"

    def test_store_with_ttl(self):
        plan = parse('STORE WORKING (task_id = "x") TTL 5m')
        assert plan.ttl.value == 5
        assert plan.ttl.unit == "m"


class TestUpdate:
    def test_update_episodic(self):
        plan = parse('UPDATE EPISODIC WHERE id = "x" (resolved = true)')
        assert plan.verb == Verb.UPDATE
        assert plan.memory_type == MemoryType.EPISODIC
        assert plan.payload.fields["resolved"] is True


class TestForget:
    def test_forget_episodic(self):
        plan = parse('FORGET EPISODIC WHERE last_accessed > 30')
        assert plan.verb == Verb.FORGET
        assert plan.memory_type == MemoryType.EPISODIC

    def test_forget_working(self):
        plan = parse('FORGET WORKING WHERE task_id = "completed-001"')
        assert plan.verb == Verb.FORGET
        assert plan.memory_type == MemoryType.WORKING


class TestLink:
    def test_link_memories(self):
        plan = parse('''
            LINK EPISODIC episode_id = "evt-001"
            TO SEMANTIC concept_id = "payment_failure"
        ''')
        assert plan.verb == Verb.LINK
        assert plan.memory_type == MemoryType.EPISODIC
        assert plan.link_from.field == "episode_id"
        assert plan.link_to_type == MemoryType.SEMANTIC
        assert plan.link_to.field == "concept_id"


class TestReflect:
    def test_reflect_basic(self):
        plan = parse('''
            REFLECT incident_id = {current}
            INCLUDE EPISODIC
            INCLUDE PROCEDURAL
        ''')
        assert plan.verb == Verb.REFLECT
        assert len(plan.sources) == 2

    def test_reflect_with_predicates(self):
        plan = parse('''
            REFLECT incident_id = {current}
            INCLUDE EPISODIC WHERE incident_id = {current}
            INCLUDE PROCEDURAL WHERE pattern_id = {matched}
        ''')
        assert plan.verb == Verb.REFLECT
        assert plan.sources[0].predicate is not None
        assert plan.sources[1].predicate is not None

    def test_reflect_with_then(self):
        plan = parse('''
            REFLECT url = {url}
            INCLUDE SEMANTIC
            THEN STORE PROCEDURAL (pattern = "x")
        ''')
        assert plan.then_stmt is not None
        assert plan.then_stmt.verb == Verb.STORE


class TestPipeline:
    def test_pipeline_basic(self):
        plan = parse('''
            PIPELINE bid_decision TIMEOUT 80ms
            LOOKUP SEMANTIC KEY url = {url}
            | RECALL EPISODIC WHERE url = {url} LIMIT 10
        ''')
        assert plan.verb == Verb.PIPELINE
        assert plan.pipeline_name == "bid_decision"
        assert plan.timeout.value == 80
        assert plan.timeout.unit == "ms"
        assert len(plan.stages) == 2

    def test_pipeline_without_name(self):
        plan = parse('''
            PIPELINE TIMEOUT 200ms
            SCAN WORKING ALL
        ''')
        assert plan.pipeline_name is None
        assert len(plan.stages) == 1

    def test_pipeline_with_reflect(self):
        plan = parse('''
            PIPELINE test TIMEOUT 100ms
            LOOKUP SEMANTIC KEY url = {url}
            | REFLECT url = {url}
                INCLUDE SEMANTIC
                INCLUDE EPISODIC
        ''')
        assert plan.verb == Verb.PIPELINE
        assert len(plan.stages) == 2
        assert plan.stages[1].verb == Verb.REFLECT
