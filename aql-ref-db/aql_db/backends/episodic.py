# aql_db/backends/episodic.py
"""Episodic memory backend - pandas DataFrame for time-series data."""

from typing import Any, Dict, List, Optional
from datetime import datetime, timedelta
import json

try:
    import pandas as pd
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False

from .base import BaseBackend


class EpisodicBackend(BaseBackend):
    """
    Episodic memory backend.

    Storage: pandas DataFrame (or list fallback)
    Columns: id, timestamp, data, scope, namespace, accessed_at

    Optimized for time-series queries with ORDER BY time.
    Updates accessed_at on reads to track active memories.
    """

    def __init__(self):
        if HAS_PANDAS:
            self._df = pd.DataFrame(columns=[
                "id", "timestamp", "data", "scope", "namespace",
                "memory_type", "accessed_at"
            ])
        else:
            self._records: List[Dict[str, Any]] = []

    def store(
        self,
        key: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Store an episodic memory."""
        now = datetime.now()
        timestamp = now.timestamp()

        record = self._make_record(
            id=key,
            memory_type="EPISODIC",
            data=data,
            scope=scope,
            namespace=namespace,
        )

        if HAS_PANDAS:
            row = {
                "id": key,
                "timestamp": timestamp,
                "data": json.dumps(data),
                "scope": scope,
                "namespace": namespace,
                "memory_type": "EPISODIC",
                "accessed_at": timestamp,
            }
            self._df = pd.concat([
                self._df,
                pd.DataFrame([row])
            ], ignore_index=True)
        else:
            self._records.append({
                **record,
                "timestamp": timestamp,
            })

        return record

    def lookup(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Look up episodes by exact conditions."""
        results = self._query(predicate)
        self._update_accessed(results)
        return self._apply_modifiers(results, modifiers)

    def recall(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Recall episodes by conditions.

        Episodic recall is essentially lookup with time-based defaults.
        """
        results = self._query(predicate)
        self._update_accessed(results)

        # Default ORDER BY time DESC for episodic
        if modifiers is None:
            modifiers = {}
        if "order_by" not in modifiers:
            modifiers["order_by"] = "timestamp"
            modifiers["order_dir"] = "DESC"

        return self._apply_modifiers(results, modifiers)

    def forget(self, predicate: Dict[str, Any]) -> int:
        """Remove episodes matching predicate."""
        if HAS_PANDAS:
            initial_len = len(self._df)
            mask = self._build_mask(predicate)
            self._df = self._df[~mask]
            return initial_len - len(self._df)
        else:
            to_remove = []
            for i, record in enumerate(self._records):
                if self._matches_predicate(record, predicate):
                    to_remove.append(i)
            for i in reversed(to_remove):
                del self._records[i]
            return len(to_remove)

    def update(
        self,
        predicate: Dict[str, Any],
        data: Dict[str, Any],
    ) -> int:
        """Update episode records."""
        count = 0
        if HAS_PANDAS:
            mask = self._build_mask(predicate)
            for idx in self._df[mask].index:
                existing_data = json.loads(self._df.at[idx, "data"])
                existing_data.update(data)
                self._df.at[idx, "data"] = json.dumps(existing_data)
                self._df.at[idx, "accessed_at"] = datetime.now().timestamp()
                count += 1
        else:
            for record in self._records:
                if self._matches_predicate(record, predicate):
                    record["data"].update(data)
                    record["metadata"]["accessed_at"] = datetime.now().timestamp()
                    count += 1
        return count

    def _query(self, predicate: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Query episodes by predicate."""
        if HAS_PANDAS:
            if self._df.empty:
                return []

            mask = self._build_mask(predicate)
            filtered = self._df[mask]

            results = []
            for _, row in filtered.iterrows():
                data = json.loads(row["data"]) if isinstance(row["data"], str) else row["data"]
                results.append({
                    "id": row["id"],
                    "memory_type": "EPISODIC",
                    "data": data,
                    "metadata": {
                        "created_at": row["timestamp"],
                        "accessed_at": row["accessed_at"],
                        "scope": row["scope"],
                        "namespace": row["namespace"],
                    },
                    "timestamp": row["timestamp"],
                })
            return results
        else:
            results = []
            for record in self._records:
                if self._matches_predicate(record, predicate):
                    results.append(record)
            return results

    def _build_mask(self, predicate: Dict[str, Any]):
        """Build pandas boolean mask from predicate."""
        if not HAS_PANDAS or self._df.empty:
            return pd.Series([False] * len(self._df)) if HAS_PANDAS else []

        mask = pd.Series([True] * len(self._df))

        conditions = predicate.get("conditions", [])
        for cond in conditions:
            field = cond.get("field")
            op = cond.get("op")
            value = cond.get("value")

            # Check if field is in data JSON
            if field not in self._df.columns:
                # Extract from data JSON
                data_values = self._df["data"].apply(
                    lambda x: json.loads(x).get(field) if isinstance(x, str) else x.get(field)
                )
            else:
                data_values = self._df[field]

            if op in ("=", "EQ"):
                mask &= (data_values == value)
            elif op in ("!=", "NEQ"):
                mask &= (data_values != value)
            elif op in (">", "GT"):
                # Handle time comparisons
                if isinstance(value, (int, float)) and field in ("last_accessed", "time"):
                    # Assume value is days
                    cutoff = datetime.now().timestamp() - (value * 86400)
                    mask &= (self._df["accessed_at"] < cutoff)
                else:
                    mask &= (data_values > value)
            elif op in ("<", "LT"):
                mask &= (data_values < value)

        return mask

    def _matches_predicate(
        self,
        record: Dict[str, Any],
        predicate: Dict[str, Any],
    ) -> bool:
        """Check if record matches predicate (for non-pandas fallback)."""
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
                if field in ("last_accessed", "time"):
                    cutoff = datetime.now().timestamp() - (value * 86400)
                    if record.get("metadata", {}).get("accessed_at", 0) >= cutoff:
                        return False
                elif not (record_value and record_value > value):
                    return False

        return True

    def _update_accessed(self, results: List[Dict[str, Any]]):
        """Update accessed_at for returned records."""
        now = datetime.now().timestamp()
        if HAS_PANDAS:
            ids = [r["id"] for r in results]
            mask = self._df["id"].isin(ids)
            self._df.loc[mask, "accessed_at"] = now
        else:
            for result in results:
                result["metadata"]["accessed_at"] = now

    def _apply_modifiers(
        self,
        records: List[Dict[str, Any]],
        modifiers: Optional[Dict[str, Any]],
    ) -> List[Dict[str, Any]]:
        """Apply modifiers with episodic-specific handling."""
        if not modifiers:
            return records

        result = records.copy()

        # ORDER BY (default to timestamp for episodic)
        order_by = modifiers.get("order_by", "timestamp")
        direction = modifiers.get("order_dir", "DESC")
        reverse = direction == "DESC"

        def get_sort_key(r):
            if order_by == "timestamp" or order_by == "time":
                return r.get("timestamp") or r.get("metadata", {}).get("created_at", 0)
            return r.get("data", {}).get(order_by) or r.get("metadata", {}).get(order_by, 0)

        result.sort(key=get_sort_key, reverse=reverse)

        # LIMIT
        limit = modifiers.get("limit")
        if limit:
            result = result[:limit]

        return result
