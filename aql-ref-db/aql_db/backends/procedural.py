# aql_db/backends/procedural.py
"""Procedural memory backend - networkx DiGraph for patterns/procedures."""

from typing import Any, Dict, List, Optional, Set
from datetime import datetime

try:
    import networkx as nx
    HAS_NETWORKX = True
except ImportError:
    HAS_NETWORKX = False

from .base import BaseBackend


class ProceduralBackend(BaseBackend):
    """
    Procedural memory backend.

    Storage: networkx DiGraph
    Nodes: patterns, procedures, steps
    Edges: relationships (next, requires, triggers)

    Pattern matching uses simple token overlap (Jaccard similarity)
    for the reference implementation.
    """

    def __init__(self):
        if not HAS_NETWORKX:
            # Fallback to simple dict if networkx not available
            self._storage: Dict[str, Dict[str, Any]] = {}
            self._graph = None
        else:
            self._graph = nx.DiGraph()
            self._storage = None

    def store(
        self,
        key: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Store a procedural pattern/procedure."""
        proc_data = {
            "pattern_id": key,
            "pattern": data.get("pattern", ""),
            "steps": data.get("steps", []),
            "severity": data.get("severity", "MEDIUM"),
            "confidence": data.get("confidence", 1.0),
            "source": data.get("source", "defined"),
            "success_count": data.get("success_count", 0),
            "failure_count": data.get("failure_count", 0),
            **{k: v for k, v in data.items()
               if k not in ("pattern", "steps", "severity", "confidence",
                            "source", "success_count", "failure_count")}
        }

        record = self._make_record(
            id=key,
            memory_type="PROCEDURAL",
            data=proc_data,
            scope=scope,
            namespace=namespace,
        )

        if self._graph is not None:
            self._graph.add_node(key, **record)
            # Add edges for step references
            steps = proc_data.get("steps", [])
            if isinstance(steps, str):
                steps = [s.strip() for s in steps.split(",")]
            for step in steps:
                if step in self._graph:
                    self._graph.add_edge(key, step, relation="uses")
        else:
            self._storage[key] = record

        return record

    def lookup(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Look up procedures by pattern_id or conditions."""
        results = []

        if self._graph is not None:
            for node_id in self._graph.nodes():
                record = self._graph.nodes[node_id]
                if self._matches_predicate(record, predicate):
                    results.append(record)
        else:
            for record in self._storage.values():
                if self._matches_predicate(record, predicate):
                    results.append(record)

        # Handle DEPTH modifier for graph traversal
        depth = modifiers.get("depth") if modifiers else None
        if depth and self._graph is not None and results:
            expanded = []
            for record in results:
                node_id = record["id"]
                ego = nx.ego_graph(self._graph, node_id, radius=depth)
                for n in ego.nodes():
                    node_record = self._graph.nodes[n]
                    if node_record not in expanded:
                        expanded.append(node_record)
            results = expanded

        return self._apply_modifiers(results, modifiers)

    def recall(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Pattern matching using similarity.

        Uses Jaccard similarity on word tokens for reference impl.
        """
        pattern_text = predicate.get("pattern") or predicate.get("expression")
        threshold = modifiers.get("threshold", 0.5) if modifiers else 0.5

        if not pattern_text:
            return self.lookup(predicate, modifiers)

        results = []
        pattern_tokens = self._tokenize(str(pattern_text))

        if self._graph is not None:
            nodes = self._graph.nodes()
        else:
            nodes = self._storage.keys()

        for node_id in nodes:
            if self._graph is not None:
                record = self._graph.nodes[node_id]
            else:
                record = self._storage[node_id]

            record_pattern = record.get("data", {}).get("pattern", "")
            record_tokens = self._tokenize(str(record_pattern))

            similarity = self._jaccard_similarity(pattern_tokens, record_tokens)

            if similarity >= threshold:
                record_copy = record.copy()
                record_copy["_similarity"] = similarity
                results.append(record_copy)

        # Sort by similarity DESC
        results.sort(key=lambda r: r.get("_similarity", 0), reverse=True)

        return self._apply_modifiers(results, modifiers)

    def forget(self, predicate: Dict[str, Any]) -> int:
        """Remove procedures matching predicate."""
        to_delete = []

        if self._graph is not None:
            for node_id in self._graph.nodes():
                record = self._graph.nodes[node_id]
                if self._matches_predicate(record, predicate):
                    to_delete.append(node_id)
            for node_id in to_delete:
                self._graph.remove_node(node_id)
        else:
            for key, record in self._storage.items():
                if self._matches_predicate(record, predicate):
                    to_delete.append(key)
            for key in to_delete:
                del self._storage[key]

        return len(to_delete)

    def update(
        self,
        predicate: Dict[str, Any],
        data: Dict[str, Any],
    ) -> int:
        """Update procedure records."""
        count = 0

        if self._graph is not None:
            for node_id in self._graph.nodes():
                record = self._graph.nodes[node_id]
                if self._matches_predicate(record, predicate):
                    record["data"].update(data)
                    record["metadata"]["accessed_at"] = datetime.now().timestamp()
                    count += 1
        else:
            for record in self._storage.values():
                if self._matches_predicate(record, predicate):
                    record["data"].update(data)
                    record["metadata"]["accessed_at"] = datetime.now().timestamp()
                    count += 1

        return count

    def _matches_predicate(
        self,
        record: Dict[str, Any],
        predicate: Dict[str, Any],
    ) -> bool:
        """Check if record matches predicate."""
        if not predicate:
            return True

        # Direct pattern_id lookup
        pattern_id = predicate.get("pattern_id")
        if pattern_id:
            return record.get("id") == pattern_id or \
                   record.get("data", {}).get("pattern_id") == pattern_id

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

        return True

    def _tokenize(self, text: str) -> Set[str]:
        """Simple word tokenization."""
        return set(text.lower().split())

    def _jaccard_similarity(self, set1: Set[str], set2: Set[str]) -> float:
        """Jaccard similarity between two sets."""
        if not set1 or not set2:
            return 0.0
        intersection = len(set1 & set2)
        union = len(set1 | set2)
        return intersection / union if union > 0 else 0.0
