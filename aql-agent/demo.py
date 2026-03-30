#!/usr/bin/env python3
# demo.py
"""
AQL Agent Demo - The viral demo script.

Ten lines to a working agent memory system.
Clone it. Try it. Tell me what breaks.

Usage:
    python demo.py              # Run without LLM
    python demo.py --llm        # Run with Claude LLM (needs API key)
    python demo.py --scenario k8s
    python demo.py --scenario rtb
"""

import argparse
from aql_agent import SimpleAgent
from aql_agent.scenarios import run_k8s_demo, run_rtb_demo


def main():
    parser = argparse.ArgumentParser(description="AQL Agent Demo")
    parser.add_argument(
        "--llm",
        action="store_true",
        help="Use Claude LLM for decisions (requires ANTHROPIC_API_KEY)"
    )
    parser.add_argument(
        "--scenario",
        choices=["k8s", "rtb", "simple"],
        default="simple",
        help="Demo scenario to run"
    )
    args = parser.parse_args()

    if args.scenario == "k8s":
        run_k8s_demo(use_llm=args.llm)
    elif args.scenario == "rtb":
        run_rtb_demo(use_llm=args.llm)
    else:
        run_simple_demo(use_llm=args.llm)


def run_simple_demo(use_llm: bool = False):
    """
    The simplest possible demo.
    Shows agent memory in ~20 lines.
    """
    print("=" * 60)
    print("AQL Agent Demo: Simple Memory Demo")
    print("=" * 60)
    print()

    # Create agent with memory
    agent = SimpleAgent()

    # Seed the agent with domain knowledge
    agent.db.execute('''
        STORE PROCEDURAL (
            pattern_id = "oom-kill-001",
            pattern = "OOMKilled memory limit exceeded",
            steps = "scale memory,check limits,notify team"
        )
    ''')

    agent.db.execute('''
        STORE SEMANTIC (
            concept = "payments-api",
            knowledge = "critical service handles all payment processing"
        )
    ''')

    print("Agent seeded with knowledge.")
    print()

    # Run the agent on a K8s event
    print("Running agent on K8s event...")
    result1 = agent.run(
        "ALERT: payments-api-7d9f OOMKilled in production namespace",
        use_llm=use_llm,
    )
    print(f"Memory assembled: {result1['memory_stats']}")
    if result1.get("decision"):
        print(f"Decision: {result1['decision'][:100]}...")
    print()

    # Run again - agent now has memory of the first event
    print("Running agent on second event (agent has memory now)...")
    result2 = agent.run(
        "ALERT: payments-api-7d9f OOMKilled again",
        use_llm=use_llm,
    )
    print(f"Memory assembled: {result2['memory_stats']}")
    if result2.get("decision"):
        print(f"Decision: {result2['decision'][:100]}...")
    print()

    # Show agent memory
    print("Agent episodic memory:")
    history = agent.db.execute(
        'RECALL EPISODIC WHERE agent_id = "agent-001" LIMIT 5'
    )
    for record in history["records"]:
        print(f"  - {record['data']}")
    print()

    print("=" * 60)
    print("The second response is smarter because the agent has memory.")
    print("That's the point of ADB.")
    print("=" * 60)


if __name__ == "__main__":
    main()
