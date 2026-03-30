# tests/test_spec.py
"""Spec compliance tests — every example from AQL v0.4 spec must parse."""

import pytest
from aql_parser import parse, Verb, MemoryType, AqlError

# Every example from AQL_SPEC_v0.4.md
SPEC_QUERIES = [
    # Tool Selection
    ("tool_load",
     'LOAD TOOLS WHERE relevance > 0.8 ORDER BY ranking DESC LIMIT 3 RETURN tool_id, schema'),

    ("tool_scan",
     'SCAN WORKING ALL RETURN active_tools'),

    # Multi-Agent Memory
    ("store_shared_semantic",
     '''STORE SEMANTIC (
         concept = "k8s_oom_pattern",
         knowledge = "payments-api OOMs every Friday"
     )
     SCOPE shared
     NAMESPACE "platform-agents"'''),

    ("store_private_episodic",
     '''STORE EPISODIC (
         incident_id = "inc-001",
         action = "scaled memory",
         resolved = true
     )
     SCOPE private'''),

    # Quality-Filtered Recall
    ("recall_episodic_confidence",
     '''RECALL EPISODIC WHERE pod = "payments-api"
        MIN_CONFIDENCE 0.7
        ORDER BY time DESC
        LIMIT 5
        RETURN incident_id, action, resolved'''),

    ("recall_semantic_like",
     '''RECALL SEMANTIC LIKE $current_context
        MIN_CONFIDENCE 0.8
        LIMIT 10'''),

    # Forgetting
    ("forget_episodic",
     'FORGET EPISODIC WHERE last_accessed > 30'),

    ("forget_working",
     'FORGET WORKING WHERE task_id = "completed-001"'),

    # Pattern Matching
    ("lookup_procedural_pattern",
     '''LOOKUP PROCEDURAL PATTERN $log_event
        THRESHOLD 0.85
        RETURN pattern_id, severity, action_steps'''),

    ("lookup_procedural_where",
     '''LOOKUP PROCEDURAL WHERE pattern_id = {matched}
        RETURN steps, priority'''),

    # Context Assembly
    ("reflect_basic",
     '''REFLECT incident_id = {current}
        INCLUDE EPISODIC WHERE incident_id = {current}
        INCLUDE PROCEDURAL WHERE pattern_id = {matched}
        INCLUDE WORKING'''),

    ("reflect_with_then",
     '''REFLECT url = {url}
        INCLUDE SEMANTIC
        INCLUDE EPISODIC
        THEN STORE PROCEDURAL (
            pattern = "learned_pattern",
            steps = "step1"
        )'''),

    # Full Pipelines
    ("pipeline_rtb",
     '''PIPELINE bid_decision TIMEOUT 80ms
        LOAD TOOLS WHERE task = "bidding" LIMIT 3
        | LOOKUP SEMANTIC KEY url = {url}
        | RECALL EPISODIC WHERE url = {url} LIMIT 10
        | REFLECT url = {url}
            INCLUDE SEMANTIC
            INCLUDE EPISODIC'''),

    ("pipeline_incident",
     '''PIPELINE incident TIMEOUT 200ms
        LOOKUP PROCEDURAL PATTERN $log_event
            THRESHOLD 0.85
        | RECALL EPISODIC WHERE pattern_id = {matched}
            MIN_CONFIDENCE 0.7
            LIMIT 5
        | LOAD TOOLS WHERE category = "kubernetes"
        | REFLECT incident_id = {current}
            INCLUDE EPISODIC
            INCLUDE PROCEDURAL'''),

    # Link
    ("link_memories",
     '''LINK EPISODIC episode_id = "evt-001"
        TO SEMANTIC concept_id = "payment_failure"'''),
]


@pytest.mark.parametrize("name,query", SPEC_QUERIES)
def test_spec_query_parses(name, query):
    """Every query in AQL v0.4 spec must parse without error."""
    try:
        plan = parse(query.strip())
        assert plan is not None, f"{name}: parser returned None"
    except AqlError as e:
        pytest.fail(f"{name}: failed — {e}")


def test_spec_verbs_correct():
    """Verify verb is correctly extracted for each spec query."""
    cases = [
        ('LOAD TOOLS WHERE relevance > 0.8 LIMIT 3', Verb.LOAD),
        ('SCAN WORKING ALL', Verb.SCAN),
        ('STORE EPISODIC (x = "y")', Verb.STORE),
        ('FORGET EPISODIC WHERE last_accessed > 30', Verb.FORGET),
        ('LOOKUP PROCEDURAL PATTERN $log THRESHOLD 0.85', Verb.LOOKUP),
        ('RECALL EPISODIC WHERE pod = "x" LIMIT 5', Verb.RECALL),
        ('UPDATE EPISODIC WHERE id = "x" (resolved = true)', Verb.UPDATE),
    ]
    for query, expected_verb in cases:
        plan = parse(query)
        assert plan.verb == expected_verb, \
            f"Expected {expected_verb}, got {plan.verb} for: {query}"


def test_spec_memory_types_correct():
    """Verify memory type is correctly extracted."""
    cases = [
        ('RECALL EPISODIC WHERE x = "y"', MemoryType.EPISODIC),
        ('RECALL SEMANTIC LIKE $ctx', MemoryType.SEMANTIC),
        ('LOOKUP PROCEDURAL PATTERN $x THRESHOLD 0.5', MemoryType.PROCEDURAL),
        ('SCAN WORKING ALL', MemoryType.WORKING),
        ('LOAD TOOLS WHERE relevance > 0.5', MemoryType.TOOLS),
    ]
    for query, expected_type in cases:
        plan = parse(query)
        assert plan.memory_type == expected_type, \
            f"Expected {expected_type}, got {plan.memory_type}"
