//! AQL Statement Executor
//!
//! Executes parsed AQL statements against the in-memory store.
//! Supports: STORE, RECALL, LOOKUP, LOAD, SCAN, FORGET, UPDATE,
//! LINK, AGGREGATE, HAVING, REFLECT, PIPELINE, WITH LINKS, FOLLOW LINKS

use aql_parser::*;
use serde_json::{json, Value};
use chrono::Utc;

use crate::memory::{MemoryStore, Record, get_nested_field};
use crate::QueryResult;

/// Execute a parsed AQL statement
pub fn execute_statement(stmt: &Statement, store: &mut MemoryStore) -> QueryResult {
    match stmt {
        Statement::Store(s) => execute_store(s, store),
        Statement::Recall(r) => execute_recall(r, store),
        Statement::Lookup(l) => execute_lookup(l, store),
        Statement::Load(l) => execute_load(l, store),
        Statement::Scan(s) => execute_scan(s, store),
        Statement::Forget(f) => execute_forget(f, store),
        Statement::Update(u) => execute_update(u, store),
        Statement::Link(l) => execute_link(l, store),
        Statement::Reflect(r) => execute_reflect(r, store),
        Statement::Pipeline(p) => execute_pipeline(p, store),
    }
}

/// Execute STORE statement
fn execute_store(stmt: &StoreStmt, store: &mut MemoryStore) -> QueryResult {
    // Build data object from payload
    let mut data = serde_json::Map::new();
    for assignment in &stmt.payload {
        let value = value_to_json(&assignment.value);
        data.insert(assignment.field.clone(), value);
    }

    let mut record = Record::new(
        &stmt.memory_type.to_string(),
        Value::Object(data),
    );

    // Apply modifiers
    if let Some(ns) = &stmt.modifiers.namespace {
        record = record.with_namespace(ns.clone());
    }
    if let Some(scope) = &stmt.modifiers.scope {
        record = record.with_scope(format!("{:?}", scope).to_lowercase());
    }
    if let Some(ttl) = &stmt.modifiers.ttl {
        record = record.with_ttl(ttl.as_millis() as i64);
    }

    match store.store(&stmt.memory_type, record) {
        Ok(r) => QueryResult::ok(json!({
            "action": "stored",
            "record": r
        })),
        Err(e) => QueryResult::err(&e),
    }
}

/// Execute RECALL statement
fn execute_recall(stmt: &RecallStmt, store: &mut MemoryStore) -> QueryResult {
    // Handle SEMANTIC with LIKE predicate
    if matches!(stmt.memory_type, MemoryType::Semantic) {
        return execute_semantic_recall(stmt, store);
    }

    let records = match query_records(store, &stmt.memory_type, &stmt.predicate, &stmt.modifiers) {
        Ok(r) => r,
        Err(e) => return QueryResult::err(&e),
    };

    // Apply AGGREGATE if present
    if let Some(aggregates) = &stmt.modifiers.aggregate {
        return execute_aggregate(&records, aggregates, &stmt.modifiers.having);
    }

    // Apply FOLLOW LINKS if present
    if let Some(follow) = &stmt.modifiers.follow_links {
        let source_ids: Vec<String> = records.iter()
            .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
            .collect();
        let depth = follow.depth.unwrap_or(1);
        let linked = store.follow_links(&source_ids, &follow.link_type, depth);
        let linked_values: Vec<Value> = linked.iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        return QueryResult::ok(json!({
            "action": "recall",
            "memory_type": stmt.memory_type.to_string(),
            "source_count": records.len(),
            "followed_links": follow.link_type,
            "count": linked_values.len(),
            "records": linked_values
        }));
    }

    QueryResult::ok(json!({
        "action": "recall",
        "memory_type": stmt.memory_type.to_string(),
        "count": records.len(),
        "records": records
    }))
}

