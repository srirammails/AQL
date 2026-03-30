# tests/test_modifiers.py
"""Unit tests for AQL modifiers."""

import pytest
from aql_parser import parse
from aql_parser import ReturnMod, LimitMod, OrderMod
from aql_parser import ThresholdMod, ConfidenceMod, SourceMod, WeightMod


class TestModifiers:
    def test_return(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" RETURN field1, field2')
        ret = plan.get_modifier(ReturnMod)
        assert ret is not None
        assert "field1" in ret.fields
        assert "field2" in ret.fields

    def test_limit(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" LIMIT 10')
        lim = plan.get_modifier(LimitMod)
        assert lim.value == 10

    def test_order_asc(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" ORDER BY time ASC')
        order = plan.get_modifier(OrderMod)
        assert order.field == "time"
        assert order.direction == "ASC"

    def test_order_desc(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" ORDER BY time DESC')
        order = plan.get_modifier(OrderMod)
        assert order.direction == "DESC"

    def test_order_default_asc(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" ORDER BY time')
        order = plan.get_modifier(OrderMod)
        assert order.direction == "ASC"

    def test_threshold(self):
        plan = parse('LOOKUP PROCEDURAL PATTERN $x THRESHOLD 0.85')
        t = plan.get_modifier(ThresholdMod)
        assert t.value == pytest.approx(0.85)

    def test_min_confidence(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" MIN_CONFIDENCE 0.7')
        c = plan.get_modifier(ConfidenceMod)
        assert c.value == pytest.approx(0.7)

    def test_from_sources(self):
        plan = parse(
            'LOOKUP PROCEDURAL WHERE pattern_id = {x} FROM confluence, jira'
        )
        src = plan.get_modifier(SourceMod)
        assert "confluence" in src.sources
        assert "jira" in src.sources

    def test_weight(self):
        plan = parse(
            'RECALL SEMANTIC LIKE $ctx WEIGHT relevance_score'
        )
        w = plan.get_modifier(WeightMod)
        assert w.field == "relevance_score"

    def test_timeout(self):
        plan = parse('PIPELINE test TIMEOUT 80ms SCAN WORKING ALL')
        assert plan.timeout.value == 80
        assert plan.timeout.unit == "ms"

    def test_ttl_minutes(self):
        plan = parse('STORE WORKING (x = "y") TTL 5m')
        assert plan.ttl.value == 5
        assert plan.ttl.unit == "m"

    def test_ttl_days(self):
        plan = parse('STORE WORKING (x = "y") TTL 90d')
        assert plan.ttl.value == 90
        assert plan.ttl.unit == "d"

    def test_ttl_hours(self):
        plan = parse('STORE WORKING (x = "y") TTL 24h')
        assert plan.ttl.value == 24
        assert plan.ttl.unit == "h"

    def test_multiple_modifiers(self):
        plan = parse('''
            RECALL EPISODIC WHERE pod = "x"
            MIN_CONFIDENCE 0.7
            ORDER BY time DESC
            LIMIT 5
            RETURN incident_id, action
        ''')
        assert plan.get_modifier(ConfidenceMod) is not None
        assert plan.get_modifier(OrderMod) is not None
        assert plan.get_modifier(LimitMod) is not None
        assert plan.get_modifier(ReturnMod) is not None
