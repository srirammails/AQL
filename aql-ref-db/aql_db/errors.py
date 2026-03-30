# aql_db/errors.py
"""Errors for AQL reference database."""


class ADBError(Exception):
    """Base error for ADB operations."""
    pass


class BackendError(ADBError):
    """Error in backend operation."""
    pass


class ExecutionError(ADBError):
    """Error during task execution."""
    pass
