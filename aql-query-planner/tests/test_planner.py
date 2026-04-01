# tests/test_planner.py
"""Tests for the query planner (v0.5 syntax with FROM/INTO)."""

import pytest
from aql_parser import parse
from aql_planner import plan, TaskType


class TestPlanSimple:
    def test_simple_lookup_produces_one_task(self):
        execution_plan = parse('LOOKUP FROM SEMANTIC KEY concept = "test"')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 1
        assert task_list.tasks[0].backend == "semantic"
        assert task_list.tasks[0].task_type == TaskType.LOOKUP

    def test_recall_episodic_routes_correctly(self):
        execution_plan = parse('RECALL FROM EPISODIC WHERE pod = "x" LIMIT 5')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 1
        assert task_list.tasks[0].backend == "episodic"
        assert task_list.tasks[0].task_type == TaskType.RECALL

    def test_scan_working_routes_correctly(self):
        execution_plan = parse('SCAN FROM WORKING ALL')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 1
        assert task_list.tasks[0].backend == "working"
        assert task_list.tasks[0].task_type == TaskType.SCAN

    def test_load_tools_routes_correctly(self):
        execution_plan = parse('LOAD FROM TOOLS WHERE relevance > 0.8 LIMIT 3')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 1
        assert task_list.tasks[0].backend == "tools"
        assert task_list.tasks[0].task_type == TaskType.LOAD

    def test_store_working_routes_correctly(self):
        execution_plan = parse('STORE INTO WORKING (task_id = "t1")')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 1
        assert task_list.tasks[0].backend == "working"
        assert task_list.tasks[0].task_type == TaskType.STORE

    def test_forget_episodic_routes_correctly(self):
        execution_plan = parse('FORGET FROM EPISODIC WHERE last_accessed > 30')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 1
        assert task_list.tasks[0].backend == "episodic"
        assert task_list.tasks[0].task_type == TaskType.FORGET


class TestPlanModifiers:
    def test_limit_modifier_extracted(self):
        execution_plan = parse('RECALL FROM EPISODIC WHERE x = "y" LIMIT 10')
        task_list = plan(execution_plan)
        assert task_list.tasks[0].modifiers.get("limit") == 10

    def test_order_modifier_extracted(self):
        execution_plan = parse('RECALL FROM EPISODIC WHERE x = "y" ORDER BY time DESC')
        task_list = plan(execution_plan)
        assert task_list.tasks[0].modifiers.get("order_by") == "time"
        assert task_list.tasks[0].modifiers.get("order_dir") == "DESC"

    def test_threshold_modifier_extracted(self):
        execution_plan = parse('LOOKUP FROM PROCEDURAL PATTERN $x THRESHOLD 0.85')
        task_list = plan(execution_plan)
        assert task_list.tasks[0].modifiers.get("threshold") == pytest.approx(0.85)

    def test_confidence_modifier_extracted(self):
        execution_plan = parse('RECALL FROM SEMANTIC LIKE $ctx MIN_CONFIDENCE 0.7')
        task_list = plan(execution_plan)
        assert task_list.tasks[0].modifiers.get("min_confidence") == pytest.approx(0.7)


class TestPlanScope:
    def test_scope_extracted(self):
        execution_plan = parse('STORE INTO SEMANTIC (concept = "x") SCOPE shared')
        task_list = plan(execution_plan)
        assert task_list.tasks[0].scope == "shared"

    def test_namespace_extracted(self):
        execution_plan = parse(
            'STORE INTO SEMANTIC (concept = "x") SCOPE shared NAMESPACE "my-agent"'
        )
        task_list = plan(execution_plan)
        assert task_list.tasks[0].namespace == "my-agent"

    def test_default_scope_is_private(self):
        execution_plan = parse('STORE INTO WORKING (x = "y")')
        task_list = plan(execution_plan)
        assert task_list.tasks[0].scope == "private"


