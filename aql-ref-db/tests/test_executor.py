# tests/test_executor.py
"""Tests for task executor."""

import pytest
from aql_db import ADB
from aql_planner import Task, TaskList, TaskType


class TestExecutor:
    def setup_method(self):
        self.db = ADB()

    def test_execute_scan_working(self):
        # Store some data first
        self.db.backends["working"].store("t1", {"x": 1})
        self.db.backends["working"].store("t2", {"x": 2})

        task = Task(
            task_type=TaskType.SCAN,
            backend="working",
        )
        task_list = TaskList(tasks=[task])

        result = self.db.executor.execute(task_list)
        assert result["memory_type"] == "WORKING"
        assert len(result["records"]) == 2

    def test_execute_store_and_lookup(self):
        # Store task
        store_task = Task(
            task_type=TaskType.STORE,
            backend="working",
            payload={"task_id": "t1", "status": "active"},
        )
        store_list = TaskList(tasks=[store_task])
        self.db.executor.execute(store_list)

        # Lookup task
        lookup_task = Task(
            task_type=TaskType.LOOKUP,
            backend="working",
            predicate={"conditions": [{"field": "status", "op": "=", "value": "active"}]},
        )
        lookup_list = TaskList(tasks=[lookup_task])
        result = self.db.executor.execute(lookup_list)

        assert len(result["records"]) == 1

    def test_execute_with_dependencies(self):
        # First task stores
        task1 = Task(
            task_type=TaskType.STORE,
            backend="episodic",
            payload={"event": "test"},
        )

        # Second task depends on first and recalls
        task2 = Task(
            task_type=TaskType.RECALL,
            backend="episodic",
            predicate={},
            depends_on=[task1.id],
        )

        task_list = TaskList(tasks=[task1, task2])
        result = self.db.executor.execute(task_list)

        assert len(result["records"]) >= 1

    def test_execute_forget(self):
        # Store first
        self.db.backends["working"].store("t1", {"status": "done"})
        self.db.backends["working"].store("t2", {"status": "active"})

        task = Task(
            task_type=TaskType.FORGET,
            backend="working",
            predicate={"conditions": [{"field": "status", "op": "=", "value": "done"}]},
        )
        task_list = TaskList(tasks=[task])
        result = self.db.executor.execute(task_list)

        assert result["deleted_count"] == 1

    def test_execute_reflect(self):
        # Setup data
        self.db.backends["working"].store("w1", {"task": "current"})
        self.db.backends["episodic"].store("e1", {"event": "past"})

        # Source tasks
        working_task = Task(
            task_type=TaskType.SCAN,
            backend="working",
        )
        episodic_task = Task(
            task_type=TaskType.RECALL,
            backend="episodic",
            predicate={},
        )

        # Reflect task
        reflect_task = Task(
            task_type=TaskType.REFLECT,
            backend="merger",
            depends_on=[working_task.id, episodic_task.id],
        )

        task_list = TaskList(
            tasks=[working_task, episodic_task, reflect_task],
            merge_strategy="reflect"
        )
        result = self.db.executor.execute(task_list)

        assert "llm_context" in result
