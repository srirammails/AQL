# aql_db/backends/working.py
"""Working memory backend - dict-based, supports SCAN ALL."""

from typing import Any, Dict, List, Optional
from datetime import datetime
import uuid

from .base import BaseBackend


class WorkingBackend(BaseBackend):
    """
    Working memory backend.

    Storage: Python dict (key -> record)
    Access pattern: O(1) lookup, O(n) scan
    Special: Only backend that supports SCAN ALL
    TTL: Records can expire automatically
    """

    def __init__(self):
        self._storage: Dict[str, Dict[str, Any]] = {}
        self._ttls: Dict[str, float] = {}  # key -> expiry timestamp

    def store(
        self,
        key: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Store a record in working memory."""
        record = self._make_record(
            id=key,
            memory_type="WORKING",
            data=data,
            scope=scope,
            namespace=namespace,
        )
        self._storage[key] = record

        if ttl:
            self._ttls[key] = datetime.now().timestamp() + ttl

        return record

    def lookup(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Look up records by predicate."""
        self._expire_records()
        results = self._filter_by_predicate(predicate)
        return self._apply_modifiers(results, modifiers)

    def recall(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Working memory doesn't do similarity - falls back to lookup."""
        return self.lookup(predicate, modifiers)

    def scan(self) -> List[Dict[str, Any]]:
        """Return all records - only WORKING supports this."""
        self._expire_records()
        return list(self._storage.values())

    def forget(self, predicate: Dict[str, Any]) -> int:
        """Delete matching records."""
        self._expire_records()
        to_delete = []
        for key, record in self._storage.items():
            if self._matches_predicate(record, predicate):
                to_delete.append(key)

        for key in to_delete:
            del self._storage[key]
            self._ttls.pop(key, None)

        return len(to_delete)

    def update(
        self,
        predicate: Dict[str, Any],
        data: Dict[str, Any],
    ) -> int:
        """Update matching records."""
        self._expire_records()
        count = 0
        for record in self._storage.values():
            if self._matches_predicate(record, predicate):
                record["data"].update(data)
                record["metadata"]["accessed_at"] = datetime.now().timestamp()
                count += 1
        return count

    def _expire_records(self):
        """Remove expired records."""
        now = datetime.now().timestamp()
        expired = [k for k, exp in self._ttls.items() if exp <= now]
        for key in expired:
            self._storage.pop(key, None)
            self._ttls.pop(key, None)

    def _filter_by_predicate(
        self,
        predicate: Dict[str, Any],
    ) -> List[Dict[str, Any]]:
        """Filter records by predicate conditions."""
        results = []
        for record in self._storage.values():
            if self._matches_predicate(record, predicate):
                # Update accessed_at on read
                record["metadata"]["accessed_at"] = datetime.now().timestamp()
                results.append(record)
        return results

    def _matches_predicate(
        self,
        record: Dict[str, Any],
        predicate: Dict[str, Any],
    ) -> bool:
        """Check if a record matches the predicate."""
        if not predicate:
            return True

        conditions = predicate.get("conditions", [])
        if not conditions:
            # Check for direct key match
            key_value = predicate.get("key_value")
            if key_value and record["id"] != key_value:
                return False
            return True

        # Check all conditions
        for cond in conditions:
            field = cond.get("field")
            op = cond.get("op")
            value = cond.get("value")

            # Get field value from record data
            record_value = record.get("data", {}).get(field)

            if not self._compare(record_value, op, value):
                return False

        return True

    def _compare(self, record_value: Any, op: str, value: Any) -> bool:
        """Compare record value against condition."""
        if record_value is None:
            return False

        if op == "=" or op == "EQ":
            return record_value == value
        elif op == "!=" or op == "NEQ":
            return record_value != value
        elif op == ">" or op == "GT":
            return record_value > value
        elif op == "<" or op == "LT":
            return record_value < value
        elif op == ">=" or op == "GTE":
            return record_value >= value
        elif op == "<=" or op == "LTE":
            return record_value <= value
        else:
            return record_value == value
