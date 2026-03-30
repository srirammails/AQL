# tests/test_failures.py
"""Tests that invalid queries fail cleanly."""

import pytest
from aql_parser import parse, AqlError

INVALID_QUERIES = [
    ("empty_query", "", "Empty query"),
    ("missing_verb", "EPISODIC WHERE url = 'x'", "Parse error"),
    ("invalid_verb", "FETCH EPISODIC WHERE url = 'x'", "Parse error"),
    ("pipeline_no_timeout", "PIPELINE test SCAN WORKING ALL", "Parse error"),
    ("store_no_payload", "STORE EPISODIC", "Parse error"),
    ("recall_no_predicate", "RECALL EPISODIC", "Parse error"),
    ("reflect_no_include", "REFLECT x = {y}", "Parse error"),
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
        parse("STORE SEMANTIC")


def test_incomplete_pipeline():
    with pytest.raises(AqlError):
        parse("PIPELINE TIMEOUT")


def test_invalid_memory_type():
    with pytest.raises(AqlError):
        parse("RECALL INVALID WHERE x = 1")
