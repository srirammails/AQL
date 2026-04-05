//! AQL v0.5 Conformance Test Runner for ClawDB
//!
//! Runs the 150 YAML test cases against ClawDB

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clawdb::{ClawDB, Config};
use serde::Deserialize;
use serde_json::Value;

/// Test suite from YAML
#[derive(Debug, Deserialize)]
struct TestSuite {
    suite: String,
    #[serde(default)]
    spec_section: String,
    #[serde(default)]
    total_tests: usize,
    tests: Vec<TestCase>,
}

/// Individual test case
#[derive(Debug, Deserialize)]
struct TestCase {
    id: String,
    name: String,
    #[serde(default)]
    spec_ref: String,
    query: String,
    expect: Expectation,
}

/// Expected test result
#[derive(Debug, Deserialize)]
struct Expectation {
    success: bool,
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    count_gte: Option<usize>,
    #[serde(default)]
    count_lte: Option<usize>,
    #[serde(default)]
    error_contains: Option<String>,
    #[serde(default)]
    contains: Option<Vec<HashMap<String, Value>>>,
}

/// Test result
#[derive(Debug)]
struct TestResult {
    id: String,
    name: String,
    passed: bool,
    errors: Vec<String>,
}

/// Load seed data into ClawDB
async fn load_seed(db: &ClawDB) -> Result<String, Box<dyn std::error::Error>> {
    let seed_queries = vec![
        // WORKING memory
        r#"STORE INTO WORKING (bid_id = "w-001", campaign = "summer_2026", floor_price = 2.5, geo = "IE", priority = 8)"#,
        r#"STORE INTO WORKING (bid_id = "w-002", campaign = "summer_2026", floor_price = 3.0, geo = "IE", priority = 10)"#,
        // EPISODIC memory
        r#"STORE INTO EPISODIC (bid_id = "e-001", campaign = "summer_2026", won = true, cpm_paid = 3.20, ctr = 0.034, viewability = 0.91) SCOPE private NAMESPACE "pubcontext-bidder""#,
        r#"STORE INTO EPISODIC (bid_id = "e-002", campaign = "summer_2026", won = false, cpm_paid = 1.10, ctr = 0.005, viewability = 0.55) SCOPE private NAMESPACE "pubcontext-bidder""#,
        r#"STORE INTO EPISODIC (bid_id = "e-003", campaign = "winter_2026", won = true, cpm_paid = 2.85, ctr = 0.021) SCOPE shared NAMESPACE "agent-b-ns""#,
        // SEMANTIC memory
        r#"STORE INTO SEMANTIC (concept = "ai_enterprise_content", avg_cpm = 3.40, active = true, iab_category = "IAB19") SCOPE private"#,
        r#"STORE INTO SEMANTIC (concept = "finance_news", avg_cpm = 4.10, active = true, iab_category = "IAB13") SCOPE private"#,
        // PROCEDURAL memory
        r#"STORE INTO PROCEDURAL (pattern = "tech_news_premium", severity = "high", source = "agent", confidence = 0.72, steps = "1. score page 2. bid floor + 18%", success_count = 8, failure_count = 2, variables = "bid_multiplier=1.18,min_floor=2.00,max_cpm=5.00") SCOPE shared"#,
        // TOOLS memory
        r#"STORE INTO TOOLS (tool_id = "ctx_scorer_v2", name = "ctx_scorer_v2", category = "bidding", ranking = 0.92, schema = "context_score_v2", task = "scoring", ad_format = "display", description = "Context scoring tool for RTB") SCOPE shared"#,
    ];

    let mut proc_id = String::new();

    for query_str in seed_queries {
        let result = db.query(query_str).await?;
        if result.success {
            // Capture PROCEDURAL ID
            if query_str.contains("PROCEDURAL") {
                if let Some(data) = &result.data {
                    if let Ok(val) = serde_json::to_value(data) {
                        if let Some(id) = val.get("record").and_then(|r| r.get("id")).and_then(|i| i.as_str()) {
                            proc_id = id.to_string();
                        }
                    }
                }
            }
        }
    }

    Ok(proc_id)
}

