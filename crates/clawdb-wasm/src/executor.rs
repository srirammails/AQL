//! AQL v0.13 statement executor for the playground (in-memory).
//!
//! ClawDB implements the core of AQL: RECALL / STORE / UPDATE / FORGET / LINK / UNLINK
//! and PIPELINE on one node. BRANCH / FORK / MERGE, SUBSCRIBE / UNSUBSCRIBE / PUBLISH,
//! reads across a NAMESPACE list, IN SCOPE and history (AS OF, WITH HISTORY) are
//! refused: ADB implements them.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use aql_parser::*;
use chrono::Utc;
use serde_json::{json, Map, Value};

use crate::memory::{get_nested_field, Link, MemoryStore, Record};
use crate::QueryResult;

type R = Result<Value, String>;
type Vars = HashMap<String, Value>;

/// Execute a parsed AQL statement
pub fn execute_statement(stmt: &Statement, store: &mut MemoryStore) -> QueryResult {
    match exec(stmt, store) {
        Ok(v) => QueryResult::ok(v),
        Err(e) => QueryResult::err(&e),
    }
}

fn exec(stmt: &Statement, store: &mut MemoryStore) -> R {
    if let Some(e) = refusal(stmt) {
        return Err(e);
    }
    match stmt {
        Statement::Use(ns) => Ok(json!({ "action": "use", "namespace": ns })),
        Statement::Pipeline(p) => execute_pipeline(p, store),
        Statement::Recall(r) => execute_recall(r, store),
        Statement::Store(s) => execute_store(s, store),
        Statement::Update(u) => execute_update(u, store),
        Statement::Forget(f) => execute_forget(f, store),
        Statement::Link(l) => execute_link(l, store),
        Statement::Unlink(u) => execute_unlink(u, store),
        Statement::Branch(_)
        | Statement::Fork(_)
        | Statement::Merge(_)
        | Statement::Subscribe(_)
        | Statement::Unsubscribe(_)
        | Statement::Publish(_) => unreachable!("refused above"),
    }
}

/// What ClawDB doesn't implement (ADB does), as the error to return.
fn refusal(stmt: &Statement) -> Option<String> {
    let adb = |what: &str| Some(format!("{what} is not supported by ClawDB; ADB implements it"));
    match stmt {
        Statement::Branch(_) => adb("BRANCH"),
        Statement::Fork(_) => adb("FORK"),
        Statement::Merge(_) => adb("MERGE (of namespaces)"),
        Statement::Subscribe(_) => adb("SUBSCRIBE"),
        Statement::Unsubscribe(_) => adb("UNSUBSCRIBE"),
        Statement::Publish(_) => adb("PUBLISH"),
        Statement::Pipeline(p) => p.stages.iter().find_map(refusal),
        Statement::Recall(r) => {
            let m = &r.modifiers;
            if m.as_of.is_some() || m.with_history.is_some() {
                if r.memory_type == MemoryType::Working {
                    return Some("AS OF and WITH HISTORY aren't valid on WORKING (it keeps no history)".into());
                }
                return adb("AS OF / WITH HISTORY");
            }
            if !m.namespaces.is_empty() {
                return adb("A read across namespaces (NAMESPACE (…))");
            }
            if m.in_scope.is_some() {
                return adb("IN SCOPE");
            }
            None
        }
        _ => None,
    }
}

// ---------------------------------------------------------------- writes

fn execute_store(stmt: &StoreStmt, store: &mut MemoryStore) -> R {
    let mut data = Map::new();
    for a in &stmt.payload {
        data.insert(a.field.clone(), value_to_json(&a.value));
    }
    let mut record = Record::new(&stmt.memory_type.to_string(), Value::Object(data));
    if let Some(ns) = &stmt.modifiers.namespace {
        record = record.with_namespace(ns.clone());
    }
    if let Some(ttl) = &stmt.modifiers.ttl {
        record = record.with_ttl(ttl.as_millis() as i64);
    }
    let r = store.store(&stmt.memory_type, record)?;
    Ok(json!({ "action": "stored", "memory_type": stmt.memory_type.to_string(), "record": r }))
}

