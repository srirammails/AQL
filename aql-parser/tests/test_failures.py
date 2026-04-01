# tests/test_failures.py
"""Tests that invalid queries fail cleanly (v0.5 syntax)."""

import pytest
from aql_parser import parse, AqlError

INVALID_QUERIES = [
    ("empty_query", "", "Empty query"),
    ("missing_verb", "FROM EPISODIC WHERE url = 'x'", "Parse error"),
    ("invalid_verb", "FETCH FROM EPISODIC WHERE url = 'x'", "Parse error"),
    ("pipeline_no_timeout", "PIPELINE test SCAN FROM WORKING ALL", "Parse error"),
    ("store_no_payload", "STORE INTO EPISODIC", "Parse error"),
    ("recall_no_predicate", "RECALL FROM EPISODIC", "Parse error"),
    ("reflect_no_from", "REFLECT", "Parse error"),
    # v0.5: old syntax without FROM/INTO should fail
    ("recall_without_from", "RECALL EPISODIC WHERE x = 'y'", "Parse error"),
    ("store_without_into", "STORE EPISODIC (x = 'y')", "Parse error"),
]


@pytest.mark.parametrize("name,query,expected", INVALID_QUERIES)
def test_invalid_query_raises(name, query, expected):
    """Invalid queries must raise AqlError."""
    with pytest.raises(AqlError):
        parse(query)


def test_empty_string():
    with pytest.raises(AqlError, match="Empty query"):
        parse("")


def test_whitespace_only():
    with pytest.raises(AqlError, match="Empty query"):
        parse("   \n\t  ")


def test_incomplete_store():
    with pytest.raises(AqlError):
        parse("STORE INTO SEMANTIC")


def test_incomplete_pipeline():
    with pytest.raises(AqlError):
        parse("PIPELINE TIMEOUT")


def test_invalid_memory_type():
    with pytest.raises(AqlError):
        parse("RECALL FROM INVALID WHERE x = 1")
