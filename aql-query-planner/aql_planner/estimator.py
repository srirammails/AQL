# aql_planner/estimator.py
"""Latency estimation and budget allocation for tasks."""

import warnings
from typing import Dict, List, Optional

# Default latency estimates in milliseconds per backend
LATENCY_ESTIMATES: Dict[str, int] = {
    "working": 1,
    "tools": 2,
    "procedural": 5,
    "semantic": 20,
    "episodic": 50,
    "merger": 5,
    "graph": 10,
    "pipeline": 0,  # Pipeline itself has no latency, stages do
}

# Minimum budget allocation per task (prevents starvation)
MIN_BUDGET_MS = 5


def estimate(backend: str) -> int:
    """
    Get the estimated latency for a backend operation.

    Args:
        backend: Backend name

    Returns:
        Estimated milliseconds for the operation
    """
    return LATENCY_ESTIMATES.get(backend, 10)


def allocate_budget(
    backends: List[str],
    total_budget_ms: int
) -> List[int]:
    """
    Allocate time budget across multiple backends proportionally.

    Args:
        backends: List of backend names in execution order
        total_budget_ms: Total time budget in milliseconds

    Returns:
        List of allocated milliseconds per backend
    """
    if not backends:
        return []

    # Calculate total estimated time
    estimates = [estimate(b) for b in backends]
    total_estimate = sum(estimates)

    # Check if we're over budget
    if total_estimate > total_budget_ms:
        warnings.warn(
            f"Pipeline may exceed timeout: estimated {total_estimate}ms, "
            f"budget {total_budget_ms}ms",
            RuntimeWarning
        )

    # Allocate proportionally
    if total_estimate == 0:
        # Equal distribution if no estimates
        per_task = total_budget_ms // len(backends)
        return [per_task] * len(backends)

    allocations = []
    remaining = total_budget_ms

    for i, est in enumerate(estimates):
        if i == len(estimates) - 1:
            # Last task gets remaining budget
            alloc = max(MIN_BUDGET_MS, remaining)
        else:
            # Proportional allocation
            proportion = est / total_estimate
            alloc = max(MIN_BUDGET_MS, int(total_budget_ms * proportion))
            remaining -= alloc

        allocations.append(alloc)

    return allocations


def estimate_pipeline(backends: List[str]) -> int:
    """
    Estimate total time for a pipeline.

    Args:
        backends: List of backend names

    Returns:
        Total estimated milliseconds
    """
    return sum(estimate(b) for b in backends)
