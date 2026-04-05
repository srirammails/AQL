//! AQL Statement Executor
//!
//! Executes parsed AQL statements against the in-memory store.

use aql_parser::*;
use serde_json::{json, Value};

use crate::memory::{MemoryStore, Record};
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
        Statement::Link(_) => QueryResult::err("LINK not supported in playground"),
        Statement::Reflect(_) => QueryResult::err("REFLECT not supported in playground"),
        Statement::Pipeline(_) => QueryResult::err("PIPELINE not supported in playground"),
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

/// Execute RECALL statement (for EPISODIC, SEMANTIC, etc.)
fn execute_recall(stmt: &RecallStmt, store: &mut MemoryStore) -> QueryResult {
    // Handle SEMANTIC with LIKE predicate
    if matches!(stmt.memory_type, MemoryType::Semantic) {
        return execute_semantic_recall(stmt, store);
    }

    let records = match query_records(store, &stmt.memory_type, &stmt.predicate, &stmt.modifiers) {
        Ok(r) => r,
        Err(e) => return QueryResult::err(&e),
    };

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
            // Try to extract a string value from conditions for keyword search
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

/// Execute LOOKUP statement (for PROCEDURAL, TOOLS, etc.)
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

    // Get all non-expired records
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

    // Apply RETURN fields projection
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

    // Find matching record IDs
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
            // Apply updates to data
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

    // Apply LIMIT
    if let Some(limit) = modifiers.limit {
        records.truncate(limit);
    }

    // Apply RETURN fields projection
    let results: Vec<Value> = if let Some(fields) = &modifiers.return_fields {
        records.iter().map(|r| project_fields(r, fields)).collect()
    } else {
        records.iter().map(|r| serde_json::to_value(r).unwrap()).collect()
    };

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
        Predicate::Like { .. } => true, // Handled separately for semantic
        Predicate::Pattern { variable, .. } => {
            // Simple pattern matching - check if any field contains the pattern
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
    // Handle metadata fields
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

    // Handle data fields
    let field_path = if path.starts_with("data.") {
        &path[5..]
    } else {
        path
    };

    get_nested_json(&record.data, field_path)
}

/// Get nested field from JSON
fn get_nested_json(value: &Value, path: &str) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current.clone())
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
        (Value::Number(a), Value::Number(b)) => {
            a.as_f64() == b.as_f64()
        }
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
        (Value::String(a), Value::String(b)) => {
            Some(a.cmp(b) as i32)
        }
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
            // Find first record matching condition, return all after
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

    // Always include id
    result.insert("id".to_string(), Value::String(record.id.clone()));

    for field in fields {
        if let Some(value) = get_record_field(record, field) {
            // Use the last part of the path as the key
            let key = field.split('.').last().unwrap_or(field);
            result.insert(key.to_string(), value);
        }
    }

    Value::Object(result)
}
