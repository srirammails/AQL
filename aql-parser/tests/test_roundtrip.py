# tests/test_roundtrip.py
"""Tests that parsed queries can be serialized (v0.5 syntax)."""

import json
import pytest
from aql_parser import parse
from tests.test_spec import SPEC_QUERIES


@pytest.mark.parametrize("name,query", SPEC_QUERIES)
def test_plan_serialises(name, query):
    """Every spec query must produce a serialisable ExecutionPlan."""
    plan = parse(query.strip())
    try:
        d = plan.to_dict()
        serialised = json.dumps(d)
        assert serialised  # non-empty
        assert len(serialised) > 10  # reasonable size
    except Exception as e:
        pytest.fail(f"{name}: serialisation failed — {e}")


def test_simple_query_dict():
    """Test basic dict conversion."""
    plan = parse('SCAN FROM WORKING ALL')
    d = plan.to_dict()
    assert d["verb"] == "SCAN"
    assert d["memory_type"] == "WORKING"


def test_store_with_scope_dict():
    """Test store with scope converts to dict."""
    plan = parse('''
        STORE INTO SEMANTIC (concept = "test")
        SCOPE shared
        NAMESPACE "my-agent"
    ''')
    d = plan.to_dict()
    assert d["verb"] == "STORE"
    assert d["scope"]["value"] == "shared"
    assert d["namespace"]["value"] == "my-agent"


def test_pipeline_dict():
    """Test pipeline converts to dict with stages."""
    plan = parse('''
        PIPELINE test TIMEOUT 80ms
        SCAN FROM WORKING ALL
        | RECALL FROM EPISODIC WHERE x = "y" LIMIT 5
    ''')
    d = plan.to_dict()
    assert d["verb"] == "PIPELINE"
    assert d["pipeline_name"] == "test"
    assert len(d["stages"]) == 2


def test_link_dict():
    """Test v0.5 LINK converts to dict."""
    plan = parse('''
        LINK FROM EPISODIC WHERE episode_id = "evt-001"
        TO SEMANTIC WHERE concept_id = "payment_failure"
        TYPE "evidence_for"
        WEIGHT 0.95
    ''')
    d = plan.to_dict()
    assert d["verb"] == "LINK"
    assert d["link_from_type"] == "EPISODIC"
    assert d["link_to_type"] == "SEMANTIC"
    assert d["link_type"] == "evidence_for"
    assert d["link_weight"] == 0.95
