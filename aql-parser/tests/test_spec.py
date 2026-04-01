# tests/test_spec.py
"""Spec compliance tests — every example from AQL v0.5 spec must parse."""

import pytest
from aql_parser import parse, Verb, MemoryType, AqlError

# Every example from AQL_SPEC_v0.5.md (updated with FROM/INTO syntax)
SPEC_QUERIES = [
    # Tool Selection
    ("tool_load",
     'LOAD FROM TOOLS WHERE relevance > 0.8 ORDER BY ranking DESC LIMIT 3 RETURN tool_id, schema'),

    ("tool_scan",
     'SCAN FROM WORKING ALL RETURN active_tools'),

    # Multi-Agent Memory
    ("store_shared_semantic",
     '''STORE INTO SEMANTIC (
         concept = "k8s_oom_pattern",
         knowledge = "payments-api OOMs every Friday"
     )
     SCOPE shared
     NAMESPACE "platform-agents"'''),

    ("store_private_episodic",
     '''STORE INTO EPISODIC (
         incident_id = "inc-001",
         action = "scaled memory",
         resolved = true
     )
     SCOPE private'''),

    # Quality-Filtered Recall
    ("recall_episodic_confidence",
     '''RECALL FROM EPISODIC WHERE pod = "payments-api"
        MIN_CONFIDENCE 0.7
        ORDER BY time DESC
        LIMIT 5
        RETURN incident_id, action, resolved'''),

    ("recall_semantic_like",
     '''RECALL FROM SEMANTIC LIKE $current_context
        MIN_CONFIDENCE 0.8
        LIMIT 10'''),

    # v0.5: Recall from ALL
    ("recall_from_all",
     '''RECALL FROM ALL WHERE url = "example.com"
        LIMIT 10'''),

    # Forgetting
    ("forget_episodic",
     'FORGET FROM EPISODIC WHERE last_accessed > 30'),

    ("forget_working",
     'FORGET FROM WORKING WHERE task_id = "completed-001"'),

    # v0.5: Forget from ALL
    ("forget_from_all",
     'FORGET FROM ALL WHERE temp = "true"'),

    # Pattern Matching
    ("lookup_procedural_pattern",
     '''LOOKUP FROM PROCEDURAL PATTERN $log_event
        THRESHOLD 0.85
        RETURN pattern_id, severity, action_steps'''),

    ("lookup_procedural_where",
     '''LOOKUP FROM PROCEDURAL WHERE pattern_id = {matched}
        RETURN steps, priority'''),

    # v0.5: WITH LINKS modifier
    ("lookup_with_links",
     '''LOOKUP FROM PROCEDURAL WHERE pattern_id = "oom-fix"
        WITH LINKS ALL
        RETURN pattern_id, steps'''),

    # v0.5: FOLLOW LINKS modifier
    ("recall_follow_links",
     '''RECALL FROM SEMANTIC LIKE $embedding
        FOLLOW LINKS TYPE "triggers"
        RETURN procedures'''),

    # Context Assembly (v0.5: uses FROM instead of INCLUDE)
    ("reflect_basic",
     '''REFLECT FROM EPISODIC WHERE incident_id = {current},
              FROM PROCEDURAL WHERE pattern_id = {matched},
              FROM WORKING'''),

    ("reflect_with_then",
     '''REFLECT FROM SEMANTIC,
              FROM EPISODIC
        THEN STORE INTO PROCEDURAL (
            pattern = "learned_pattern",
            steps = "step1"
        )'''),

    # v0.5: REFLECT FROM ALL
    ("reflect_from_all",
     '''REFLECT FROM ALL WHERE incident_id = {current}'''),

    # Full Pipelines (updated with FROM/INTO)
    ("pipeline_rtb",
     '''PIPELINE bid_decision TIMEOUT 80ms
        LOAD FROM TOOLS WHERE task = "bidding" LIMIT 3
        | LOOKUP FROM SEMANTIC KEY url = {url}
        | RECALL FROM EPISODIC WHERE url = {url} LIMIT 10
        | REFLECT FROM SEMANTIC,
                  FROM EPISODIC'''),

    ("pipeline_incident",
     '''PIPELINE incident TIMEOUT 200ms
        LOOKUP FROM PROCEDURAL PATTERN $log_event
            THRESHOLD 0.85
        | RECALL FROM EPISODIC WHERE pattern_id = {matched}
            MIN_CONFIDENCE 0.7
            LIMIT 5
        | LOAD FROM TOOLS WHERE category = "kubernetes"
        | REFLECT FROM EPISODIC,
                  FROM PROCEDURAL'''),

    # v0.5: Link with FROM/TO/TYPE/WEIGHT
    ("link_memories",
     '''LINK FROM EPISODIC WHERE episode_id = "evt-001"
        TO SEMANTIC WHERE concept_id = "payment_failure"
        TYPE "evidence_for"
        WEIGHT 0.95'''),

    # v0.5: Link without TYPE/WEIGHT
    ("link_simple",
     '''LINK FROM PROCEDURAL WHERE pattern_id = "oom-fix"
        TO EPISODIC WHERE incident_id = "inc-001"'''),

    # v0.5: WINDOW predicate
    ("scan_window_last_n",
     'SCAN FROM WORKING WINDOW LAST 10'),

    ("scan_window_duration",
     'SCAN FROM WORKING WINDOW LAST 30s'),

    ("scan_window_top",
     'SCAN FROM WORKING WINDOW TOP 3 BY attention_weight'),

    # v0.5: AGGREGATE
    ("recall_aggregate",
     '''RECALL FROM EPISODIC WHERE pod = "payments"
        AGGREGATE COUNT(*) AS total, AVG(resolution_time) AS avg_time'''),

    # v0.5: AGGREGATE with HAVING
    ("recall_aggregate_having",
     '''RECALL FROM EPISODIC WHERE campaign = "summer"
        AGGREGATE COUNT(*) AS incidents
        HAVING incidents > 5
        RETURN url, incidents
        ORDER BY incidents DESC'''),
]


