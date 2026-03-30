# aql_db/backends/base.py
"""Base backend interface that all backends implement."""

from abc import ABC, abstractmethod
from typing import Any, Dict, List, Optional
from datetime import datetime


class BaseBackend(ABC):
    """
    Abstract base class for all ADB backends.

    All backends return records in a standard format:
    {
        "id": str,
        "memory_type": str,
        "data": dict,
        "metadata": {
            "created_at": float (timestamp),
            "accessed_at": float (timestamp),
            "scope": str,
            "namespace": str | None,
        }
    }
    """

    @abstractmethod
    def store(
        self,
        key: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Write a new record.

        Args:
            key: Primary identifier
            data: Payload dictionary
            scope: "private" | "shared" | "cluster"
            namespace: Agent identity
            ttl: Time-to-live in seconds (optional)

        Returns:
            The stored record
        """
        pass

    @abstractmethod
    def lookup(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Find by exact key or condition.

        Args:
            predicate: Search conditions
            modifiers: LIMIT, ORDER BY, etc.

        Returns:
            List of matching records
        """
        pass

    @abstractmethod
    def recall(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Find by similarity or context.

        For non-vector backends: falls back to lookup.
        For semantic: cosine similarity search.

        Args:
            predicate: Search conditions (may include embedding/text)
            modifiers: LIMIT, MIN_CONFIDENCE, etc.

        Returns:
            List of matching records
        """
        pass

    @abstractmethod
    def forget(self, predicate: Dict[str, Any]) -> int:
        """
        Delete matching records.

        Args:
            predicate: Conditions for deletion

        Returns:
            Count of deleted records
        """
        pass

    def scan(self) -> List[Dict[str, Any]]:
        """
        Return all records.

        Default implementation raises error - only WorkingBackend overrides.
        """
        raise NotImplementedError("SCAN ALL only supported for WORKING memory")

    @abstractmethod
    def update(
        self,
        predicate: Dict[str, Any],
        data: Dict[str, Any],
    ) -> int:
        """
        Modify matching records.

        Args:
            predicate: Conditions for update
            data: Fields to update

        Returns:
            Count of updated records
        """
        pass

    def _make_record(
        self,
        id: str,
        memory_type: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Helper to create standard record format."""
        now = datetime.now().timestamp()
        return {
            "id": id,
            "memory_type": memory_type,
            "data": data,
            "metadata": {
                "created_at": now,
                "accessed_at": now,
                "scope": scope,
                "namespace": namespace,
            }
        }

    def _apply_modifiers(
        self,
        records: List[Dict[str, Any]],
        modifiers: Optional[Dict[str, Any]],
    ) -> List[Dict[str, Any]]:
        """Apply common modifiers (ORDER BY, LIMIT) to results."""
        if not modifiers:
            return records

        result = records.copy()

        # ORDER BY
        order_by = modifiers.get("order_by")
        if order_by:
            direction = modifiers.get("order_dir", "ASC")
            reverse = direction == "DESC"
            result.sort(
                key=lambda r: r.get("data", {}).get(order_by)
                or r.get("metadata", {}).get(order_by, 0),
                reverse=reverse,
            )

        # LIMIT
        limit = modifiers.get("limit")
        if limit:
            result = result[:limit]

        return result