fn execute_update(stmt: &UpdateStmt, store: &mut MemoryStore) -> R {
    let ns = stmt.modifiers.namespace.clone();
    let ttl = stmt.modifiers.ttl;
    let mem = store.get_store_mut(&stmt.memory_type).ok_or("UPDATE names one memory type, not ALL")?;
    let now = Utc::now().timestamp_millis();
    let mut updated = 0;
    for record in mem.values_mut() {
        if record.is_expired() || !record.in_namespace(ns.as_deref()) || !matches_conditions(record, &stmt.conditions) {
            continue;
        }
        if let Value::Object(data) = &mut record.data {
            for a in &stmt.payload {
                match value_to_json(&a.value) {
                    // field = null removes the field
                    Value::Null => {
                        data.remove(&a.field);
                    }
                    v => {
                        data.insert(a.field.clone(), v);
                    }
                }
            }
        }
        if let Some(ttl) = ttl {
            record.ttl_ms = Some(now - record.created_at + ttl.as_millis() as i64);
        }
        record.version += 1;
        record.accessed_at = now;
        updated += 1;
    }
    Ok(json!({ "action": "update", "memory_type": stmt.memory_type.to_string(), "updated": updated, "count": updated }))
}

/// FORGET; without WHERE, everything in the memory type (the parser has made sure
/// a NAMESPACE is named then).
fn execute_forget(stmt: &ForgetStmt, store: &mut MemoryStore) -> R {
    let ns = stmt.modifiers.namespace.as_deref();
    let ids: Vec<String> = store
        .stores_for(&stmt.memory_type)
        .into_iter()
        .flat_map(|s| s.values())
        .filter(|r| r.in_namespace(ns) && matches_conditions(r, &stmt.conditions))
        .map(|r| r.id.clone())
        .collect();
    let deleted = store.forget(&stmt.memory_type, ids);
    Ok(json!({ "action": "forget", "memory_type": stmt.memory_type.to_string(), "deleted": deleted, "count": deleted }))
}

/// The live records of `mem` (ALL: every type) in `ns` matching `conditions`, oldest first.
fn matching(store: &MemoryStore, mem: &MemoryType, conditions: &[Condition], ns: Option<&str>) -> Vec<Record> {
    let mut out: Vec<Record> = store
        .stores_for(mem)
        .into_iter()
        .flat_map(|s| s.values())
        .filter(|r| !r.is_expired() && r.in_namespace(ns) && matches_conditions(r, conditions))
        .cloned()
        .collect();
    out.sort_by_key(|r| r.seq);
    out
}

/// LINK: create, or set the weight of the link of this TYPE that exists (§3).
fn execute_link(stmt: &LinkStmt, store: &mut MemoryStore) -> R {
    let ns = stmt.namespace.as_deref();
    let sources = matching(store, &stmt.from_type, &stmt.from_conditions, ns);
    if sources.is_empty() {
        return Err("No source records found matching conditions".into());
    }
    let targets = matching(store, &stmt.to_type, &stmt.to_conditions, ns);
    if targets.is_empty() {
        return Err("No target records found matching conditions".into());
    }
    let mut n = 0;
    for s in &sources {
        for t in &targets {
            store.upsert_link(s, t, &stmt.link_type, stmt.weight.map(|w| w as f32));
            n += 1;
        }
    }
    Ok(json!({
        "action": "link",
        "link_type": stmt.link_type,
        "source_count": sources.len(),
        "target_count": targets.len(),
        "count": n
    }))
}

/// UNLINK: the links from the matched records to the matched records (of TYPE, if
/// given). No match is no change, not an error.
fn execute_unlink(stmt: &UnlinkStmt, store: &mut MemoryStore) -> R {
    let ns = stmt.namespace.as_deref();
    let sources = matching(store, &stmt.from_type, &stmt.from_conditions, ns);
    let targets = matching(store, &stmt.to_type, &stmt.to_conditions, ns);
    let mut n = 0;
    for s in &sources {
        for t in &targets {
            n += store.unlink(&s.id, &t.id, stmt.link_type.as_deref());
        }
    }
    Ok(json!({ "action": "unlink", "count": n }))
}

