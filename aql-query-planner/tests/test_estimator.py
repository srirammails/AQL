# tests/test_estimator.py
"""Tests for the latency estimator."""

import pytest
import warnings
from aql_planner import estimate, allocate_budget, estimate_pipeline


class TestEstimate:
    def test_working_estimate_is_1ms(self):
        assert estimate("working") == 1

    def test_tools_estimate_is_2ms(self):
        assert estimate("tools") == 2

    def test_procedural_estimate_is_5ms(self):
        assert estimate("procedural") == 5

    def test_semantic_estimate_is_20ms(self):
        assert estimate("semantic") == 20

    def test_episodic_estimate_is_50ms(self):
        assert estimate("episodic") == 50

    def test_merger_estimate_is_5ms(self):
        assert estimate("merger") == 5

    def test_unknown_backend_defaults_to_10ms(self):
        assert estimate("unknown") == 10


class TestAllocateBudget:
    def test_single_backend_gets_full_budget(self):
        allocations = allocate_budget(["working"], 100)
        assert allocations == [100]

    def test_two_backends_proportional_allocation(self):
        # working=1ms, episodic=50ms, total estimate=51ms
        # with 100ms budget, should allocate proportionally
        allocations = allocate_budget(["working", "episodic"], 100)
        assert len(allocations) == 2
        assert sum(allocations) >= 100  # at least the budget

    def test_pipeline_fits_within_budget(self):
        # 80ms budget for working(1) + semantic(20) + episodic(50) = 71ms
        allocations = allocate_budget(["working", "semantic", "episodic"], 80)
        assert len(allocations) == 3
        # No warning should be issued since 71 < 80

    def test_pipeline_over_budget_warns(self):
        # 30ms budget for working(1) + semantic(20) + episodic(50) = 71ms
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            allocations = allocate_budget(["working", "semantic", "episodic"], 30)
            assert len(w) == 1
            assert "exceed timeout" in str(w[0].message)

    def test_empty_backends_returns_empty(self):
        assert allocate_budget([], 100) == []

    def test_minimum_budget_per_task(self):
        # Even with tiny budget, each task gets minimum
        allocations = allocate_budget(["working", "tools", "semantic"], 10)
        assert all(a >= 5 for a in allocations)  # MIN_BUDGET_MS = 5


class TestEstimatePipeline:
    def test_simple_pipeline_estimate(self):
        backends = ["working", "semantic", "episodic"]
        total = estimate_pipeline(backends)
        assert total == 1 + 20 + 50  # 71ms

    def test_empty_pipeline_estimate(self):
        assert estimate_pipeline([]) == 0

    def test_single_stage_estimate(self):
        assert estimate_pipeline(["tools"]) == 2
