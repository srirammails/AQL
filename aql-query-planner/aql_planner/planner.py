# aql_planner/planner.py
"""Main query planner - orchestrates routing and budget allocation."""

from typing import Any, Optional

from aql_parser import (
    ExecutionPlan, Verb, MemoryType,
    LimitMod, OrderMod, ThresholdMod, ConfidenceMod,
    ReturnMod, SourceMod, WeightMod, ScopeMod, NamespaceMod
)

from .task import Task, TaskList, TaskType
from .router import route
from .estimator import estimate, allocate_budget
from .errors import PlannerError


# Map AQL Verbs to TaskTypes
VERB_TO_TASK_TYPE: dict[Verb, TaskType] = {
    Verb.LOOKUP: TaskType.LOOKUP,
    Verb.RECALL: TaskType.RECALL,
    Verb.SCAN: TaskType.SCAN,
    Verb.LOAD: TaskType.LOAD,
    Verb.STORE: TaskType.STORE,
    Verb.UPDATE: TaskType.UPDATE,
    Verb.FORGET: TaskType.FORGET,
    Verb.LINK: TaskType.LINK,
    Verb.REFLECT: TaskType.REFLECT,
    Verb.PIPELINE: TaskType.MERGE,  # Pipeline produces merged results
}


def extract_predicate(plan: ExecutionPlan) -> Optional[dict]:
    """Extract predicate information from execution plan."""
    if plan.predicate is None:
        return None

    pred = plan.predicate
    return {
        "type": pred.pred_type.value if hasattr(pred, 'pred_type') else "unknown",
        "conditions": [
            {
                "field": c.field,
                "op": c.op.value if hasattr(c.op, 'value') else str(c.op),
                "value": c.value
            }
            for c in (pred.conditions if hasattr(pred, 'conditions') else [])
        ] if hasattr(pred, 'conditions') else [],
        "pattern": pred.pattern if hasattr(pred, 'pattern') else None,
        "key_field": pred.key_field if hasattr(pred, 'key_field') else None,
        "key_value": pred.key_value if hasattr(pred, 'key_value') else None,
    }


def extract_modifiers(plan: ExecutionPlan) -> dict:
    """Extract modifier information from execution plan."""
    mods = {}

    limit = plan.get_modifier(LimitMod)
    if limit:
        mods["limit"] = limit.value

    order = plan.get_modifier(OrderMod)
    if order:
        mods["order_by"] = order.field
        mods["order_dir"] = order.direction

    threshold = plan.get_modifier(ThresholdMod)
    if threshold:
        mods["threshold"] = threshold.value

    confidence = plan.get_modifier(ConfidenceMod)
    if confidence:
        mods["min_confidence"] = confidence.value

    return_mod = plan.get_modifier(ReturnMod)
    if return_mod:
        mods["return_fields"] = return_mod.fields

    source = plan.get_modifier(SourceMod)
    if source:
        mods["sources"] = source.sources

    weight = plan.get_modifier(WeightMod)
    if weight:
        mods["weight_field"] = weight.field

    return mods


def extract_payload(plan: ExecutionPlan) -> Optional[dict]:
    """Extract payload data from STORE/UPDATE plans."""
    if plan.payload is None:
        return None

    return {
        field: value
        for field, value in plan.payload.fields.items()
    } if hasattr(plan.payload, 'fields') else None


def extract_scope(plan: ExecutionPlan) -> tuple[str, Optional[str]]:
    """Extract scope and namespace from plan."""
    scope = "private"
    namespace = None

    if plan.scope:
        scope = plan.scope.value.value if hasattr(plan.scope.value, 'value') else str(plan.scope.value)

    if plan.namespace:
        namespace = plan.namespace.value

    return scope, namespace


def plan_simple(execution_plan: ExecutionPlan) -> TaskList:
    """
    Plan a simple (non-pipeline, non-reflect) query.

    Args:
        execution_plan: Parsed AQL execution plan

    Returns:
        TaskList with a single task
    """
    backend = route(execution_plan.verb, execution_plan.memory_type)
    task_type = VERB_TO_TASK_TYPE.get(execution_plan.verb, TaskType.LOOKUP)
    scope, namespace = extract_scope(execution_plan)

    task = Task(
        task_type=task_type,
        backend=backend,
        predicate=extract_predicate(execution_plan),
        payload=extract_payload(execution_plan),
        modifiers=extract_modifiers(execution_plan),
        budget_ms=estimate(backend),
        scope=scope,
        namespace=namespace,
    )

    return TaskList(
        tasks=[task],
        total_budget_ms=task.budget_ms,
        merge_strategy="sequential"
    )