// ---------------------------------------------------------------- RECALL

/// RECALL: the access path follows from what is asked — FROM WORKING is the session's
/// state (newest first), FROM TOOLS is ranked by `ranking`, LIKE on PROCEDURAL is the
/// best-matching procedures, LIKE on SEMANTIC searches the seeded concepts, anything
/// else is a filtered read (an exact `WHERE key = …` included).
fn execute_recall(stmt: &RecallStmt, store: &mut MemoryStore) -> R {
    let m = &stmt.modifiers;
    let ns = m.namespace.as_deref();

    let mut records = match &stmt.predicate {
        Predicate::Like { variable, text } => {
            let text = text.as_deref().ok_or_else(|| format!("LIKE ${variable}: no value for ${variable} (bind it in an earlier PIPELINE stage)"))?;
            match stmt.memory_type {
                MemoryType::Semantic => return semantic_like(text, m, store),
                MemoryType::Procedural => procedures_like(store, text, m.confidence.unwrap_or(0.7) as f64, ns),
                t => return Err(format!("LIKE on {t} needs embeddings (similarity search); ADB implements it")),
            }
        }
        Predicate::Where { conditions } => matching(store, &stmt.memory_type, conditions, ns),
        Predicate::All => matching(store, &stmt.memory_type, &[], ns),
    };

    // Default order (records come oldest first)
    match (&stmt.predicate, stmt.memory_type) {
        (Predicate::Like { .. }, _) => {}
        (_, MemoryType::Working) => records.reverse(),
        (_, MemoryType::Tools) => sort_records(&mut records, "ranking", false),
        _ => {}
    }

    if let Some(window) = &m.window {
        records.sort_by(|a, b| b.seq.cmp(&a.seq));
        match window {
            Window::LastN { count } => records.truncate(*count),
            Window::LastDuration { duration } => {
                let cutoff = Utc::now().timestamp_millis() - duration.as_millis() as i64;
                records.retain(|r| r.created_at >= cutoff);
            }
        }
    }

    if m.aggregate.is_some() {
        return aggregate(&records, m);
    }

    if let Some(order) = &m.order_by {
        sort_records(&mut records, &order.field, order.ascending);
    }
    if let Some(limit) = m.limit {
        records.truncate(limit);
    }

    let source_count = records.len();
    if let Some(follow) = &m.follow_links {
        records = follow_links(store, &records, follow);
    }

    let out: Vec<Value> = records
        .iter()
        .map(|r| {
            let mut v = record_json(r, m.return_fields.as_deref());
            if let Some(select) = &m.with_links {
                let links: Vec<Value> = store.links_of(&r.id).filter(|l| selects(select, l)).map(|l| json!(l)).collect();
                v["links"] = Value::Array(links);
            }
            v
        })
        .collect();

    let mut result = json!({
        "action": "recall",
        "memory_type": stmt.memory_type.to_string(),
        "count": out.len(),
        "records": out
    });
    if m.follow_links.is_some() {
        result["source_count"] = json!(source_count);
    }
    Ok(result)
}

/// RECALL FROM SEMANTIC LIKE …: the playground's pre-seeded concepts, by similarity.
fn semantic_like(query: &str, m: &Modifiers, store: &MemoryStore) -> R {
    let limit = m.limit.unwrap_or(5);
    let mut results = store.semantic.search(query, limit);
    if let Some(c) = m.confidence {
        results.retain(|r| r.similarity >= c);
    }
    Ok(json!({
        "action": "recall",
        "memory_type": "SEMANTIC",
        "query": query,
        "count": results.len(),
        "records": results,
        "note": "LIKE on SEMANTIC searches the playground's pre-seeded tech concepts, ranked by similarity."
    }))
}

fn tokens(s: &str) -> HashSet<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(|t| t.to_lowercase()).collect()
}

