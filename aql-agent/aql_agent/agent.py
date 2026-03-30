# aql_agent/agent.py
"""Simple agent demonstrating ADB + AQL memory system."""

from typing import Any, Dict, Optional
import os

from aql_db import ADB


class SimpleAgent:
    """
    Minimal agent demonstrating ADB + AQL.

    Uses Claude via Anthropic API for LLM reasoning (optional).
    Memory provided by ADB reference implementation.

    Usage:
        agent = SimpleAgent()  # No LLM, just memory demo
        agent = SimpleAgent(api_key="...")  # With Claude LLM

        # Seed knowledge
        agent.db.execute('STORE SEMANTIC (concept="x", knowledge="...")')

        # Run on event
        decision = agent.run("ALERT: pod crashed")
    """

    def __init__(self, api_key: Optional[str] = None):
        """
        Initialize agent with memory.

        Args:
            api_key: Optional Anthropic API key for LLM reasoning.
                     If not provided, agent returns assembled context.
        """
        self.db = ADB()
        self.api_key = api_key or os.environ.get("ANTHROPIC_API_KEY")
        self._client = None

    @property
    def client(self):
        """Lazy-load Anthropic client."""
        if self._client is None and self.api_key:
            try:
                import anthropic
                self._client = anthropic.Anthropic(api_key=self.api_key)
            except ImportError:
                pass
        return self._client

    def run(
        self,
        event: str,
        agent_id: str = "agent-001",
        use_llm: bool = True,
    ) -> Dict[str, Any]:
        """
        Process an event using agent memory.

        Args:
            event: Event string to process
            agent_id: Unique agent identifier
            use_llm: Whether to send to LLM (requires API key)

        Returns:
            Dict with decision and context
        """
        # Step 1: Store current event in working memory
        self.db.execute(f'''
            STORE WORKING (
                event = "{self._escape(event)}",
                agent_id = "{agent_id}",
                status = "processing"
            )
            SCOPE private
        ''')

        # Step 2: Assemble context from all memory types
        context = self.db.execute(f'''
            REFLECT agent_id = "{agent_id}"
            INCLUDE WORKING
            INCLUDE SEMANTIC
            INCLUDE EPISODIC
        ''')

        llm_context = context.get("llm_context", "")

        # Step 3: Get LLM decision (if available)
        decision = None
        if use_llm and self.client:
            decision = self._get_llm_decision(event, llm_context)

        # Step 4: Store as new episode (always, even without LLM)
        decision_short = self._escape(decision[:200]) if decision else "processed"
        self.db.execute(f'''
            STORE EPISODIC (
                agent_id = "{agent_id}",
                event = "{self._escape(event[:100])}",
                decision = "{decision_short}",
                status = "decided"
            )
        ''')

        # Step 5: Clear working memory
        self.db.execute(f'FORGET WORKING WHERE agent_id = "{agent_id}"')

        return {
            "event": event,
            "context": llm_context,
            "decision": decision,
            "memory_stats": {
                "episodic_count": len(context.get("episodic", [])),
                "semantic_count": len(context.get("semantic", [])),
                "procedural_count": len(context.get("procedural", [])),
            }
        }

    def _get_llm_decision(self, event: str, context: str) -> Optional[str]:
        """Send context to LLM and get decision."""
        if not self.client:
            return None

        try:
            response = self.client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=1000,
                messages=[{
                    "role": "user",
                    "content": f"""You are an autonomous agent with memory.

Your memory context:
{context}

Current event: {event}

What should you do next? Be specific and actionable.
Provide a concise decision (2-3 sentences max)."""
                }]
            )
            return response.content[0].text
        except Exception as e:
            return f"Error getting decision: {e}"

    def _escape(self, text: str) -> str:
        """Escape text for AQL queries."""
        return text.replace('"', '\\"').replace('\n', ' ')

    def seed_k8s_knowledge(self):
        """Pre-seed agent with Kubernetes domain knowledge."""
        self.db.execute('''
            STORE PROCEDURAL (
                pattern_id = "oom-kill-001",
                pattern = "OOMKilled pod memory limit exceeded",
                steps = "check memory limits,scale pod,notify team",
                severity = "HIGH"
            )
        ''')

        self.db.execute('''
            STORE PROCEDURAL (
                pattern_id = "cpu-throttle-001",
                pattern = "CPU throttling high utilization",
                steps = "check CPU limits,scale horizontally,optimize code",
                severity = "MEDIUM"
            )
        ''')

        self.db.execute('''
            STORE SEMANTIC (
                concept = "payments-api",
                knowledge = "critical service handles all payment processing"
            )
        ''')

        self.db.execute('''
            STORE SEMANTIC (
                concept = "auth-service",
                knowledge = "authentication service handles user login and tokens"
            )
        ''')

    def seed_rtb_knowledge(self):
        """Pre-seed agent with RTB/bidding domain knowledge."""
        self.db.execute('''
            STORE TOOLS (
                tool_id = "bid_calculator",
                description = "Calculate optimal bid price based on context",
                category = "bidding",
                ranking = "0.9"
            )
        ''')

        self.db.execute('''
            STORE TOOLS (
                tool_id = "audience_scorer",
                description = "Score audience segments for targeting",
                category = "targeting",
                ranking = "0.85"
            )
        ''')

        self.db.execute('''
            STORE SEMANTIC (
                concept = "sports-sites",
                knowledge = "sports news sites have high engagement with male 25-45 demographic"
            )
        ''')
