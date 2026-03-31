# tests/test_modifiers.py
"""Unit tests for AQL modifiers."""

import pytest
from aql_parser import parse
from aql_parser import ReturnMod, LimitMod, OrderMod
from aql_parser import ThresholdMod, ConfidenceMod, SourceMod, WeightMod
from aql_parser import AggregateMod, HavingMod, AggOp, WindowType


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


class TestWindowPredicate:
    """Tests for WINDOW predicate (v0.5)."""

    def test_window_last_n(self):
        plan = parse('SCAN WORKING WINDOW LAST 10')
        assert plan.predicate.type == "window"
        assert plan.predicate.window.window_type == WindowType.LAST_N
        assert plan.predicate.window.count == 10

    def test_window_last_duration_seconds(self):
        plan = parse('SCAN WORKING WINDOW LAST 30s')
        assert plan.predicate.window.window_type == WindowType.LAST_DUR
        assert plan.predicate.window.duration_value == 30
        assert plan.predicate.window.duration_unit == "s"

    def test_window_last_duration_minutes(self):
        plan = parse('SCAN WORKING WINDOW LAST 5m')
        assert plan.predicate.window.window_type == WindowType.LAST_DUR
        assert plan.predicate.window.duration_value == 5
        assert plan.predicate.window.duration_unit == "m"

    def test_window_last_duration_ms(self):
        plan = parse('SCAN WORKING WINDOW LAST 500ms')
        assert plan.predicate.window.window_type == WindowType.LAST_DUR
        assert plan.predicate.window.duration_value == 500
        assert plan.predicate.window.duration_unit == "ms"

    def test_window_top_by(self):
        plan = parse('SCAN WORKING WINDOW TOP 3 BY attention_weight')
        assert plan.predicate.window.window_type == WindowType.TOP
        assert plan.predicate.window.count == 3
        assert plan.predicate.window.field == "attention_weight"

    def test_window_since(self):
        plan = parse('SCAN WORKING WINDOW SINCE event_id = "inc-001"')
        assert plan.predicate.window.window_type == WindowType.SINCE
        assert plan.predicate.window.key_expr.field == "event_id"
        assert plan.predicate.window.key_expr.value == "inc-001"

    def test_window_with_return(self):
        plan = parse('''
            SCAN WORKING WINDOW LAST 10
            RETURN event, status, timestamp
        ''')
        assert plan.predicate.window.window_type == WindowType.LAST_N
        ret = plan.get_modifier(ReturnMod)
        assert "event" in ret.fields
        assert "status" in ret.fields


class TestAggregateModifier:
    """Tests for AGGREGATE modifier (v0.5)."""

    def test_aggregate_count_star(self):
        plan = parse('RECALL EPISODIC WHERE pod = "x" AGGREGATE COUNT(*) AS total')
        agg = plan.get_modifier(AggregateMod)
        assert len(agg.functions) == 1
        assert agg.functions[0].op == AggOp.COUNT
        assert agg.functions[0].field is None
        assert agg.functions[0].alias == "total"

    def test_aggregate_avg(self):
        plan = parse('RECALL EPISODIC WHERE url = "x" AGGREGATE AVG(bid_price) AS avg_cpm')
        agg = plan.get_modifier(AggregateMod)
        assert agg.functions[0].op == AggOp.AVG
        assert agg.functions[0].field == "bid_price"
        assert agg.functions[0].alias == "avg_cpm"

    def test_aggregate_sum(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" AGGREGATE SUM(amount) AS total_amount')
        agg = plan.get_modifier(AggregateMod)
        assert agg.functions[0].op == AggOp.SUM
        assert agg.functions[0].field == "amount"

    def test_aggregate_min(self):
        plan = parse('RECALL EPISODIC WHERE x = "y" AGGREGATE MIN(latency) AS min_latency')
        agg = plan.get_modifier(AggregateMod)
        assert agg.functions[0].op == AggOp.MIN

    def test_aggregate_max(self):
        plan = parse('LOOKUP PROCEDURAL WHERE x = "y" AGGREGATE MAX(confidence) AS best')
        agg = plan.get_modifier(AggregateMod)
        assert agg.functions[0].op == AggOp.MAX

    def test_aggregate_multiple(self):
        plan = parse('''
            RECALL EPISODIC WHERE url = "sports.example.com"
            AGGREGATE AVG(bid_price) AS avg_cpm, COUNT(*) AS impressions
        ''')
        agg = plan.get_modifier(AggregateMod)
        assert len(agg.functions) == 2
        assert agg.functions[0].op == AggOp.AVG
        assert agg.functions[0].alias == "avg_cpm"
        assert agg.functions[1].op == AggOp.COUNT
        assert agg.functions[1].alias == "impressions"

    def test_aggregate_with_other_modifiers(self):
        plan = parse('''
            RECALL EPISODIC WHERE pod = "payments"
            AGGREGATE COUNT(*) AS total, AVG(resolution_time) AS avg_time
            MIN_CONFIDENCE 0.7
            LIMIT 5
        ''')
        agg = plan.get_modifier(AggregateMod)
        conf = plan.get_modifier(ConfidenceMod)
        lim = plan.get_modifier(LimitMod)
        assert len(agg.functions) == 2
        assert conf.value == pytest.approx(0.7)
        assert lim.value == 5


class TestHavingModifier:
    """Tests for HAVING modifier (v0.5)."""

    def test_having_gt(self):
        plan = parse('''
            RECALL EPISODIC WHERE campaign = "summer"
            AGGREGATE COUNT(*) AS incidents
            HAVING incidents > 5
        ''')
        having = plan.get_modifier(HavingMod)
        assert having.condition.field == "incidents"
        assert having.condition.op.value == ">"
        assert having.condition.value == 5

    def test_having_gte(self):
        plan = parse('''
            RECALL EPISODIC WHERE x = "y"
            AGGREGATE AVG(score) AS avg_score
            HAVING avg_score >= 0.8
        ''')
        having = plan.get_modifier(HavingMod)
        assert having.condition.field == "avg_score"
        assert having.condition.op.value == ">="

    def test_having_eq(self):
        plan = parse('''
            RECALL EPISODIC WHERE x = "y"
            AGGREGATE COUNT(*) AS count
            HAVING count = 10
        ''')
        having = plan.get_modifier(HavingMod)
        assert having.condition.op.value == "="

    def test_aggregate_having_return(self):
        plan = parse('''
            RECALL EPISODIC WHERE campaign = "summer"
            AGGREGATE COUNT(*) AS incidents
            HAVING incidents > 5
            RETURN url, incidents
            ORDER BY incidents DESC
        ''')
        agg = plan.get_modifier(AggregateMod)
        having = plan.get_modifier(HavingMod)
        ret = plan.get_modifier(ReturnMod)
        order = plan.get_modifier(OrderMod)
        assert agg is not None
        assert having is not None
        assert "incidents" in ret.fields
        assert order.direction == "DESC"
