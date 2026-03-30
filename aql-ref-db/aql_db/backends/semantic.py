# aql_db/backends/semantic.py
"""Semantic memory backend - numpy cosine similarity."""

from typing import Any, Dict, List, Optional
from datetime import datetime
import hashlib

try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False

from .base import BaseBackend


class SemanticBackend(BaseBackend):
    """
    Semantic memory backend.

    Storage: list of records + embedding vectors
    Similarity: cosine similarity using numpy

    For reference impl, uses TF-IDF-like vectors from word frequencies.
    Replace with sentence-transformers for better accuracy.
    """

    def __init__(self):
        self._records: List[Dict[str, Any]] = []
        self._embeddings: List[Any] = []  # numpy arrays or lists
        self._vocab: Dict[str, int] = {}  # word -> index for TF-IDF

    def store(
        self,
        key: str,
        data: Dict[str, Any],
        scope: str = "private",
        namespace: Optional[str] = None,
        ttl: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Store a semantic concept."""
        sem_data = {
            "concept_id": key,
            "concept": data.get("concept", key),
            "knowledge": data.get("knowledge", ""),
            **{k: v for k, v in data.items()
               if k not in ("concept", "knowledge")}
        }

        record = self._make_record(
            id=key,
            memory_type="SEMANTIC",
            data=sem_data,
            scope=scope,
            namespace=namespace,
        )

        # Generate embedding from concept + knowledge text
        text = f"{sem_data['concept']} {sem_data['knowledge']}"
        embedding = self._text_to_embedding(text)

        self._records.append(record)
        self._embeddings.append(embedding)

        return record

    def lookup(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """Look up by exact concept match."""
        results = []
        for record in self._records:
            if self._matches_predicate(record, predicate):
                record["metadata"]["accessed_at"] = datetime.now().timestamp()
                results.append(record)
        return self._apply_modifiers(results, modifiers)

    def recall(
        self,
        predicate: Dict[str, Any],
        modifiers: Optional[Dict[str, Any]] = None,
    ) -> List[Dict[str, Any]]:
        """
        Similarity search using cosine similarity.

        Args:
            predicate: Should contain 'expression' or 'pattern' with search text
            modifiers: MIN_CONFIDENCE, LIMIT, etc.
        """
        search_text = predicate.get("expression") or predicate.get("pattern")

        if not search_text:
            return self.lookup(predicate, modifiers)

        min_confidence = modifiers.get("min_confidence", 0.0) if modifiers else 0.0
        query_embedding = self._text_to_embedding(str(search_text))

        results = []
        for i, record in enumerate(self._records):
            similarity = self._cosine_similarity(
                query_embedding, self._embeddings[i]
            )

            if similarity >= min_confidence:
                record_copy = record.copy()
                record_copy["_similarity"] = similarity
                record_copy["metadata"]["accessed_at"] = datetime.now().timestamp()
                results.append(record_copy)

        # Sort by similarity DESC
        results.sort(key=lambda r: r.get("_similarity", 0), reverse=True)

        return self._apply_modifiers(results, modifiers)

    def forget(self, predicate: Dict[str, Any]) -> int:
        """Remove matching semantic records."""
        indices_to_remove = []
        for i, record in enumerate(self._records):
            if self._matches_predicate(record, predicate):
                indices_to_remove.append(i)

        # Remove in reverse order to maintain indices
        for i in reversed(indices_to_remove):
            del self._records[i]
            del self._embeddings[i]

        return len(indices_to_remove)

    def update(
        self,
        predicate: Dict[str, Any],
        data: Dict[str, Any],
    ) -> int:
        """Update semantic records."""
        count = 0
        for i, record in enumerate(self._records):
            if self._matches_predicate(record, predicate):
                record["data"].update(data)
                record["metadata"]["accessed_at"] = datetime.now().timestamp()

                # Re-generate embedding if concept/knowledge changed
                if "concept" in data or "knowledge" in data:
                    text = f"{record['data']['concept']} {record['data']['knowledge']}"
                    self._embeddings[i] = self._text_to_embedding(text)

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

        # KEY concept = x
        key_value = predicate.get("key_value")
        if key_value:
            return record.get("id") == key_value or \
                   record.get("data", {}).get("concept") == key_value

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

        return True

    def _text_to_embedding(self, text: str) -> Any:
        """
        Convert text to embedding vector.

        Reference impl uses simple TF-IDF-like word frequency vectors.
        For production, use sentence-transformers.
        """
        words = text.lower().split()

        # Update vocabulary
        for word in words:
            if word not in self._vocab:
                self._vocab[word] = len(self._vocab)

        # Create sparse vector
        if HAS_NUMPY:
            vec = np.zeros(max(len(self._vocab), 1))
            for word in words:
                if word in self._vocab:
                    vec[self._vocab[word]] += 1
            # Normalize
            norm = np.linalg.norm(vec)
            if norm > 0:
                vec = vec / norm
            return vec
        else:
            # Fallback: use dict-based sparse vector
            vec = {}
            for word in words:
                vec[word] = vec.get(word, 0) + 1
            # Normalize
            total = sum(v * v for v in vec.values()) ** 0.5
            if total > 0:
                vec = {k: v / total for k, v in vec.items()}
            return vec

    def _cosine_similarity(self, vec1: Any, vec2: Any) -> float:
        """Compute cosine similarity between two vectors."""
        if HAS_NUMPY:
            # Pad vectors to same length
            max_len = max(len(vec1), len(vec2))
            v1 = np.zeros(max_len)
            v2 = np.zeros(max_len)
            v1[:len(vec1)] = vec1[:len(vec1)] if len(vec1) <= max_len else vec1[:max_len]
            v2[:len(vec2)] = vec2[:len(vec2)] if len(vec2) <= max_len else vec2[:max_len]

            dot = np.dot(v1, v2)
            norm1 = np.linalg.norm(v1)
            norm2 = np.linalg.norm(v2)

            if norm1 == 0 or norm2 == 0:
                return 0.0
            return float(dot / (norm1 * norm2))
        else:
            # Dict-based sparse vector similarity
            if isinstance(vec1, dict) and isinstance(vec2, dict):
                common_keys = set(vec1.keys()) & set(vec2.keys())
                dot = sum(vec1[k] * vec2[k] for k in common_keys)
                norm1 = sum(v * v for v in vec1.values()) ** 0.5
                norm2 = sum(v * v for v in vec2.values()) ** 0.5
                if norm1 == 0 or norm2 == 0:
                    return 0.0
                return dot / (norm1 * norm2)
            return 0.0