/// RECALL FROM PROCEDURAL LIKE "text": procedures whose `pattern` (else `name`, else
/// `steps` / `content`) shares at least `threshold` of the text's words, best first.
/// The score is on each record as `_score`.
fn procedures_like(store: &MemoryStore, text: &str, threshold: f64, ns: Option<&str>) -> Vec<Record> {
    let want = tokens(text);
    if want.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(f64, Record)> = matching(store, &MemoryType::Procedural, &[], ns)
        .into_iter()
        .filter_map(|r| {
            let field = ["pattern", "name", "steps", "content"]
                .iter()
                .find_map(|f| r.data.get(*f).and_then(|v| v.as_str()).map(String::from))?;
            let have = tokens(&field);
            let score = want.intersection(&have).count() as f64 / want.len() as f64;
            (score >= threshold).then_some((score, r))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored
        .into_iter()
        .map(|(score, mut r)| {
            if let Value::Object(d) = &mut r.data {
                d.insert("_score".into(), json!(score));
            }
            r
        })
        .collect()
}

/// Whether WITH LINKS / FOLLOW LINKS selects the link (any type or these; at least the weight)
fn selects(select: &LinkSelect, l: &Link) -> bool {
    (select.types.is_empty() || select.types.contains(&l.link_type)) && select.min_weight.is_none_or(|w| l.weight >= w - 1e-9)
}

/// FOLLOW LINKS: the records reached through the selected links, both directions, up
/// to DEPTH hops (1 by default); each once, nearest first, by path weight (the product
/// of the link weights) within a hop. Each carries `_hop` and `_path_weight`.
fn follow_links(store: &MemoryStore, sources: &[Record], follow: &FollowLinks) -> Vec<Record> {
    let depth = follow.depth.unwrap_or(1).max(1);
    let mut visited: HashSet<String> = sources.iter().map(|r| r.id.clone()).collect();
    let mut frontier: Vec<(String, f64)> = sources.iter().map(|r| (r.id.clone(), 1.0)).collect();
    let mut out = Vec::new();
    for hop in 1..=depth {
        let mut found: Vec<(String, f64)> = Vec::new();
        for (id, w) in &frontier {
            for l in store.links_of(id).filter(|l| selects(&follow.select, l)) {
                let other = if l.source_id == *id { &l.target_id } else { &l.source_id };
                if visited.contains(other) {
                    continue;
                }
                let pw = w * l.weight;
                match found.iter_mut().find(|(o, _)| o == other) {
                    Some(e) if pw > e.1 => e.1 = pw,
                    Some(_) => {}
                    None => found.push((other.clone(), pw)),
                }
            }
        }
        if found.is_empty() {
            break;
        }
        found.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        for (id, pw) in &found {
            visited.insert(id.clone());
            if let Some(rec) = store.get_record(id).filter(|r| !r.is_expired()) {
                let mut r = rec.clone();
                if let Value::Object(d) = &mut r.data {
                    d.insert("_hop".into(), json!(hop));
                    d.insert("_path_weight".into(), json!(pw));
                }
                out.push(r);
            }
        }
        frontier = found;
    }
    out
}

/// A record as JSON; RETURN keeps only those fields in `data` (named as written).
fn record_json(r: &Record, fields: Option<&[String]>) -> Value {
    let mut v = serde_json::to_value(r).unwrap_or(Value::Null);
    if let Some(fields) = fields.filter(|f| !f.iter().any(|f| f == "*")) {
        let mut data = Map::new();
        for f in fields {
            if let Some(x) = get_record_field(r, f) {
                data.insert(f.clone(), x);
            }
        }
        v["data"] = Value::Object(data);
    }
    v
}

// ---------------------------------------------------------------- aggregates

/// AGGREGATE (with GROUP BY, HAVING): `values` is one object (null when HAVING
/// suppresses it), or with GROUP BY one row per group. Without GROUP BY, LIMIT applies
/// to the records before aggregating; with it, ORDER BY / LIMIT / RETURN apply to the rows.
fn aggregate(records: &[Record], m: &Modifiers) -> R {
    let aggs = m.aggregate.as_deref().unwrap_or_default();
    let Some(keys) = &m.group_by else {
        let recs: Vec<&Record> = records.iter().take(m.limit.unwrap_or(usize::MAX)).collect();
        let row = agg_row(&recs, aggs);
        let pass = m.having.as_ref().is_none_or(|h| eval_conditions(h, &|f| row.get(f).cloned()));
        let values = if pass { Value::Object(project_row(row, m.return_fields.as_deref())) } else { Value::Null };
        return Ok(json!({ "action": "aggregate", "count": recs.len(), "values": values }));
    };

    let mut groups: Vec<(Vec<Value>, Vec<&Record>)> = Vec::new();
    for r in records {
        let key: Vec<Value> = keys.iter().map(|k| get_record_field(r, k).unwrap_or(Value::Null)).collect();
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, rs)) => rs.push(r),
            None => groups.push((key, vec![r])),
        }
    }
    let mut rows: Vec<Map<String, Value>> = groups
        .into_iter()
        .map(|(key, rs)| {
            let mut row: Map<String, Value> = keys.iter().cloned().zip(key).collect();
            row.extend(agg_row(&rs, aggs));
            row
        })
        .filter(|row| m.having.as_ref().is_none_or(|h| eval_conditions(h, &|f| row.get(f).cloned())))
        .collect();
    if let Some(order) = &m.order_by {
        rows.sort_by(|a, b| order_values(a.get(&order.field), b.get(&order.field), order.ascending));
    }
    if let Some(limit) = m.limit {
        rows.truncate(limit);
    }
    let rows: Vec<Value> = rows.into_iter().map(|r| Value::Object(project_row(r, m.return_fields.as_deref()))).collect();
    Ok(json!({ "action": "aggregate", "count": rows.len(), "values": rows }))
}