class TestPlanPipeline:
    def test_pipeline_produces_ordered_tasks(self):
        execution_plan = parse('''
            PIPELINE test TIMEOUT 100ms
            SCAN FROM WORKING ALL
            | RECALL FROM EPISODIC WHERE x = "y" LIMIT 5
        ''')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 2
        assert task_list.pipeline_name == "test"

    def test_pipeline_tasks_have_correct_dependencies(self):
        execution_plan = parse('''
            PIPELINE test TIMEOUT 100ms
            SCAN FROM WORKING ALL
            | LOOKUP FROM SEMANTIC KEY concept = "x"
            | RECALL FROM EPISODIC WHERE pod = "y" LIMIT 5
        ''')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 3

        # First task has no dependencies
        assert task_list.tasks[0].depends_on == []

        # Second task depends on first
        assert task_list.tasks[1].depends_on == [task_list.tasks[0].id]

        # Third task depends on second
        assert task_list.tasks[2].depends_on == [task_list.tasks[1].id]

    def test_pipeline_budget_allocated(self):
        execution_plan = parse('''
            PIPELINE test TIMEOUT 80ms
            SCAN FROM WORKING ALL
            | RECALL FROM EPISODIC WHERE x = "y" LIMIT 5
        ''')
        task_list = plan(execution_plan)
        assert task_list.total_budget_ms == 80
        # Each task should have a budget > 0
        assert all(t.budget_ms > 0 for t in task_list.tasks)


class TestPlanReflect:
    def test_reflect_produces_source_tasks_plus_merge(self):
        """v0.5: REFLECT uses FROM instead of INCLUDE."""
        execution_plan = parse('''
            REFLECT FROM EPISODIC,
                    FROM SEMANTIC
        ''')
        task_list = plan(execution_plan)

        # 2 source tasks + 1 merge task
        assert len(task_list.tasks) == 3
        assert task_list.merge_strategy == "reflect"

    def test_reflect_merge_task_depends_on_sources(self):
        execution_plan = parse('''
            REFLECT FROM EPISODIC,
                    FROM PROCEDURAL
        ''')
        task_list = plan(execution_plan)

        # Find the merge task (last one with REFLECT type)
        merge_task = task_list.tasks[-1]
        assert merge_task.task_type == TaskType.REFLECT
        assert merge_task.backend == "merger"

        # Should depend on both source tasks
        source_ids = [t.id for t in task_list.tasks[:-1]]
        assert set(merge_task.depends_on) == set(source_ids)

    def test_reflect_with_three_sources(self):
        execution_plan = parse('''
            REFLECT FROM EPISODIC,
                    FROM SEMANTIC,
                    FROM WORKING
        ''')
        task_list = plan(execution_plan)

        # 3 source tasks + 1 merge task
        assert len(task_list.tasks) == 4


class TestFullScenarios:
    def test_full_rtb_pipeline_task_count(self):
        """RTB scenario from spec."""
        execution_plan = parse('''
            PIPELINE bid_decision TIMEOUT 80ms
            LOAD FROM TOOLS WHERE task = "bidding" LIMIT 3
            | LOOKUP FROM SEMANTIC KEY url = {url}
            | RECALL FROM EPISODIC WHERE url = {url} LIMIT 10
        ''')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 3
        assert task_list.pipeline_name == "bid_decision"
        assert task_list.total_budget_ms == 80

    def test_full_k8s_pipeline_task_count(self):
        """K8s scenario from spec."""
        execution_plan = parse('''
            PIPELINE incident TIMEOUT 200ms
            LOOKUP FROM PROCEDURAL PATTERN $log_event THRESHOLD 0.5
            | RECALL FROM EPISODIC WHERE pod = "payments-api" LIMIT 5
        ''')
        task_list = plan(execution_plan)
        assert len(task_list.tasks) == 2
        assert task_list.pipeline_name == "incident"
        assert task_list.total_budget_ms == 200
