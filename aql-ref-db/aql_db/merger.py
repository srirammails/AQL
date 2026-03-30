# aql_db/merger.py
"""Result merger for REFLECT - assembles context for LLM."""

from typing import Any, Dict, List, Optional
from datetime import datetime


class ResultMerger:
    """
    Combines results from multiple memory backends
    into a single context package for the LLM.
    """

    def merge(self, source_results: Dict[str, Any]) -> Dict[str, Any]:
        """
        Merge results from multiple source tasks.

        Args:
            source_results: Dict mapping task_id to result

        Returns:
            Merged context with llm_context string
        """
        context = {
            "episodic": [],
            "semantic": [],
            "procedural": [],
            "working": {},
            "tools": [],
            "assembled_at": datetime.now().isoformat(),
        }

        for task_id, result in source_results.items():
            if not result:
                continue

            memory_type = result.get("memory_type", "").lower()
            records = result.get("records", [])

            if memory_type == "episodic":
                context["episodic"].extend(records)
            elif memory_type == "semantic":
                context["semantic"].extend(records)
            elif memory_type == "procedural":
                context["procedural"].extend(records)
            elif memory_type == "working":
                # Working memory merges into single state dict
                for record in records:
                    if isinstance(record, dict) and "data" in record:
                        context["working"].update(record["data"])
                    elif isinstance(record, dict):
                        context["working"].update(record)
            elif memory_type == "tools":
                context["tools"].extend(records)

        # Build the LLM context string
        context["llm_context"] = self.format_for_llm(context)

        return context

    def format_for_llm(self, context: Dict[str, Any]) -> str:
        """
        Format assembled memory as a readable context string
        for the LLM to reason over.

        Args:
            context: Merged context dict

        Returns:
            Formatted string for LLM consumption
        """
        parts = []

        # Working memory - current task state
        if context.get("working"):
            parts.append("## Current Task State")
            for key, value in context["working"].items():
                parts.append(f"- {key}: {value}")
            parts.append("")

        # Available tools
        if context.get("tools"):
            parts.append("## Available Tools")
            for tool in context["tools"]:
                if isinstance(tool, dict):
                    data = tool.get("data", tool)
                    tool_id = data.get("tool_id", "unknown")
                    desc = data.get("description", "")
                    ranking = data.get("ranking", 0)
                    parts.append(f"- **{tool_id}** (rank: {ranking:.2f}): {desc}")
            parts.append("")

        # Semantic knowledge
        if context.get("semantic"):
            parts.append("## Relevant Knowledge")
            for item in context["semantic"]:
                if isinstance(item, dict):
                    data = item.get("data", item)
                    concept = data.get("concept", "")
                    knowledge = data.get("knowledge", "")
                    parts.append(f"- **{concept}**: {knowledge}")
            parts.append("")

        # Episodic history
        if context.get("episodic"):
            parts.append("## Relevant History")
            for item in context["episodic"]:
                if isinstance(item, dict):
                    data = item.get("data", item)
                    # Format episode data
                    episode_str = ", ".join(f"{k}={v}" for k, v in data.items())
                    parts.append(f"- {episode_str}")
            parts.append("")

        # Procedural patterns
        if context.get("procedural"):
            parts.append("## Available Procedures")
            for item in context["procedural"]:
                if isinstance(item, dict):
                    data = item.get("data", item)
                    pattern_id = data.get("pattern_id", "")
                    steps = data.get("steps", [])
                    if isinstance(steps, str):
                        steps = steps.split(",")
                    parts.append(f"- **{pattern_id}**: {' -> '.join(steps)}")
            parts.append("")

        return "\n".join(parts).strip()