fn project_row(row: Map<String, Value>, fields: Option<&[String]>) -> Map<String, Value> {
    match fields.filter(|f| !f.iter().any(|f| f == "*")) {
        Some(fields) => row.into_iter().filter(|(k, _)| fields.contains(k)).collect(),
        None => row,
    }
}

fn number(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        json!(f as i64)
    } else {
        serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null)
    }
}

fn agg_row(records: &[&Record], aggs: &[AggregateFunc]) -> Map<String, Value> {
    let mut row = Map::new();
    for agg in aggs {
        let name = format!("{:?}", agg.func).to_lowercase();
        let alias = agg.alias.clone().unwrap_or_else(|| match &agg.field {
            Some(f) => format!("{name}_{f}"),
            None => name.clone(),
        });
        let nums: Vec<f64> = agg
            .field
            .as_ref()
            .map(|f| records.iter().filter_map(|r| get_record_field(r, f)).filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        let value = match agg.func {
            AggregateFuncType::Count => match &agg.field {
                None => json!(records.len()),
                Some(f) => json!(records.iter().filter(|r| get_record_field(r, f).is_some_and(|v| !v.is_null())).count()),
            },
            AggregateFuncType::Sum => number(nums.iter().sum()),
            AggregateFuncType::Avg if nums.is_empty() => Value::Null,
            AggregateFuncType::Avg => number(nums.iter().sum::<f64>() / nums.len() as f64),
            AggregateFuncType::Min => nums.iter().copied().reduce(f64::min).map(number).unwrap_or(Value::Null),
            AggregateFuncType::Max => nums.iter().copied().reduce(f64::max).map(number).unwrap_or(Value::Null),
        };
        row.insert(alias, value);
    }
    row
}

// ---------------------------------------------------------------- PIPELINE

/// PIPELINE: stages in order (within TIMEOUT, if given); `{name}` takes the value of
/// `name` in the first record (or the aggregate values) of an earlier stage. A failing
/// stage fails the pipeline; the stages before it stay done.
fn execute_pipeline(stmt: &PipelineStmt, store: &mut MemoryStore) -> R {
    let start = Utc::now().timestamp_millis();
    let timeout = stmt.timeout.map(|t| t.as_millis() as i64);
    let name = if stmt.name.is_empty() { String::new() } else { format!(" {}", stmt.name) };
    let mut vars = Vars::new();
    let mut results = Vec::new();
    let mut last = Value::Null;
    for (i, stage) in stmt.stages.iter().enumerate() {
        let elapsed = Utc::now().timestamp_millis() - start;
        if let Some(t) = timeout.filter(|t| elapsed > *t) {
            return Err(format!("PIPELINE{name} timed out ({elapsed}ms > {t}ms) before stage {}", i + 1));
        }
        let out = exec(&bind_statement(stage, &vars), store).map_err(|e| format!("PIPELINE{name} stage {}: {e}", i + 1))?;
        let first = out.get("records").and_then(|r| r.get(0)).and_then(|r| r.get("data")).or_else(|| out.get("values"));
        if let Some(Value::Object(d)) = first {
            vars.extend(d.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        results.push(json!({ "stage": i + 1, "result": { "success": true, "data": out.clone() } }));
        last = out;
    }
    Ok(json!({
        "action": "pipeline",
        "name": stmt.name,
        "status": "completed",
        "stages": stmt.stages.len(),
        "elapsed_ms": Utc::now().timestamp_millis() - start,
        "timeout_ms": timeout,
        "results": results,
        "final_result": { "success": true, "data": last }
    }))
}

fn bind_statement(stmt: &Statement, vars: &Vars) -> Statement {
    let mut s = stmt.clone();
    match &mut s {
        Statement::Recall(r) => match &mut r.predicate {
            Predicate::Where { conditions } => bind_conditions(conditions, vars),
            Predicate::Like { variable, text: text @ None } => {
                *text = vars.get(variable.as_str()).map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string()));
            }
            _ => {}
        },
        Statement::Store(st) => st.payload.iter_mut().for_each(|a| bind_value(&mut a.value, vars)),
        Statement::Update(u) => {
            bind_conditions(&mut u.conditions, vars);
            u.payload.iter_mut().for_each(|a| bind_value(&mut a.value, vars));
        }
        _ => {}
    }
    s
}

fn bind_conditions(conditions: &mut [Condition], vars: &Vars) {
    for c in conditions {
        match c {
            Condition::Simple { value, .. } => bind_value(value, vars),
            Condition::Group { conditions, .. } => bind_conditions(conditions, vars),
        }
    }
}

fn bind_value(value: &mut aql_parser::Value, vars: &Vars) {
    match value {
        aql_parser::Value::Variable(name) => {
            if let Some(v) = vars.get(name.as_str()) {
                *value = json_to_aql(v);
            }
        }
        aql_parser::Value::Array(items) => items.iter_mut().for_each(|v| bind_value(v, vars)),
        _ => {}
    }
}

fn json_to_aql(v: &Value) -> aql_parser::Value {
    match v {
        Value::Null | Value::Object(_) => aql_parser::Value::Null,
        Value::Bool(b) => aql_parser::Value::Bool(*b),
        Value::Number(n) => n.as_i64().map(aql_parser::Value::Int).unwrap_or_else(|| aql_parser::Value::Float(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => aql_parser::Value::String(s.clone()),
        Value::Array(a) => aql_parser::Value::Array(a.iter().map(json_to_aql).collect()),
    }
}

// ---------------------------------------------------------------- conditions

fn matches_conditions(record: &Record, conditions: &[Condition]) -> bool {
    eval_conditions(conditions, &|f| get_record_field(record, f))
}

/// Conditions in order, each joined to the one before by its AND / OR.
fn eval_conditions(conditions: &[Condition], get: &dyn Fn(&str) -> Option<Value>) -> bool {
    let mut result = true;
    for (i, c) in conditions.iter().enumerate() {
        let v = match c {
            Condition::Simple { field, operator, value, .. } => {
                get(field).is_some_and(|fv| compare_values(&fv, operator, &value_to_json(value)))
            }
            Condition::Group { conditions, .. } => eval_conditions(conditions, get),
        };
        result = if i == 0 {
            v
        } else {
            match c.logical_op() {
                Some(LogicalOp::Or) => result || v,
                _ => result && v,
            }
        };
    }
    result
}

/// A field of a record: its id and metadata by name (`metadata.x` or `x`), else data.
fn get_record_field(record: &Record, path: &str) -> Option<Value> {
    let meta = path.strip_prefix("metadata.").unwrap_or(path);
    match meta {
        "id" => return Some(Value::String(record.id.clone())),
        "created_at" => return Some(json!(record.created_at)),
        "accessed_at" => return Some(json!(record.accessed_at)),
        "version" => return Some(json!(record.version)),
        "memory_type" => return Some(Value::String(record.memory_type.clone())),
        "namespace" => return record.namespace.clone().map(Value::String),
        "scope" => return record.scope.clone().map(Value::String),
        _ => {}
    }
    get_nested_field(&record.data, path.strip_prefix("data.").unwrap_or(path)).cloned()
}

fn compare_values(left: &Value, operator: &Operator, right: &Value) -> bool {
    let cmp = |r: &Value| json_compare(left, r);
    match operator {
        Operator::Eq => json_equals(left, right),
        Operator::Ne => !json_equals(left, right),
        Operator::Gt => cmp(right) == Some(Ordering::Greater),
        Operator::Gte => matches!(cmp(right), Some(Ordering::Greater | Ordering::Equal)),
        Operator::Lt => cmp(right) == Some(Ordering::Less),
        Operator::Lte => matches!(cmp(right), Some(Ordering::Less | Ordering::Equal)),
        Operator::Contains => match (left, right) {
            (Value::String(s), Value::String(t)) => s.contains(t.as_str()),
            (Value::Array(arr), _) => arr.iter().any(|v| json_equals(v, right)),
            _ => false,
        },
        Operator::StartsWith => matches!((left, right), (Value::String(s), Value::String(t)) if s.starts_with(t.as_str())),
        Operator::EndsWith => matches!((left, right), (Value::String(s), Value::String(t)) if s.ends_with(t.as_str())),
        Operator::In => match right {
            Value::Array(arr) => arr.iter().any(|v| json_equals(left, v)),
            v => json_equals(left, v),
        },
        Operator::Between => match right {
            Value::Array(b) if b.len() == 2 => {
                matches!(cmp(&b[0]), Some(Ordering::Greater | Ordering::Equal)) && matches!(cmp(&b[1]), Some(Ordering::Less | Ordering::Equal))
            }
            _ => false,
        },
    }
}

fn json_equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

/// A time string (RFC 3339, or a date) as milliseconds since the epoch.
fn time_ms(s: &str) -> Option<i64> {
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(t.timestamp_millis());
    }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0)).map(|t| t.and_utc().timestamp_millis())
}

