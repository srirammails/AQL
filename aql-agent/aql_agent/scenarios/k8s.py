# aql_agent/scenarios/k8s.py
"""Kubernetes incident response demo scenario."""

from ..agent import SimpleAgent


def run_k8s_demo(use_llm: bool = False):
    """
    Run the Kubernetes incident response demo.

    Shows how agent memory improves decisions over time.

    Args:
        use_llm: Whether to use LLM for decisions (requires API key)
    """
    print("=" * 60)
    print("AQL Agent Demo: Kubernetes Incident Response")
    print("=" * 60)
    print()

    # Create agent with memory
    agent = SimpleAgent()
    agent.seed_k8s_knowledge()

    print("Agent initialized with K8s domain knowledge:")
    print("  - Procedural: OOM and CPU throttle patterns")
    print("  - Semantic: payments-api and auth-service knowledge")
    print()

    # First incident
    print("-" * 40)
    print("Event 1: OOM Kill Alert")
    print("-" * 40)

    result1 = agent.run(
        "ALERT: payments-api-7d9f OOMKilled in production namespace",
        agent_id="k8s-agent",
        use_llm=use_llm,
    )

    print(f"Context assembled from memory:")
    print(f"  - Episodic records: {result1['memory_stats']['episodic_count']}")
    print(f"  - Semantic records: {result1['memory_stats']['semantic_count']}")
    print(f"  - Procedural records: {result1['memory_stats']['procedural_count']}")
    print()

    if result1.get("decision"):
        print(f"LLM Decision: {result1['decision'][:200]}...")
    else:
        print("Context for LLM:")
        print(result1["context"][:500] if result1["context"] else "  (empty)")
    print()

    # Second incident - agent now has memory of first
    print("-" * 40)
    print("Event 2: Repeated OOM Kill (Agent has memory now)")
    print("-" * 40)

    result2 = agent.run(
        "ALERT: payments-api-7d9f OOMKilled again",
        agent_id="k8s-agent",
        use_llm=use_llm,
    )

    print(f"Context assembled from memory:")
    print(f"  - Episodic records: {result2['memory_stats']['episodic_count']}")
    print(f"  - Semantic records: {result2['memory_stats']['semantic_count']}")
    print(f"  - Procedural records: {result2['memory_stats']['procedural_count']}")
    print()

    if result2.get("decision"):
        print(f"LLM Decision: {result2['decision'][:200]}...")
    else:
        print("Context for LLM (now includes first incident):")
        print(result2["context"][:500] if result2["context"] else "  (empty)")
    print()

    # Show memory growth
    print("-" * 40)
    print("Agent Memory State")
    print("-" * 40)

    history = agent.db.execute(
        'RECALL EPISODIC WHERE agent_id = "k8s-agent" LIMIT 10'
    )
    print(f"Episodic memories: {len(history['records'])}")
    for record in history["records"]:
        event = record["data"].get("event", "")[:50]
        print(f"  - {event}...")

    print()
    print("=" * 60)
    print("Demo complete. The second response should be smarter")
    print("because the agent remembers the first incident.")
    print("=" * 60)


if __name__ == "__main__":
    run_k8s_demo()
