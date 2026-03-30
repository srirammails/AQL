# tests/test_integration.py
"""Integration tests - full AQL string to result."""

import pytest
from aql_db import ADB


class TestK8sScenario:
    """Kubernetes incident response scenario."""

    def setup_method(self):
        self.db = ADB()

        # Seed procedural memory with known pattern
        self.db.execute('''
            STORE PROCEDURAL (
                pattern_id = "oom-kill-001",
                pattern = "OOMKilled pod memory limit exceeded",
                severity = "HIGH",
                steps = "check memory limits,scale pod,notify team"
            )
        ''')

        # Seed episodic memory with past incidents
        self.db.execute('''
            STORE EPISODIC (
                incident_id = "inc-2024-03-21",
                pod = "payments-api",
                action = "scaled to 512Mi",
                resolved = "true"
            )
        ''')

    def test_recall_episodic_by_pod(self):
        result = self.db.execute('''
            RECALL EPISODIC WHERE pod = "payments-api"
            ORDER BY time DESC
            LIMIT 5
        ''')
        assert len(result["records"]) >= 1
        assert result["records"][0]["data"]["pod"] == "payments-api"

    def test_lookup_procedural_pattern(self):
        result = self.db.execute('''
            LOOKUP PROCEDURAL PATTERN $log_event
            THRESHOLD 0.3
        ''')
        # Should find our OOM pattern
        assert len(result["records"]) >= 1

    def test_store_and_recall_episodic(self):
        self.db.execute('''
            STORE EPISODIC (
                incident_id = "test-001",
                pod = "auth-service",
                resolved = "false"
            )
        ''')
        result = self.db.execute('''
            RECALL EPISODIC WHERE pod = "auth-service" LIMIT 1
        ''')
        assert len(result["records"]) == 1
        assert result["records"][0]["data"]["resolved"] == "false"

    def test_forget_working_memory(self):
        self.db.execute('STORE WORKING (task_id = "t1", status = "active")')
        self.db.execute('FORGET WORKING WHERE task_id = "t1"')
        result = self.db.execute('SCAN WORKING ALL')
        task_ids = [r["data"].get("task_id") for r in result["records"]]
        assert "t1" not in task_ids


class TestRTBScenario:
    """Real-time bidding scenario."""

    def setup_method(self):
        self.db = ADB()

        # Seed semantic memory with URL knowledge
        self.db.execute('''
            STORE SEMANTIC (
                concept = "sports.example.com",
                knowledge = "sports news site high engagement male 25-45"
            )
        ''')

        # Seed episodic with past bid history
        self.db.execute('''
            STORE EPISODIC (
                url = "sports.example.com",
                bid_price = "2.50",
                impression = "true",
                click = "false"
            )
        ''')

        # Seed tool registry
        self.db.execute('''
            STORE TOOLS (
                tool_id = "bid_calculator",
                description = "Calculate optimal bid price",
                category = "bidding",
                ranking = "0.9"
            )
        ''')

    def test_load_tools_by_category(self):
        result = self.db.execute('''
            LOAD TOOLS WHERE category = "bidding" LIMIT 3
        ''')
        assert len(result["tools"]) >= 1
        assert result["tools"][0]["data"]["category"] == "bidding"

    def test_lookup_semantic_key(self):
        result = self.db.execute('''
            LOOKUP SEMANTIC KEY concept = "sports.example.com"
        ''')
        assert len(result["records"]) >= 1

    def test_recall_semantic_like(self):
        # Note: In reference impl, $var uses var name as search text.
        # "sports" matches "sports news site" via word overlap.
        result = self.db.execute('''
            RECALL SEMANTIC LIKE $sports
            MIN_CONFIDENCE 0.1
            LIMIT 5
        ''')
        # Should find our sports.example.com concept
        assert len(result["records"]) >= 1


class TestMultiAgentScenario:
    """Multi-agent shared memory scenario."""

    def test_shared_semantic_memory(self):
        db = ADB()
        db.execute('''
            STORE SEMANTIC (
                concept = "k8s_oom_pattern",
                knowledge = "payments OOM errors every Friday"
            )
            SCOPE shared
            NAMESPACE "platform-agents"
        ''')
        # Note: In reference impl, $var uses var name as search text.
        # "Friday" matches "Friday" via word overlap.
        result = db.execute('''
            RECALL SEMANTIC LIKE $Friday
            MIN_CONFIDENCE 0.1
            LIMIT 5
        ''')
        assert len(result["records"]) > 0

    def test_private_working_memory(self):
        db = ADB()
        db.execute('''
            STORE WORKING (task_id = "agent-task-001", status = "processing")
            SCOPE private
        ''')
        result = db.execute('SCAN WORKING ALL')
        assert len(result["records"]) == 1
        assert result["records"][0]["metadata"]["scope"] == "private"


class TestPipeline:
    """Pipeline execution tests."""

    def setup_method(self):
        self.db = ADB()

    def test_simple_pipeline(self):
        # Seed data
        self.db.execute('STORE WORKING (task = "test")')

        result = self.db.execute('''
            PIPELINE test TIMEOUT 100ms
            SCAN WORKING ALL
            | RECALL EPISODIC WHERE pod = "test" LIMIT 5
        ''')
        # Pipeline should complete without error
        assert result is not None

    def test_pipeline_with_store_and_recall(self):
        self.db.execute('''
            STORE EPISODIC (
                event = "pipeline_test",
                value = "123"
            )
        ''')

        result = self.db.execute('''
            PIPELINE data_flow TIMEOUT 200ms
            SCAN WORKING ALL
            | RECALL EPISODIC WHERE event = "pipeline_test" LIMIT 10
        ''')
        assert len(result.get("records", [])) >= 1


class TestReflect:
    """REFLECT context assembly tests."""

    def setup_method(self):
        self.db = ADB()

        # Seed all memory types
        self.db.execute('STORE WORKING (current_task = "analyze")')
        self.db.execute('''
            STORE EPISODIC (event = "user_login", user = "alice")
        ''')
        self.db.execute('''
            STORE SEMANTIC (concept = "auth", knowledge = "handles authentication")
        ''')

    def test_reflect_includes_all_sources(self):
        result = self.db.execute('''
            REFLECT task_id = {current}
            INCLUDE WORKING
            INCLUDE EPISODIC
            INCLUDE SEMANTIC
        ''')

        assert "llm_context" in result
        assert "assembled_at" in result

        # Should have content from each source
        context = result["llm_context"]
        assert len(context) > 0

    def test_reflect_produces_readable_context(self):
        result = self.db.execute('''
            REFLECT agent_id = {test}
            INCLUDE WORKING
            INCLUDE EPISODIC
        ''')

        context = result["llm_context"]
        # Should be a formatted string
        assert isinstance(context, str)
