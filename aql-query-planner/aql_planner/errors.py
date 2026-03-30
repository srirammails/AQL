# aql_planner/errors.py
"""Errors for AQL query planner."""


class PlannerError(Exception):
    """Base error for planner operations."""
    pass


class RoutingError(PlannerError):
    """Invalid verb/memory_type combination."""
    pass


class BudgetError(PlannerError):
    """Pipeline budget exceeded."""
    pass
