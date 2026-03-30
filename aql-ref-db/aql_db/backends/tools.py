# aql_db/backends/tools.py
"""Tool registry backend - dict with ranking scores."""

from typing import Any, Dict, List, Optional
from datetime import datetime

from .base import BaseBackend


class ToolsBackend(BaseBackend):
    """
    Tool registry backend.

    Storage: dict of tool records + ranking scores
    Each tool has ranking, call_count, and relevance_scores
    Rankings persist across sessions - agents get smarter over time
    """

    def __init__(self):
        self._tools: Dict[str, Dict[str, Any]] = {}

    def store(
        self,
        key: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Register a new tool."""
        # Ensure tool has required fields
        tool_data = {
            "tool_id": key,
            "schema": data.get("schema", {}),
            "description": data.get("description", ""),
            "category": data.get("category", "general"),
            "ranking": data.get("ranking", 0.5),
            "call_count": data.get("call_count", 0),
            "last_called": None,
            "relevance_scores": data.get("relevance_scores", {}),
            **{k: v for k, v in data.items()
               if k not in ("schema", "description", "category", "ranking",
                            "call_count", "relevance_scores")}
        }

        record = self._make_record(
            id=key,
            memory_type="TOOLS",
            data=tool_data,
            scope=scope,
            namespace=namespace,
        )
        self._tools[key] = record
        return record

    def lookup(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Look up tools by predicate (category, tool_id, etc.)."""
        results = self._filter_tools(predicate)
        return self._apply_modifiers(results, modifiers)

    def recall(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Tool recall uses relevance scores for ranking."""
        results = self._filter_tools(predicate)

        # Apply relevance threshold if present
        threshold = modifiers.get("threshold") if modifiers else None
        if threshold:
            results = [
                r for r in results
                if r["data"].get("ranking", 0) >= threshold
            ]

        # Sort by ranking DESC by default for tools
        results.sort(key=lambda r: r["data"].get("ranking", 0), reverse=True)

        return self._apply_modifiers(results, modifiers)

    def forget(self, predicate: Dict[str, Any]) -> int:
        """Remove tools matching predicate."""
        to_delete = []
        for key, record in self._tools.items():
            if self._matches_predicate(record, predicate):
                to_delete.append(key)

        for key in to_delete:
            del self._tools[key]

        return len(to_delete)

    def update(
        self,
        predicate: Dict[str, Any],
        data: Dict[str, Any],
    ) -> int:
        """Update tool records (ranking, call_count, etc.)."""
        count = 0
        for record in self._tools.values():
            if self._matches_predicate(record, predicate):
                # Handle special ranking update
                if "ranking" in data:
                    old_ranking = record["data"].get("ranking", 0.5)
                    # Exponential moving average
                    record["data"]["ranking"] = min(1.0, data["ranking"])

                # Handle call_count increment
                if "call_count" in data:
                    record["data"]["call_count"] = data["call_count"]
                    record["data"]["last_called"] = datetime.now().timestamp()

                # Update other fields
                for k, v in data.items():
                    if k not in ("ranking", "call_count"):
                        record["data"][k] = v

                record["metadata"]["accessed_at"] = datetime.now().timestamp()
                count += 1
        return count

    def update_ranking(self, tool_id: str, success: bool) -> float:
        """
        Update tool ranking based on success/failure signal.

        Uses exponential moving average:
        new_ranking = old_ranking * 0.9 + success_signal * 0.1

        Args:
            tool_id: Tool identifier
            success: Whether the tool call was successful

        Returns:
            New ranking value
        """
        if tool_id not in self._tools:
            return 0.0

        record = self._tools[tool_id]
        old_ranking = record["data"].get("ranking", 0.5)
        success_signal = 1.0 if success else 0.0
        new_ranking = old_ranking * 0.9 + success_signal * 0.1
        record["data"]["ranking"] = new_ranking
        record["data"]["call_count"] = record["data"].get("call_count", 0) + 1
        record["data"]["last_called"] = datetime.now().timestamp()
        return new_ranking

    def _filter_tools(self, predicate: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Filter tools by predicate."""
        results = []
        for record in self._tools.values():
            if self._matches_predicate(record, predicate):
                results.append(record)
        return results

    def _matches_predicate(
        self,
        record: Dict[str, Any],
        predicate: Dict[str, Any],
    ) -> bool:
        """Check if tool matches predicate."""
        if not predicate:
            return True

        conditions = predicate.get("conditions", [])
        if not conditions:
            return True

        for cond in conditions:
            field = cond.get("field")
            op = cond.get("op")
            value = cond.get("value")

            record_value = record.get("data", {}).get(field)

            if op in ("=", "EQ"):
                if record_value != value:
                    return False
            elif op in (">", "GT"):
                if not (record_value and record_value > value):
                    return False
            elif op in ("<", "LT"):
                if not (record_value and record_value < value):
                    return False
            elif op in (">=", "GTE"):
                if not (record_value and record_value >= value):
                    return False
            elif op in ("<=", "LTE"):
                if not (record_value and record_value <= value):
                    return False

        return True
