# aql-agent

**Demo agent using AQL/ADB for memory**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

## The Viral Demo

Ten lines to a working agent memory system:

```python
from aql_agent import SimpleAgent

agent = SimpleAgent()

# Seed knowledge
agent.db.execute('STORE SEMANTIC (concept="payments-api", knowledge="critical service")')

# Run on event - agent assembles memory context
result = agent.run("ALERT: payments-api OOMKilled")
print(result["context"])

# Run again - agent remembers first event
result2 = agent.run("ALERT: payments-api OOMKilled again")
# Second response is smarter because agent has memory
```

## Installation

```bash
pip install aql-agent

# With LLM support
pip install aql-agent[llm]
```

## Quick Start

```bash
# Run the demo
python demo.py

# Run with Claude LLM (requires ANTHROPIC_API_KEY)
python demo.py --llm

# Run specific scenario
python demo.py --scenario k8s
python demo.py --scenario rtb
```

## Demo Scenarios

### Kubernetes Incident Response

```bash
python demo.py --scenario k8s
```

Shows how agent memory improves incident response:
1. First OOM alert: Agent uses procedural + semantic memory
2. Second OOM alert: Agent remembers first incident, escalates

### Real-Time Bidding

```bash
python demo.py --scenario rtb
```

Shows agent using:
- Tool registry for bid calculation
- Semantic memory for audience knowledge
- Episodic memory for bid history

## API

### `SimpleAgent`

```python
class SimpleAgent:
    def __init__(self, api_key: Optional[str] = None):
        """
        Args:
            api_key: Anthropic API key for LLM reasoning.
                     If not provided, uses ANTHROPIC_API_KEY env var.
                     If neither, agent returns context without LLM decision.
        """

    def run(self, event: str, agent_id: str = "agent-001", use_llm: bool = True) -> dict:
        """
        Process an event using agent memory.

        Returns:
            {
                "event": str,
                "context": str,       # LLM-ready context from memory
                "decision": str,      # LLM decision (if use_llm=True)
                "memory_stats": {...}
            }
        """

    def seed_k8s_knowledge(self):
        """Pre-seed with Kubernetes domain knowledge."""

    def seed_rtb_knowledge(self):
        """Pre-seed with RTB/bidding domain knowledge."""
```

## How It Works

```
Event → Store in Working Memory
      → REFLECT (assemble context from all memory types)
      → Send to LLM with context
      → Store decision in Episodic Memory
      → Clear Working Memory
      → Return decision
```

The agent gets smarter over time because:
1. Episodic memory accumulates past decisions
2. REFLECT assembles relevant context for each new event
3. LLM sees history and makes better decisions

## License

Apache 2.0

---

*AQL Agent v0.1 · March 2026*
*"I built a complete agent memory system in Python.*
*Ten lines of code. Five memory types. One query language."*