/// Execute RECALL FROM SEMANTIC LIKE ...
fn execute_semantic_recall(stmt: &RecallStmt, store: &MemoryStore) -> QueryResult {
    let query = match &stmt.predicate {
        Predicate::Like { variable } => variable.clone(),
        Predicate::Where { conditions } => {
            if let Some(cond) = conditions.first() {
                if let Condition::Simple { value, .. } = cond {
                    if let aql_parser::Value::String(s) = value {
                        s.clone()
                    } else {
                        return QueryResult::err("Semantic search requires a string query");
                    }
                } else {
                    return QueryResult::err("Semantic search requires LIKE predicate");
                }
            } else {
                return QueryResult::err("Semantic search requires LIKE predicate");
            }
        }
        _ => return QueryResult::err("Semantic search requires LIKE predicate (e.g., RECALL FROM SEMANTIC LIKE \"query\")"),
    };

    let limit = stmt.modifiers.limit.unwrap_or(5);
    let results = store.semantic.search(&query, limit);

    QueryResult::ok(json!({
        "action": "recall",
        "memory_type": "SEMANTIC",
        "query": query,
        "count": results.len(),
        "records": results,
        "note": "Semantic memory is pre-seeded with tech concepts. Results ranked by similarity."
    }))
}

/// Execute AGGREGATE on records
fn execute_aggregate(records: &[Value], aggregates: &[AggregateFunc], having: &Option<Vec<Condition>>) -> QueryResult {
    let mut results = serde_json::Map::new();

    for agg in aggregates {
        let alias = agg.alias.clone().unwrap_or_else(|| {
            match &agg.field {
                Some(f) => format!("{}_{}", format!("{:?}", agg.func).to_lowercase(), f),
                None => format!("{:?}", agg.func).to_lowercase(),
            }
        });

        let value = match agg.func {
            AggregateFuncType::Count => {
                Value::Number(records.len().into())
            }
            AggregateFuncType::Sum => {
                if let Some(field) = &agg.field {
                    let sum: f64 = records.iter()
                        .filter_map(|r| get_nested_field(r, field))
                        .filter_map(|v| v.as_f64())
                        .sum();
                    serde_json::Number::from_f64(sum)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            AggregateFuncType::Avg => {
                if let Some(field) = &agg.field {
                    let values: Vec<f64> = records.iter()
                        .filter_map(|r| get_nested_field(r, field))
                        .filter_map(|v| v.as_f64())
                        .collect();
                    if values.is_empty() {
                        Value::Null
                    } else {
                        let avg = values.iter().sum::<f64>() / values.len() as f64;
                        serde_json::Number::from_f64(avg)
                            .map(Value::Number)
                            .unwrap_or(Value::Null)
                    }
                } else {
                    Value::Null
                }
            }
            AggregateFuncType::Min => {
                if let Some(field) = &agg.field {
                    records.iter()
                        .filter_map(|r| get_nested_field(r, field))
                        .filter_map(|v| v.as_f64())
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .and_then(|v| serde_json::Number::from_f64(v).map(Value::Number))
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            AggregateFuncType::Max => {
                if let Some(field) = &agg.field {
                    records.iter()
                        .filter_map(|r| get_nested_field(r, field))
                        .filter_map(|v| v.as_f64())
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .and_then(|v| serde_json::Number::from_f64(v).map(Value::Number))
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
        };

        results.insert(alias, value);
    }

    // Apply HAVING filter
    if let Some(having_conditions) = having {
        let result_value = Value::Object(results.clone());
        if !matches_having(&result_value, having_conditions) {
            return QueryResult::ok(json!({
                "action": "aggregate",
                "count": records.len(),
                "results": {},
                "filtered": true,
                "note": "HAVING condition not met"
            }));
        }
    }

    QueryResult::ok(json!({
        "action": "aggregate",
        "count": records.len(),
        "results": results
    }))
}

/// Check if aggregate results match HAVING conditions
fn matches_having(results: &Value, conditions: &[Condition]) -> bool {
    for condition in conditions {
        match condition {
            Condition::Simple { field, operator, value, .. } => {
                if let Some(result_value) = results.get(field) {
                    if !compare_values(result_value, operator, &value_to_json(value)) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Condition::Group { conditions, .. } => {
                if !matches_having(results, conditions) {
                    return false;
                }
            }
        }
    }
    true
}

/// Execute LOOKUP statement
fn execute_lookup(stmt: &LookupStmt, store: &mut MemoryStore) -> QueryResult {
    let records = match query_records(store, &stmt.memory_type, &stmt.predicate, &stmt.modifiers) {
        Ok(r) => r,
        Err(e) => return QueryResult::err(&e),
    };

    QueryResult::ok(json!({
        "action": "lookup",
        "memory_type": stmt.memory_type.to_string(),
        "count": records.len(),
        "records": records
    }))
}

/// Execute LOAD statement (for TOOLS)
fn execute_load(stmt: &LoadStmt, store: &mut MemoryStore) -> QueryResult {
    let records = match query_records(store, &MemoryType::Tools, &stmt.predicate, &stmt.modifiers) {
        Ok(r) => r,
        Err(e) => return QueryResult::err(&e),
    };

    QueryResult::ok(json!({
        "action": "load",
        "memory_type": "TOOLS",
        "count": records.len(),
        "records": records
    }))
}

/// Execute SCAN statement (for WORKING)
fn execute_scan(stmt: &ScanStmt, store: &mut MemoryStore) -> QueryResult {
    let working = &store.working;

    let mut records: Vec<&Record> = working.values()
        .filter(|r| !r.is_expired())
        .collect();

    // Sort by created_at descending (most recent first)
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Apply WINDOW modifier
    if let Some(window) = &stmt.window {
        records = apply_window(records, window);
    } else if let Some(window) = &stmt.modifiers.window {
        records = apply_window(records, window);
    }

    // Apply LIMIT
    if let Some(limit) = stmt.modifiers.limit {
        records.truncate(limit);
    }

    // Convert to JSON
    let results: Vec<Value> = if let Some(fields) = &stmt.modifiers.return_fields {
        records.iter().map(|r| project_fields(r, fields)).collect()
    } else {
        records.iter().map(|r| serde_json::to_value(r).unwrap()).collect()
    };

    QueryResult::ok(json!({
        "action": "scan",
        "memory_type": "WORKING",
        "count": results.len(),
        "records": results
    }))
}

/// Execute FORGET statement
fn execute_forget(stmt: &ForgetStmt, store: &mut MemoryStore) -> QueryResult {
    if matches!(stmt.memory_type, MemoryType::Semantic) {
        return QueryResult::err("Cannot FORGET from Semantic memory in playground (read-only)");
    }

    let ids: Vec<String> = if let Some(mem) = store.get_store(&stmt.memory_type) {
        mem.values()
            .filter(|r| matches_conditions(r, &stmt.conditions))
            .map(|r| r.id.clone())
            .collect()
    } else {
        return QueryResult::err(&format!("Unknown memory type: {:?}", stmt.memory_type));
    };

    let deleted = store.forget(&stmt.memory_type, ids);

    QueryResult::ok(json!({
        "action": "forget",
        "memory_type": stmt.memory_type.to_string(),
        "deleted": deleted
    }))
}

/// Execute UPDATE statement
fn execute_update(stmt: &UpdateStmt, store: &mut MemoryStore) -> QueryResult {
    if matches!(stmt.memory_type, MemoryType::Semantic) {
        return QueryResult::err("Cannot UPDATE Semantic memory in playground (read-only)");
    }

    let mem = match store.get_store_mut(&stmt.memory_type) {
        Some(m) => m,
        None => return QueryResult::err(&format!("Unknown memory type: {:?}", stmt.memory_type)),
    };

    let mut updated = 0;
    for record in mem.values_mut() {
        if matches_conditions(record, &stmt.conditions) {
            if let Value::Object(ref mut data) = record.data {
                for assignment in &stmt.payload {
                    data.insert(assignment.field.clone(), value_to_json(&assignment.value));
                }
            }
            updated += 1;
        }
    }

    QueryResult::ok(json!({
        "action": "update",
        "memory_type": stmt.memory_type.to_string(),
        "updated": updated
    }))
}

/// Execute LINK statement
fn execute_link(stmt: &LinkStmt, store: &mut MemoryStore) -> QueryResult {
    // Validate: cannot link FROM ALL
    if matches!(stmt.from_type, MemoryType::All) {
        return QueryResult::err("LINK FROM ALL is not allowed");
    }

    // Find source records matching conditions
    let source_ids: Vec<String> = if let Some(mem) = store.get_store(&stmt.from_type) {
        mem.values()
            .filter(|r| matches_conditions(r, &stmt.from_conditions))
            .map(|r| r.id.clone())
            .collect()
    } else {
        return QueryResult::err(&format!("Unknown source memory type: {:?}", stmt.from_type));
    };

    if source_ids.is_empty() {
        return QueryResult::err("No source records found matching conditions");
    }

    // Find target records matching conditions
    let target_ids: Vec<String> = if let Some(mem) = store.get_store(&stmt.to_type) {
        mem.values()
            .filter(|r| matches_conditions(r, &stmt.to_conditions))
            .map(|r| r.id.clone())
            .collect()
    } else {
        return QueryResult::err(&format!("Unknown target memory type: {:?}", stmt.to_type));
    };

    if target_ids.is_empty() {
        return QueryResult::err("No target records found matching conditions");
    }

    // Create links
    let mut links_created = 0;
    let mut errors = Vec::new();

    for source_id in &source_ids {
        for target_id in &target_ids {
            match store.create_link(source_id, target_id, &stmt.link_type, stmt.weight) {
                Ok(_) => links_created += 1,
                Err(e) => errors.push(e),
            }
        }
    }

    QueryResult::ok(json!({
        "action": "link",
        "link_type": stmt.link_type,
        "source_count": source_ids.len(),
        "target_count": target_ids.len(),
        "links_created": links_created,
        "errors": errors
    }))
}

/// Execute REFLECT statement
fn execute_reflect(stmt: &ReflectStmt, store: &mut MemoryStore) -> QueryResult {
    let mut all_records: Vec<Value> = Vec::new();

    // Gather records from all sources
    for source in &stmt.sources {
        let records = if matches!(source.memory_type, MemoryType::All) {
            store.get_all_records().iter()
                .map(|r| serde_json::to_value(r).unwrap())
                .collect()
        } else if let Some(mem) = store.get_store(&source.memory_type) {
            let filtered: Vec<Value> = mem.values()
                .filter(|r| !r.is_expired())
                .filter(|r| {
                    if let Some(pred) = &source.predicate {
                        matches_predicate(r, pred)
                    } else {
                        true
                    }
                })
                .map(|r| serde_json::to_value(r).unwrap())
                .collect();
            filtered
        } else {
            continue;
        };
        all_records.extend(records);
    }

    // Apply WITH LINKS - include links in response
    let links_info = if let Some(with_links) = &stmt.with_links {
        let mut links = Vec::new();
        for record in &all_records {
            if let Some(id) = record.get("id").and_then(|v| v.as_str()) {
                let record_links = match with_links {
                    WithLinks::All => store.get_links_from(id, None),
                    WithLinks::Type { link_type } => store.get_links_from(id, Some(link_type)),
                };
                for link in record_links {
                    links.push(serde_json::to_value(link).unwrap());
                }
            }
        }
        Some(links)
    } else {
        None
    };

    // Apply FOLLOW LINKS - traverse to linked records
    if let Some(follow) = &stmt.follow_links {
        let source_ids: Vec<String> = all_records.iter()
            .filter_map(|r| r.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();
        let depth = follow.depth.unwrap_or(1);
        let linked = store.follow_links(&source_ids, &follow.link_type, depth);
        let linked_values: Vec<Value> = linked.iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        all_records.extend(linked_values);
    }

    // Execute THEN clause if present
    if let Some(then_stmt) = &stmt.then_clause {
        let then_result = execute_statement(then_stmt, store);
        return QueryResult::ok(json!({
            "action": "reflect",
            "source_count": all_records.len(),
            "records": all_records,
            "links": links_info,
            "then_result": serde_json::from_str::<Value>(&serde_json::to_string(&then_result).unwrap()).ok()
        }));
    }

    QueryResult::ok(json!({
        "action": "reflect",
        "count": all_records.len(),
        "records": all_records,
        "links": links_info
    }))
}

/// Execute PIPELINE statement
fn execute_pipeline(stmt: &PipelineStmt, store: &mut MemoryStore) -> QueryResult {
    // Validate: TIMEOUT is required
    if stmt.timeout.is_none() {
        return QueryResult::err("PIPELINE requires TIMEOUT (e.g., PIPELINE name TIMEOUT 50ms ...)");
    }

    let timeout = stmt.timeout.unwrap();
    let timeout_ms = timeout.as_millis() as i64;
    let start_ms = Utc::now().timestamp_millis();
    let mut stage_results = Vec::new();
    let mut last_result: Option<QueryResult> = None;

    for (i, stage) in stmt.stages.iter().enumerate() {
        // Check timeout using chrono (WASM-compatible)
        let elapsed_ms = Utc::now().timestamp_millis() - start_ms;
        if elapsed_ms > timeout_ms {
            return QueryResult::ok(json!({
                "action": "pipeline",
                "name": stmt.name,
                "status": "timeout",
                "completed_stages": i,
                "total_stages": stmt.stages.len(),
                "elapsed_ms": elapsed_ms,
                "timeout_ms": timeout_ms,
                "results": stage_results
            }));
        }

        // Execute stage
        let result = execute_statement(stage, store);
        let result_json = serde_json::to_value(&result).unwrap();
        stage_results.push(json!({
            "stage": i + 1,
            "result": result_json
        }));

        if !result.success {
            let elapsed_ms = Utc::now().timestamp_millis() - start_ms;
            return QueryResult::ok(json!({
                "action": "pipeline",
                "name": stmt.name,
                "status": "error",
                "failed_stage": i + 1,
                "total_stages": stmt.stages.len(),
                "elapsed_ms": elapsed_ms,
                "results": stage_results
            }));
        }

        last_result = Some(result);
    }

    let elapsed_ms = Utc::now().timestamp_millis() - start_ms;
    QueryResult::ok(json!({
        "action": "pipeline",
        "name": stmt.name,
        "status": "completed",
        "stages": stmt.stages.len(),
        "elapsed_ms": elapsed_ms,
        "timeout_ms": timeout_ms,
        "results": stage_results,
        "final_result": last_result.map(|r| serde_json::to_value(&r).unwrap())
    }))
}

/// Query records from a memory type with predicate and modifiers
fn query_records(
    store: &MemoryStore,
    memory_type: &MemoryType,
    predicate: &Predicate,
    modifiers: &Modifiers,
) -> Result<Vec<Value>, String> {
    let mem = store.get_store(memory_type)
        .ok_or_else(|| format!("Cannot query {:?}", memory_type))?;

    // Filter by predicate
    let mut records: Vec<&Record> = mem.values()
        .filter(|r| !r.is_expired())
        .filter(|r| matches_predicate(r, predicate))
        .collect();

    // Apply ORDER BY
    if let Some(order) = &modifiers.order_by {
        sort_records(&mut records, &order.field, order.ascending);
    }

    // Apply LIMIT (before AGGREGATE if both present)
    if let Some(limit) = modifiers.limit {
        // Only apply LIMIT before aggregation if no AGGREGATE
        if modifiers.aggregate.is_none() {
            records.truncate(limit);
        }
    }

    // Apply WITH LINKS - include links in each record
    let results: Vec<Value> = if let Some(with_links) = &modifiers.with_links {
        records.iter().map(|r| {
            let mut record_json = serde_json::to_value(r).unwrap();
            let links: Vec<Value> = match with_links {
                WithLinks::All => r.get_all_links().iter()
                    .map(|l| serde_json::to_value(l).unwrap())
                    .collect(),
                WithLinks::Type { link_type } => r.get_links_by_type(link_type).iter()
                    .map(|l| serde_json::to_value(l).unwrap())
                    .collect(),
            };
            if let Value::Object(ref mut obj) = record_json {
                obj.insert("links".to_string(), Value::Array(links));
            }
            record_json
        }).collect()
    } else if let Some(fields) = &modifiers.return_fields {
        records.iter().map(|r| project_fields(r, fields)).collect()
    } else {
        records.iter().map(|r| serde_json::to_value(r).unwrap()).collect()
    };

    // Apply LIMIT after conversion if AGGREGATE present
    if let Some(limit) = modifiers.limit {
        if modifiers.aggregate.is_some() {
            return Ok(results.into_iter().take(limit).collect());
        }
    }

    Ok(results)
}

/// Check if a record matches a predicate
fn matches_predicate(record: &Record, predicate: &Predicate) -> bool {
    match predicate {
        Predicate::All => true,
        Predicate::Where { conditions } => matches_conditions(record, conditions),
        Predicate::Key { field, value } => {
            if let Some(v) = get_record_field(record, field) {
                json_equals(&v, &value_to_json(value))
            } else {
                false
            }
        }
        Predicate::Like { .. } => true,
        Predicate::Pattern { variable, .. } => {
            let pattern = variable.to_lowercase();
            if let Value::Object(data) = &record.data {
                data.values().any(|v| {
                    if let Value::String(s) = v {
                        s.to_lowercase().contains(&pattern)
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        }
    }
}

/// Check if a record matches a list of conditions
fn matches_conditions(record: &Record, conditions: &[Condition]) -> bool {
    if conditions.is_empty() {
        return true;
    }

    let mut result = evaluate_condition(record, &conditions[0]);

    for condition in &conditions[1..] {
        let cond_result = evaluate_condition(record, condition);
        match condition.logical_op() {
            Some(LogicalOp::Or) => result = result || cond_result,
            Some(LogicalOp::And) | None => result = result && cond_result,
        }
    }

    result
}

/// Evaluate a single condition
fn evaluate_condition(record: &Record, condition: &Condition) -> bool {
    match condition {
        Condition::Simple { field, operator, value, .. } => {
            if let Some(field_value) = get_record_field(record, field) {
                compare_values(&field_value, operator, &value_to_json(value))
            } else {
                false
            }
        }
        Condition::Group { conditions, .. } => {
            matches_conditions(record, conditions)
        }
    }
}

/// Get a field value from a record
fn get_record_field(record: &Record, path: &str) -> Option<Value> {
    if path == "id" {
        return Some(Value::String(record.id.clone()));
    }
    if path == "created_at" || path == "metadata.created_at" {
        return Some(Value::Number(record.created_at.into()));
    }
    if path == "namespace" || path == "metadata.namespace" {
        return record.namespace.clone().map(Value::String);
    }
    if path == "scope" || path == "metadata.scope" {
        return record.scope.clone().map(Value::String);
    }

    let field_path = if path.starts_with("data.") {
        &path[5..]
    } else {
        path
    };

    get_nested_field(&record.data, field_path).cloned()
}

/// Compare two JSON values with an operator
fn compare_values(left: &Value, operator: &Operator, right: &Value) -> bool {
    match operator {
        Operator::Eq => json_equals(left, right),
        Operator::Ne => !json_equals(left, right),
        Operator::Gt => json_compare(left, right).map_or(false, |c| c > 0),
        Operator::Gte => json_compare(left, right).map_or(false, |c| c >= 0),
        Operator::Lt => json_compare(left, right).map_or(false, |c| c < 0),
        Operator::Lte => json_compare(left, right).map_or(false, |c| c <= 0),
        Operator::Contains => {
            match (left, right) {
                (Value::String(s), Value::String(t)) => s.contains(t.as_str()),
                (Value::Array(arr), _) => arr.contains(right),
                _ => false,
            }
        }
        Operator::StartsWith => {
            match (left, right) {
                (Value::String(s), Value::String(t)) => s.starts_with(t.as_str()),
                _ => false,
            }
        }
        Operator::EndsWith => {
            match (left, right) {
                (Value::String(s), Value::String(t)) => s.ends_with(t.as_str()),
                _ => false,
            }
        }
        Operator::In => {
            match right {
                Value::Array(arr) => arr.iter().any(|v| json_equals(left, v)),
                _ => false,
            }
        }
    }
}

/// Check JSON equality
fn json_equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => a == b,
        _ => false,
    }
}

/// Compare JSON values (returns ordering)
fn json_compare(a: &Value, b: &Value) -> Option<i32> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let af = a.as_f64()?;
            let bf = b.as_f64()?;
            af.partial_cmp(&bf).map(|c| c as i32)
        }
        (Value::String(a), Value::String(b)) => Some(a.cmp(b) as i32),
        _ => None,
    }
}

/// Convert AQL Value to JSON Value
fn value_to_json(value: &aql_parser::Value) -> Value {
    match value {
        aql_parser::Value::Null => Value::Null,
        aql_parser::Value::Bool(b) => Value::Bool(*b),
        aql_parser::Value::Int(i) => Value::Number((*i).into()),
        aql_parser::Value::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        aql_parser::Value::String(s) => Value::String(s.clone()),
        aql_parser::Value::Variable(v) => Value::String(format!("${}", v)),
        aql_parser::Value::Array(arr) => {
            Value::Array(arr.iter().map(value_to_json).collect())
        }
    }
}

/// Sort records by field
fn sort_records(records: &mut Vec<&Record>, field: &str, ascending: bool) {
    records.sort_by(|a, b| {
        let va = get_record_field(a, field);
        let vb = get_record_field(b, field);

        let cmp = match (va, vb) {
            (Some(Value::Number(na)), Some(Value::Number(nb))) => {
                na.as_f64().partial_cmp(&nb.as_f64()).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Some(Value::String(sa)), Some(Value::String(sb))) => sa.cmp(&sb),
            _ => std::cmp::Ordering::Equal,
        };

        if ascending { cmp } else { cmp.reverse() }
    });
}

/// Apply WINDOW modifier
fn apply_window<'a>(mut records: Vec<&'a Record>, window: &Window) -> Vec<&'a Record> {
    match window {
        Window::LastN { count } => {
            records.truncate(*count);
            records
        }
        Window::LastDuration { duration } => {
            let cutoff = chrono::Utc::now().timestamp_millis() - duration.as_millis() as i64;
            records.into_iter()
                .filter(|r| r.created_at >= cutoff)
                .collect()
        }
        Window::TopBy { count, field } => {
            sort_records(&mut records, field, false);
            records.truncate(*count);
            records
        }
        Window::Since { condition } => {
            if let Some(pos) = records.iter().position(|r| evaluate_condition(r, condition)) {
                records.truncate(pos + 1);
            }
            records
        }
    }
}

/// Project specific fields from a record
fn project_fields(record: &Record, fields: &[String]) -> Value {
    let mut result = serde_json::Map::new();
    result.insert("id".to_string(), Value::String(record.id.clone()));

    for field in fields {
        if let Some(value) = get_record_field(record, field) {
            let key = field.split('.').last().unwrap_or(field);
            result.insert(key.to_string(), value);
        }
    }

    Value::Object(result)
}