@pytest.mark.parametrize("name,query", SPEC_QUERIES)
def test_spec_query_parses(name, query):
    """Every query in AQL v0.5 spec must parse without error."""
    try:
        plan = parse(query.strip())
        assert plan is not None, f"{name}: parser returned None"
    except AqlError as e:
        pytest.fail(f"{name}: failed — {e}")


def test_spec_verbs_correct():
    """Verify verb is correctly extracted for each spec query."""
    cases = [
        ('LOAD FROM TOOLS WHERE relevance > 0.8 LIMIT 3', Verb.LOAD),
        ('SCAN FROM WORKING ALL', Verb.SCAN),
        ('STORE INTO EPISODIC (x = "y")', Verb.STORE),
        ('FORGET FROM EPISODIC WHERE last_accessed > 30', Verb.FORGET),
        ('LOOKUP FROM PROCEDURAL PATTERN $log THRESHOLD 0.85', Verb.LOOKUP),
        ('RECALL FROM EPISODIC WHERE pod = "x" LIMIT 5', Verb.RECALL),
        ('UPDATE INTO EPISODIC WHERE id = "x" (resolved = true)', Verb.UPDATE),
    ]
    for query, expected_verb in cases:
        plan = parse(query)
        assert plan.verb == expected_verb, \
            f"Expected {expected_verb}, got {plan.verb} for: {query}"


def test_spec_memory_types_correct():
    """Verify memory type is correctly extracted."""
    cases = [
        ('RECALL FROM EPISODIC WHERE x = "y"', MemoryType.EPISODIC),
        ('RECALL FROM SEMANTIC LIKE $ctx', MemoryType.SEMANTIC),
        ('LOOKUP FROM PROCEDURAL PATTERN $x THRESHOLD 0.5', MemoryType.PROCEDURAL),
        ('SCAN FROM WORKING ALL', MemoryType.WORKING),
        ('LOAD FROM TOOLS WHERE relevance > 0.5', MemoryType.TOOLS),
    ]
    for query, expected_type in cases:
        plan = parse(query)
        assert plan.memory_type == expected_type, \
            f"Expected {expected_type}, got {plan.memory_type}"


def test_recall_from_all_memory_type():
    """v0.5: RECALL FROM ALL should have memory_type = None."""
    plan = parse('RECALL FROM ALL WHERE url = "example.com"')
    assert plan.memory_type is None


def test_forget_from_all_memory_type():
    """v0.5: FORGET FROM ALL should have memory_type = None."""
    plan = parse('FORGET FROM ALL WHERE temp = "true"')
    assert plan.memory_type is None