/// Run a single test case
async fn run_test(db: &ClawDB, test: &TestCase, proc_id: &str) -> TestResult {
    let mut errors = Vec::new();

    // Substitute $PROC_ID
    let query_str = test.query.trim().replace("$PROC_ID", proc_id);

    // Execute query
    let result = match db.query(&query_str).await {
        Ok(r) => r,
        Err(e) => {
            return TestResult {
                id: test.id.clone(),
                name: test.name.clone(),
                passed: !test.expect.success, // If we expected failure, this might be ok
                errors: vec![format!("Execution error: {}", e)],
            };
        }
    };

    // Check success
    if result.success != test.expect.success {
        errors.push(format!(
            "Expected success={}, got {}",
            test.expect.success, result.success
        ));
        if !result.success {
            if let Some(err) = &result.error {
                errors.push(format!("Error: {}", err));
            }
        }
    }

    // Check error_contains
    if let Some(expected_error) = &test.expect.error_contains {
        if let Some(actual_error) = &result.error {
            let actual_lower = actual_error.to_lowercase();
            let expected_lower = expected_error.to_lowercase();
            if !actual_lower.contains(&expected_lower) {
                errors.push(format!(
                    "Error should contain '{}', got '{}'",
                    expected_error, actual_error
                ));
            }
        } else if !result.success {
            errors.push(format!("Expected error containing '{}', but no error message", expected_error));
        }
    }

    // Check count
    if result.success {
        if let Some(data) = &result.data {
            let data_val = serde_json::to_value(data).unwrap_or(Value::Null);
            let actual_count = data_val
                .get("count")
                .and_then(|c| c.as_u64())
                .map(|c| c as usize)
                .or_else(|| {
                    data_val
                        .get("records")
                        .and_then(|r| r.as_array())
                        .map(|a| a.len())
                });

            if let Some(expected) = test.expect.count {
                if let Some(actual) = actual_count {
                    if actual != expected {
                        errors.push(format!("Expected count={}, got {}", expected, actual));
                    }
                }
            }

            if let Some(min) = test.expect.count_gte {
                if let Some(actual) = actual_count {
                    if actual < min {
                        errors.push(format!("Expected count>={}, got {}", min, actual));
                    }
                }
            }

            if let Some(max) = test.expect.count_lte {
                if let Some(actual) = actual_count {
                    if actual > max {
                        errors.push(format!("Expected count<={}, got {}", max, actual));
                    }
                }
            }

            // Check contains
            if let Some(expected_records) = &test.expect.contains {
                if let Some(records) = data_val.get("records").and_then(|r| r.as_array()) {
                    for expected in expected_records {
                        let found = records.iter().any(|record| {
                            let data = record.get("data").unwrap_or(record);
                            expected.iter().all(|(key, value)| {
                                data.get(key).map_or(false, |v| v == value)
                                    || record.get(key).map_or(false, |v| v == value)
                            })
                        });
                        if !found {
                            errors.push(format!(
                                "Expected record containing {:?} not found",
                                expected
                            ));
                        }
                    }
                }
            }
        }
    }

    TestResult {
        id: test.id.clone(),
        name: test.name.clone(),
        passed: errors.is_empty(),
        errors,
    }
}

/// Run all test suites
async fn run_all_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║       AQL v0.5 Conformance Test Suite - ClawDB             ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Create temp directory for LanceDB
    let temp_dir = tempfile::tempdir()?;
    let config = Config::new(temp_dir.path());

    // Initialize ClawDB
    let db = ClawDB::open(config).await?;

    // Load seed data
    println!("Loading seed data...");
    let proc_id = load_seed(&db).await?;
    println!("Seed data loaded (PROC_ID: {})\n", proc_id);

    // Find test suite files
    let suites_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("suites");

    let mut suite_files: Vec<_> = fs::read_dir(&suites_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml"))
        .collect();
    suite_files.sort_by_key(|e| e.path());

    let mut total_tests = 0;
    let mut total_passed = 0;
    let mut failed_tests = Vec::new();

    // Run each suite
    for entry in suite_files {
        let path = entry.path();
        let content = fs::read_to_string(&path)?;
        let suite: TestSuite = serde_yaml::from_str(&content)?;

        println!("━━━ {} ({}) ━━━", suite.suite, path.file_name().unwrap().to_string_lossy());

        let mut suite_passed = 0;
        let mut suite_failed = 0;

        for test in &suite.tests {
            total_tests += 1;
            let result = run_test(&db, test, &proc_id).await;

            if result.passed {
                suite_passed += 1;
                total_passed += 1;
                println!("  ✓ {}: {}", result.id, result.name);
            } else {
                suite_failed += 1;
                println!("  ✗ {}: {}", result.id, result.name);
                for err in &result.errors {
                    println!("    ↳ {}", err);
                }
                failed_tests.push(result);
            }
        }

        let status = if suite_failed == 0 { "✓" } else { "!" };
        println!("{} Suite: {}/{} passed\n", status, suite_passed, suite_passed + suite_failed);
    }

    // Summary
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                        SUMMARY                             ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("  Total:  {}", total_tests);
    println!("  Passed: {}", total_passed);
    println!("  Failed: {}", total_tests - total_passed);
    println!("  Rate:   {:.1}%\n", (total_passed as f64 / total_tests as f64) * 100.0);

    if !failed_tests.is_empty() {
        println!("Failed Tests:");
        for ft in &failed_tests {
            println!("  {}: {}", ft.id, ft.name);
        }
    }

    Ok(())
}

#[tokio::test]
async fn conformance_test_suite() {
    run_all_tests().await.expect("Test suite failed");
}
