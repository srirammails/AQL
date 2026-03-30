# aql_agent/scenarios/rtb.py
"""Real-time bidding demo scenario."""

from ..agent import SimpleAgent


def run_rtb_demo(use_llm: bool = False):
    """
    Run the real-time bidding demo.

    Shows how agent uses tools, semantic, and episodic memory
    to make bid decisions.

    Args:
        use_llm: Whether to use LLM for decisions (requires API key)
    """
    print("=" * 60)
    print("AQL Agent Demo: Real-Time Bidding")
    print("=" * 60)
    print()

    # Create agent with memory
    agent = SimpleAgent()
    agent.seed_rtb_knowledge()

    print("Agent initialized with RTB domain knowledge:")
    print("  - Tools: bid_calculator, audience_scorer")
    print("  - Semantic: sports-sites audience knowledge")
    print()

    # Seed some bid history
    agent.db.execute('''
        STORE EPISODIC (
            url = "sports.example.com",
            bid_price = "2.50",
            won = "true",
            click = "false"
        )
    ''')

    agent.db.execute('''
        STORE EPISODIC (
            url = "sports.example.com",
            bid_price = "2.75",
            won = "true",
            click = "true"
        )
    ''')

    print("Seeded bid history for sports.example.com")
    print()

    # Process bid request
    print("-" * 40)
    print("Bid Request: sports.example.com")
    print("-" * 40)

    result = agent.run(
        "BID_REQUEST: sports.example.com, user_segment=sports_enthusiast, floor_price=1.50",
        agent_id="rtb-agent",
        use_llm=use_llm,
    )

    print(f"Context assembled from memory:")
    print(f"  - Episodic records: {result['memory_stats']['episodic_count']}")
    print(f"  - Semantic records: {result['memory_stats']['semantic_count']}")
    print()

    if result.get("decision"):
        print(f"LLM Decision: {result['decision'][:200]}...")
    else:
        print("Context for LLM:")
        print(result["context"][:500] if result["context"] else "  (empty)")
    print()

    # Load tools
    print("-" * 40)
    print("Available Tools (LOAD TOOLS)")
    print("-" * 40)

    tools = agent.db.execute('''
        LOAD TOOLS WHERE category = "bidding"
        ORDER BY ranking DESC
        LIMIT 3
    ''')

    for tool in tools.get("tools", []):
        data = tool.get("data", {})
        print(f"  - {data.get('tool_id')}: {data.get('description')}")
        print(f"    Ranking: {data.get('ranking')}")
    print()

    print("=" * 60)
    print("Demo complete. Agent used memory to inform bid decision.")
    print("=" * 60)


if __name__ == "__main__":
    run_rtb_demo()