def plan_reflect(execution_plan: ExecutionPlan) -> TaskList:
    """
    Plan a REFLECT query with multiple INCLUDE sources.

    Args:
        execution_plan: Parsed REFLECT execution plan

    Returns:
        TaskList with source tasks + merge task + optional write task
    """
    tasks = []

    # Create a task for each INCLUDE source
    for source in execution_plan.sources:
        memory_type = source.memory_type

        # Route based on memory type - use SCAN for WORKING, RECALL for others
        if memory_type == MemoryType.WORKING:
            backend = route(Verb.SCAN, memory_type)
            task_type = TaskType.SCAN
        else:
            backend = route(Verb.RECALL, memory_type)
            task_type = TaskType.RECALL

        # Build predicate from source
        pred = {"memory_type": memory_type.value}
        if source.predicate:
            pred["source_predicate"] = {
                "type": source.predicate.type,
                "condition": source.predicate.condition,
                "expression": source.predicate.expression,
            }

        task = Task(
            task_type=task_type,
            backend=backend,
            predicate=pred,
            budget_ms=estimate(backend),
        )
        tasks.append(task)

    # Create merge task that depends on all source tasks
    merge_task = Task(
        task_type=TaskType.REFLECT,
        backend="merger",
        depends_on=[t.id for t in tasks],
        budget_ms=estimate("merger"),
    )
    tasks.append(merge_task)

    # If THEN clause exists, add write-back task
    if execution_plan.then_stmt:
        then_plan = execution_plan.then_stmt
        then_tasks = plan_simple(then_plan)
        write_task = then_tasks.tasks[0]
        write_task.depends_on = [merge_task.id]
        tasks.append(write_task)

    return TaskList(
        tasks=tasks,
        total_budget_ms=sum(t.budget_ms for t in tasks),
        merge_strategy="reflect"
    )


def plan_pipeline(execution_plan: ExecutionPlan) -> TaskList:
    """
    Plan a PIPELINE query with multiple stages.

    Args:
        execution_plan: Parsed PIPELINE execution plan

    Returns:
        TaskList with ordered, dependent tasks
    """
    # Get total budget from timeout
    total_budget_ms = 100  # default
    if execution_plan.timeout:
        total_budget_ms = execution_plan.timeout.value
        if execution_plan.timeout.unit == "s":
            total_budget_ms *= 1000

    # Collect backends for budget allocation
    backends = []
    stage_plans = []

    for stage in execution_plan.stages:
        if stage.verb == Verb.REFLECT:
            # REFLECT in pipeline gets special handling
            reflect_tasks = plan_reflect(stage)
            for t in reflect_tasks.tasks:
                backends.append(t.backend)
            stage_plans.append(("reflect", reflect_tasks))
        else:
            backend = route(stage.verb, stage.memory_type)
            backends.append(backend)
            stage_plans.append(("simple", stage))

    # Allocate budget across backends
    allocations = allocate_budget(backends, total_budget_ms)

    # Build tasks with dependencies
    tasks = []
    alloc_idx = 0

    for plan_type, stage_data in stage_plans:
        if plan_type == "reflect":
            # Add all reflect tasks
            reflect_tasks = stage_data
            for t in reflect_tasks.tasks:
                t.budget_ms = allocations[alloc_idx]
                if tasks:
                    t.depends_on.append(tasks[-1].id)
                tasks.append(t)
                alloc_idx += 1
        else:
            # Simple stage
            stage = stage_data
            backend = route(stage.verb, stage.memory_type)
            task_type = VERB_TO_TASK_TYPE.get(stage.verb, TaskType.LOOKUP)
            scope, namespace = extract_scope(stage)

            task = Task(
                task_type=task_type,
                backend=backend,
                predicate=extract_predicate(stage),
                payload=extract_payload(stage),
                modifiers=extract_modifiers(stage),
                budget_ms=allocations[alloc_idx],
                scope=scope,
                namespace=namespace,
                depends_on=[tasks[-1].id] if tasks else [],
            )
            tasks.append(task)
            alloc_idx += 1

    return TaskList(
        tasks=tasks,
        total_budget_ms=total_budget_ms,
        pipeline_name=execution_plan.pipeline_name,
        merge_strategy="sequential"
    )


def plan(execution_plan: ExecutionPlan) -> TaskList:
    """
    Main entry point - plan any AQL query.

    Args:
        execution_plan: Parsed AQL execution plan from aql-parser

    Returns:
        TaskList ready for execution

    Raises:
        PlannerError: If the plan cannot be created
    """
    if execution_plan.verb == Verb.PIPELINE:
        return plan_pipeline(execution_plan)

    if execution_plan.verb == Verb.REFLECT:
        return plan_reflect(execution_plan)

    return plan_simple(execution_plan)