/// Numbers with numbers, strings with strings, and a record time (milliseconds) with a
/// time string.
fn json_compare(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64()?.partial_cmp(&y.as_f64()?),
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        (Value::Number(x), Value::String(y)) => x.as_i64()?.partial_cmp(&time_ms(y)?),
        (Value::String(x), Value::Number(y)) => time_ms(x)?.partial_cmp(&y.as_i64()?),
        _ => None,
    }
}

/// Sort order of two values; missing / null values last when ascending.
fn order_values(a: Option<&Value>, b: Option<&Value>, ascending: bool) -> Ordering {
    let a = a.filter(|v| !v.is_null());
    let b = b.filter(|v| !v.is_null());
    let o = match (a, b) {
        (Some(x), Some(y)) => json_compare(x, y).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    if ascending { o } else { o.reverse() }
}

fn sort_records(records: &mut [Record], field: &str, ascending: bool) {
    records.sort_by(|a, b| order_values(get_record_field(a, field).as_ref(), get_record_field(b, field).as_ref(), ascending));
}

fn value_to_json(value: &aql_parser::Value) -> Value {
    match value {
        aql_parser::Value::Null => Value::Null,
        aql_parser::Value::Bool(b) => Value::Bool(*b),
        aql_parser::Value::Int(i) => json!(i),
        aql_parser::Value::Float(f) => serde_json::Number::from_f64(*f).map(Value::Number).unwrap_or(Value::Null),
        aql_parser::Value::String(s) => Value::String(s.clone()),
        aql_parser::Value::Variable(v) => Value::String(format!("${}", v)),
        aql_parser::Value::Array(arr) => Value::Array(arr.iter().map(value_to_json).collect()),
    }
}
