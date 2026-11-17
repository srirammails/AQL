//! AQL v0.13 executor for ClawDB.
//!
//! ClawDB implements the core of AQL on one node: RECALL / STORE / UPDATE / FORGET /
//! LINK / UNLINK and PIPELINE. BRANCH / FORK / MERGE, SUBSCRIBE / UNSUBSCRIBE /
//! PUBLISH, reads across a NAMESPACE list, IN SCOPE and history (AS OF, WITH HISTORY)
//! are refused: ADB implements them.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use aql_parser::{self, Condition, LinkSelect, LogicalOp, MemoryType as AqlMemoryType, Modifiers, Operator, Predicate, Statement};
use serde_json::{json, Map, Value};
use tokio::sync::RwLock;

use crate::backends::{compare_values, json_compare, LanceBackend, MemoryBackend};
use crate::config::Config;
use crate::db::{Link, LinksStorage};
use crate::error::{ClawError, ClawResult};
use crate::script;
use crate::types::{MemoryRecord, MemoryType, QueryData, QueryMetadata, QueryResult};

type Backends = HashMap<MemoryType, Arc<LanceBackend>>;

/// AQL Executor
pub struct Executor {
    config: Config,
}

impl Executor {
    /// Create a new executor
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Execute an AQL query string (a statement, or a script)
    pub async fn execute(&self, aql: &str, backends: &Backends, links: &Arc<RwLock<LinksStorage>>) -> ClawResult<QueryResult> {
        self.execute_with_vars(aql, backends, links, HashMap::new()).await
    }

    /// Execute an AQL script with variables: statements separated by `;`, `USE "ns"`
    /// naming the namespace of the statements after it. The answer is the last
    /// statement's. A failing statement stops the script ("statement N: …", N counting
    /// USE statements); the statements before it stay done. A script that doesn't
    /// parse is an `Err`.
    pub async fn execute_with_vars(
        &self,
        aql: &str,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: HashMap<String, Value>,
    ) -> ClawResult<QueryResult> {
        let start = Instant::now();
        let stmts = match aql_parser::parse_script(aql, None) {
            Ok(s) => s,
            Err(e) if script::split(aql).len() > 1 => return Err(ClawError::InvalidQuery(script::parse_failure(aql, &e))),
            Err(e) => return Err(e.into()),
        };
        let (positions, written) = script::positions(aql, stmts.len());

        let mut answer = Ok(QueryData::Message {
            message: "USE names the namespace of the statements after it in the same script".into(),
        });
        for (stmt, n) in stmts.iter().zip(positions) {
            answer = self.execute_statement(stmt, backends, links, &variables).await;
            if let Err(e) = answer {
                answer = Err(if written > 1 { ClawError::InvalidQuery(format!("statement {n}: {e}")) } else { e });
                break;
            }
        }

        let metadata = QueryMetadata { execution_time_ms: start.elapsed().as_millis() as u64, ..Default::default() };
        Ok(match answer {
            Ok(data) => QueryResult { success: true, data: Some(data), error: None, metadata },
            Err(e) => QueryResult { success: false, data: None, error: Some(e.to_string()), metadata },
        })
    }

    /// Execute a parsed statement
    async fn execute_statement(
        &self,
        stmt: &Statement,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        if let Some(e) = refusal(stmt) {
            return Err(ClawError::InvalidQuery(e));
        }
        match stmt {
            Statement::Use(ns) => Ok(QueryData::Message { message: format!("USE \"{ns}\"") }),
            Statement::Store(s) => self.execute_store(s, backends, variables).await,
            Statement::Recall(r) => self.execute_recall(r, backends, links, variables).await,
            Statement::Forget(f) => self.execute_forget(f, backends, links, variables).await,
            Statement::Update(u) => self.execute_update(u, backends, variables).await,
            Statement::Link(l) => self.execute_link(l, backends, links, variables).await,
            Statement::Unlink(u) => self.execute_unlink(u, backends, links, variables).await,
            Statement::Pipeline(p) => self.execute_pipeline(p, backends, links, variables).await,
            Statement::Branch(_)
            | Statement::Fork(_)
            | Statement::Merge(_)
            | Statement::Subscribe(_)
            | Statement::Unsubscribe(_)
            | Statement::Publish(_) => unreachable!("refused above"),
        }
    }

    fn backend<'a>(&self, backends: &'a Backends, mt: AqlMemoryType) -> ClawResult<&'a Arc<LanceBackend>> {
        let memory_type: MemoryType = mt.into();
        backends.get(&memory_type).ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))
    }

    /// The backends a statement on `mt` touches (ALL: every one)
    fn backends_for<'a>(&self, backends: &'a Backends, mt: AqlMemoryType) -> ClawResult<Vec<&'a Arc<LanceBackend>>> {
        match mt {
            AqlMemoryType::All => Ok(backends.values().collect()),
            t => Ok(vec![self.backend(backends, t)?]),
        }
    }

    /// The live records of `mt` (ALL: every type) in `namespace` matching `conditions`,
    /// oldest first.
    async fn read(&self, backends: &Backends, mt: AqlMemoryType, conditions: &[Condition], namespace: Option<&str>) -> ClawResult<Vec<MemoryRecord>> {
        let mods = Modifiers { namespace: namespace.map(String::from), ..Default::default() };
        let mut out = Vec::new();
        for b in self.backends_for(backends, mt)? {
            out.extend(b.query(conditions, &mods).await?);
        }
        out.sort_by_key(|r| r.metadata.created_at);
        Ok(out)
    }

    /// Execute STORE statement
    async fn execute_store(&self, stmt: &aql_parser::StoreStmt, backends: &Backends, variables: &HashMap<String, Value>) -> ClawResult<QueryData> {
        let backend = self.backend(backends, stmt.memory_type)?;
        let mut data = Map::new();
        for assignment in &stmt.payload {
            data.insert(assignment.field.clone(), self.resolve_value(&assignment.value, variables));
        }
        let mut record = MemoryRecord::new(stmt.memory_type.into(), Value::Object(data))
            .with_namespace(stmt.modifiers.namespace.as_deref().unwrap_or(&self.config.default_namespace))
            .with_scope(&self.config.default_scope);
        if let Some(ttl) = &stmt.modifiers.ttl {
            record = record.with_ttl_ms(ttl.as_millis() as i64);
        }
        let stored = backend.store(record).await?;
        Ok(QueryData::Stored { record: stored })
    }

    /// RECALL: the access path follows from what is asked — FROM WORKING is the
    /// session's state (newest first), FROM TOOLS is ranked by `ranking`, LIKE "text"
    /// on PROCEDURAL is the best-matching procedures, anything else is a filtered read
    /// (an exact `WHERE key = …` / `WHERE id = …` included).
    async fn execute_recall(
        &self,
        stmt: &aql_parser::RecallStmt,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let m = &stmt.modifiers;
        let ns = m.namespace.as_deref();

        let mut records = match &stmt.predicate {
            Predicate::Like { variable, text } => {
                let text = text
                    .clone()
                    .or_else(|| variables.get(variable).map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string())))
                    .ok_or_else(|| ClawError::InvalidQuery(format!("LIKE ${variable}: no value for ${variable}")))?;
                if stmt.memory_type != AqlMemoryType::Procedural {
                    return Err(ClawError::InvalidQuery(
                        "LIKE requires semantic memory with embeddings (similarity search); ADB implements it".into(),
                    ));
                }
                let candidates = self.read(backends, AqlMemoryType::Procedural, &[], ns).await?;
                procedures_like(candidates, &text, m.confidence.unwrap_or(0.7) as f64)
            }
            Predicate::Where { conditions } => self.read(backends, stmt.memory_type, &self.bind_conditions(conditions, variables), ns).await?,
            Predicate::All => self.read(backends, stmt.memory_type, &[], ns).await?,
        };

        // Default order (records come oldest first)
        match (&stmt.predicate, stmt.memory_type) {
            (Predicate::Like { .. }, _) => {}
            (_, AqlMemoryType::Working) => records.reverse(),
            (_, AqlMemoryType::Tools) => sort_records(&mut records, "ranking", false),
            _ => {}
        }

        if let Some(window) = &m.window {
            records.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));
            match window {
                aql_parser::Window::LastN { count } => records.truncate(*count),
                aql_parser::Window::LastDuration { duration } => {
                    let cutoff = chrono::Utc::now() - chrono::Duration::milliseconds(duration.as_millis() as i64);
                    records.retain(|r| r.metadata.created_at >= cutoff);
                }
            }
        }

        if m.aggregate.is_some() {
            return Ok(aggregate(&records, m));
        }

        if let Some(order) = &m.order_by {
            sort_records(&mut records, &order.field, order.ascending);
        }
        if let Some(limit) = m.limit {
            records.truncate(limit);
        }

        if let Some(follow) = &m.follow_links {
            records = self.follow_links(&records, backends, links, follow).await?;
        }
        if let Some(select) = &m.with_links {
            let guard = links.read().await;
            for r in &mut records {
                r.links = Some(guard.iter().filter(|l| (l.source_id == r.id || l.target_id == r.id) && selects(select, l)).map(link_json).collect());
            }
        }

        let count = records.len();
        let records = self.apply_return_fields(records, m);
        Ok(QueryData::Records { records, count })
    }

    /// FOLLOW LINKS: the records reached through the selected links, both directions,
    /// up to DEPTH hops (1 by default); each once, nearest first, by path weight (the
    /// product of the link weights) within a hop. Each carries `_hop` and `_path_weight`.
    async fn follow_links(
        &self,
        sources: &[MemoryRecord],
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        follow: &aql_parser::FollowLinks,
    ) -> ClawResult<Vec<MemoryRecord>> {
        let depth = follow.depth.unwrap_or(1).max(1);
        let guard = links.read().await;
        let mut visited: HashSet<String> = sources.iter().map(|r| r.id.clone()).collect();
        let mut frontier: Vec<(String, f64)> = sources.iter().map(|r| (r.id.clone(), 1.0)).collect();
        let mut out = Vec::new();
        for hop in 1..=depth {
            let mut found: Vec<(String, MemoryType, f64)> = Vec::new();
            for (id, w) in &frontier {
                for l in guard.iter().filter(|l| (l.source_id == *id || l.target_id == *id) && selects(&follow.select, l)) {
                    let (other, other_type) = if l.source_id == *id { (&l.target_id, l.target_type) } else { (&l.source_id, l.source_type) };
                    if visited.contains(other) {
                        continue;
                    }
                    let pw = w * link_weight(l);
                    match found.iter_mut().find(|(o, _, _)| o == other) {
                        Some(e) if pw > e.2 => e.2 = pw,
                        Some(_) => {}
                        None => found.push((other.clone(), other_type, pw)),
                    }
                }
            }
            if found.is_empty() {
                break;
            }
            found.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));
            for (id, mt, pw) in &found {
                visited.insert(id.clone());
                let Some(backend) = backends.get(mt) else { continue };
                if let Some(mut r) = backend.get_by_id(id).await?.filter(|r| !r.metadata.is_expired()) {
                    if let Value::Object(d) = &mut r.data {
                        d.insert("_hop".into(), json!(hop));
                        d.insert("_path_weight".into(), json!(pw));
                    }
                    out.push(r);
                }
            }
            frontier = found.into_iter().map(|(id, _, w)| (id, w)).collect();
        }
        Ok(out)
    }

    /// FORGET; without WHERE, everything in the memory type (the parser has made sure
    /// a NAMESPACE is named then). The records' links go with them.
    async fn execute_forget(
        &self,
        stmt: &aql_parser::ForgetStmt,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let conditions = in_namespace(self.bind_conditions(&stmt.conditions, variables), stmt.modifiers.namespace.as_deref());
        let mut ids = HashSet::new();
        let mut count = 0;
        for backend in self.backends_for(backends, stmt.memory_type)? {
            ids.extend(backend.query(&conditions, &Modifiers::default()).await?.into_iter().map(|r| r.id));
            count += backend.delete(&conditions).await?;
        }
        links.write().await.retain(|l| !ids.contains(&l.source_id) && !ids.contains(&l.target_id));
        Ok(QueryData::Affected { count })
    }

    /// UPDATE; `field = null` removes the field.
    async fn execute_update(&self, stmt: &aql_parser::UpdateStmt, backends: &Backends, variables: &HashMap<String, Value>) -> ClawResult<QueryData> {
        let backend = self.backend(backends, stmt.memory_type)?;
        let mut updates = Map::new();
        for assignment in &stmt.payload {
            updates.insert(assignment.field.clone(), self.resolve_value(&assignment.value, variables));
        }
        let conditions = in_namespace(self.bind_conditions(&stmt.conditions, variables), stmt.modifiers.namespace.as_deref());
        let count = backend.update(&conditions, Value::Object(updates)).await?;
        Ok(QueryData::Affected { count })
    }

    /// LINK: create, or set the weight of the link of this TYPE that exists between the
    /// two records (no WEIGHT keeps the weight). Both ends are in the statement's namespace.
    async fn execute_link(
        &self,
        stmt: &aql_parser::LinkStmt,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let ns = stmt.namespace.as_deref();
        let sources = self.read(backends, stmt.from_type, &self.bind_conditions(&stmt.from_conditions, variables), ns).await?;
        if sources.is_empty() {
            return Err(ClawError::InvalidQuery("No source records found for LINK".into()));
        }
        let targets = self.read(backends, stmt.to_type, &self.bind_conditions(&stmt.to_conditions, variables), ns).await?;
        if targets.is_empty() {
            return Err(ClawError::InvalidQuery("No target records found for LINK".into()));
        }

        let mut guard = links.write().await;
        let mut count = 0;
        for s in &sources {
            for t in &targets {
                match guard.iter_mut().find(|l| l.source_id == s.id && l.target_id == t.id && l.link_type == stmt.link_type) {
                    Some(l) => {
                        if stmt.weight.is_some() {
                            l.weight = stmt.weight.map(|w| w as f32);
                        }
                    }
                    None => guard.push(Link {
                        source_id: s.id.clone(),
                        source_type: s.memory_type,
                        target_id: t.id.clone(),
                        target_type: t.memory_type,
                        link_type: stmt.link_type.clone(),
                        weight: stmt.weight.map(|w| w as f32),
                    }),
                }
                count += 1;
            }
        }
        Ok(QueryData::Affected { count })
    }

    /// UNLINK: the links from the matched records to the matched records (of TYPE, if
    /// given). No match is no change, not an error.
    async fn execute_unlink(
        &self,
        stmt: &aql_parser::UnlinkStmt,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let ns = stmt.namespace.as_deref();
        let sources: HashSet<String> =
            self.read(backends, stmt.from_type, &self.bind_conditions(&stmt.from_conditions, variables), ns).await?.into_iter().map(|r| r.id).collect();
        let targets: HashSet<String> =
            self.read(backends, stmt.to_type, &self.bind_conditions(&stmt.to_conditions, variables), ns).await?.into_iter().map(|r| r.id).collect();
        let mut guard = links.write().await;
        let before = guard.len();
        guard.retain(|l| {
            !(sources.contains(&l.source_id) && targets.contains(&l.target_id) && stmt.link_type.as_ref().is_none_or(|t| *t == l.link_type))
        });
        Ok(QueryData::Affected { count: before - guard.len() })
    }

    /// PIPELINE: stages in order (within TIMEOUT, if given); `{name}` takes the value of
    /// `name` in the first record (or the aggregate values) of an earlier stage. The
    /// answer is the last stage's. A failing stage fails the pipeline; the stages
    /// before it stay done.
    async fn execute_pipeline(
        &self,
        stmt: &aql_parser::PipelineStmt,
        backends: &Backends,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let start = Instant::now();
        let name = if stmt.name.is_empty() { String::new() } else { format!(" {}", stmt.name) };
        let mut vars = variables.clone();
        let mut last = QueryData::Message { message: format!("PIPELINE{name}: no stages") };
        for (i, stage) in stmt.stages.iter().enumerate() {
            if let Some(t) = stmt.timeout.filter(|t| start.elapsed() > *t) {
                return Err(ClawError::InvalidQuery(format!(
                    "PIPELINE{name} timed out ({}ms > {}ms) before stage {}",
                    start.elapsed().as_millis(),
                    t.as_millis(),
                    i + 1
                )));
            }
            let data = Box::pin(self.execute_statement(stage, backends, links, &vars))
                .await
                .map_err(|e| ClawError::InvalidQuery(format!("PIPELINE{name} stage {}: {e}", i + 1)))?;
            let first = match &data {
                QueryData::Records { records, .. } => records.first().map(|r| r.data.clone()),
                QueryData::Values { values, .. } => Some(values.clone()),
                _ => None,
            };
            if let Some(Value::Object(obj)) = first {
                vars.extend(obj);
            }
            last = data;
        }
        Ok(last)
    }

    /// Bind variables in conditions
    fn bind_conditions(&self, conditions: &[Condition], variables: &HashMap<String, Value>) -> Vec<Condition> {
        conditions
            .iter()
            .map(|c| match c {
                Condition::Simple { field, operator, value, logical_op } => Condition::Simple {
                    field: field.clone(),
                    operator: *operator,
                    value: self.bind_value(value, variables),
                    logical_op: *logical_op,
                },
                Condition::Group { conditions, logical_op } => Condition::Group {
                    conditions: self.bind_conditions(conditions, variables),
                    logical_op: *logical_op,
                },
            })
            .collect()
    }

    /// Bind a variable in a value
    fn bind_value(&self, value: &aql_parser::Value, variables: &HashMap<String, Value>) -> aql_parser::Value {
        match value {
            aql_parser::Value::Variable(name) => variables.get(name).map(json_to_aql_value).unwrap_or_else(|| value.clone()),
            aql_parser::Value::Array(arr) => aql_parser::Value::Array(arr.iter().map(|v| self.bind_value(v, variables)).collect()),
            _ => value.clone(),
        }
    }

    /// Resolve a value (including variable substitution)
    fn resolve_value(&self, value: &aql_parser::Value, variables: &HashMap<String, Value>) -> Value {
        match value {
            aql_parser::Value::Null => Value::Null,
            aql_parser::Value::Bool(b) => Value::Bool(*b),
            aql_parser::Value::Int(i) => Value::Number((*i).into()),
            aql_parser::Value::Float(f) => serde_json::Number::from_f64(*f).map(Value::Number).unwrap_or(Value::Null),
            aql_parser::Value::String(s) => Value::String(s.clone()),
            aql_parser::Value::Variable(name) => variables.get(name).cloned().unwrap_or(Value::Null),
            aql_parser::Value::Array(arr) => Value::Array(arr.iter().map(|v| self.resolve_value(v, variables)).collect()),
        }
    }

    /// Apply RETURN field projection to records
    fn apply_return_fields(&self, records: Vec<MemoryRecord>, modifiers: &Modifiers) -> Vec<MemoryRecord> {
        let return_fields = match &modifiers.return_fields {
            Some(fields) if !fields.iter().any(|f| f == "*") => fields,
            _ => return records,
        };
        records
            .into_iter()
            .map(|mut record| {
                let mut projected = Map::new();
                for field in return_fields {
                    let value = match field.strip_prefix("metadata.") {
                        Some("namespace") => Some(Value::String(record.metadata.namespace.clone())),
                        Some("scope") => Some(Value::String(record.metadata.scope.clone())),
                        Some("version") => Some(Value::Number(record.metadata.version.into())),
                        Some("created_at") => Some(Value::String(record.metadata.created_at.to_rfc3339())),
                        Some("accessed_at") => Some(Value::String(record.metadata.accessed_at.to_rfc3339())),
                        Some(_) => Some(Value::Null),
                        None => record.resolve_field(field),
                    };
                    if let Some(v) = value {
                        projected.insert(field.clone(), v);
                    }
                }
                record.data = Value::Object(projected);
                record
            })
            .collect()
    }
}

/// What ClawDB doesn't implement (ADB does), as the error to return.
pub(crate) fn refusal(stmt: &Statement) -> Option<String> {
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
                if r.memory_type == AqlMemoryType::Working {
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

/// `conditions` AND `namespace = ns` (no namespace named: unchanged)
fn in_namespace(conditions: Vec<Condition>, ns: Option<&str>) -> Vec<Condition> {
    let Some(ns) = ns else { return conditions };
    let mut ns_cond = Condition::simple("namespace".into(), Operator::Eq, aql_parser::Value::String(ns.into()));
    if conditions.is_empty() {
        return vec![ns_cond];
    }
    ns_cond.set_logical_op(Some(LogicalOp::And));
    vec![Condition::group(conditions), ns_cond]
}

fn link_weight(l: &Link) -> f64 {
    l.weight.map(|w| format!("{w}").parse().unwrap_or(w as f64)).unwrap_or(1.0)
}

fn selects(select: &LinkSelect, l: &Link) -> bool {
    (select.types.is_empty() || select.types.contains(&l.link_type)) && select.min_weight.is_none_or(|w| link_weight(l) >= w - 1e-9)
}

fn link_json(l: &Link) -> Value {
    json!({
        "source_id": l.source_id,
        "target_id": l.target_id,
        "from_type": l.source_type.to_string(),
        "to_type": l.target_type.to_string(),
        "link_type": l.link_type,
        "weight": link_weight(l),
    })
}

fn tokens(s: &str) -> HashSet<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(|t| t.to_lowercase()).collect()
}

/// RECALL FROM PROCEDURAL LIKE "text": procedures whose `pattern` (else `name`, else
/// `steps` / `content`) shares at least `threshold` of the text's words, best first.
/// The score is on each record as `_score`.
fn procedures_like(candidates: Vec<MemoryRecord>, text: &str, threshold: f64) -> Vec<MemoryRecord> {
    let want = tokens(text);
    if want.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(f64, MemoryRecord)> = candidates
        .into_iter()
        .filter_map(|r| {
            let field = ["pattern", "name", "steps", "content"].iter().find_map(|f| r.data.get(*f).and_then(|v| v.as_str()).map(String::from))?;
            let score = want.intersection(&tokens(&field)).count() as f64 / want.len() as f64;
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

// ---------------------------------------------------------------- aggregates

/// AGGREGATE (with GROUP BY, HAVING): `values` is one object (null when HAVING
/// suppresses it), or with GROUP BY one row per group. Without GROUP BY, LIMIT applies
/// to the records before aggregating; with it, ORDER BY / LIMIT / RETURN apply to the rows.
fn aggregate(records: &[MemoryRecord], m: &Modifiers) -> QueryData {
    let aggs = m.aggregate.as_deref().unwrap_or_default();
    let Some(keys) = &m.group_by else {
        let recs: Vec<&MemoryRecord> = records.iter().take(m.limit.unwrap_or(usize::MAX)).collect();
        let row = agg_row(&recs, aggs);
        let pass = m.having.as_ref().is_none_or(|h| eval_conditions(h, &|f| row.get(f).cloned()));
        let values = if pass { Value::Object(project_row(row, m.return_fields.as_deref())) } else { Value::Null };
        return QueryData::Values { values, count: recs.len() };
    };

    let mut groups: Vec<(Vec<Value>, Vec<&MemoryRecord>)> = Vec::new();
    for r in records {
        let key: Vec<Value> = keys.iter().map(|k| r.resolve_field(k).unwrap_or(Value::Null)).collect();
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
    QueryData::Values { count: rows.len(), values: Value::Array(rows) }
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

fn agg_row(records: &[&MemoryRecord], aggs: &[aql_parser::AggregateFunc]) -> Map<String, Value> {
    use aql_parser::AggregateFuncType as F;
    let mut row = Map::new();
    for agg in aggs {
        let name = format!("{:?}", agg.func).to_lowercase();
        let alias = agg.alias.clone().unwrap_or_else(|| match &agg.field {
            Some(f) => format!("{name}_{f}"),
            None => name.clone(),
        });
        let nums: Vec<f64> =
            agg.field.as_ref().map(|f| records.iter().filter_map(|r| r.resolve_field(f)).filter_map(|v| v.as_f64()).collect()).unwrap_or_default();
        let value = match agg.func {
            F::Count => match &agg.field {
                None => json!(records.len()),
                Some(f) => json!(records.iter().filter(|r| r.resolve_field(f).is_some_and(|v| !v.is_null())).count()),
            },
            F::Sum => number(nums.iter().sum()),
            F::Avg if nums.is_empty() => Value::Null,
            F::Avg => number(nums.iter().sum::<f64>() / nums.len() as f64),
            F::Min => nums.iter().copied().reduce(f64::min).map(number).unwrap_or(Value::Null),
            F::Max => nums.iter().copied().reduce(f64::max).map(number).unwrap_or(Value::Null),
        };
        row.insert(alias, value);
    }
    row
}

/// Conditions in order, each joined to the one before by its AND / OR.
fn eval_conditions(conditions: &[Condition], get: &dyn Fn(&str) -> Option<Value>) -> bool {
    let mut result = true;
    for (i, c) in conditions.iter().enumerate() {
        let v = match c {
            Condition::Simple { field, operator, value, .. } => get(field).is_some_and(|fv| compare_values(&fv, operator, value)),
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

/// Sort order of two values; missing / null values last when ascending.
fn order_values(a: Option<&Value>, b: Option<&Value>, ascending: bool) -> Ordering {
    let a = a.filter(|v| !v.is_null());
    let b = b.filter(|v| !v.is_null());
    let o = match (a, b) {
        (Some(x), Some(y)) => json_compare(x, y).map(|c| c.cmp(&0)).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    if ascending {
        o
    } else {
        o.reverse()
    }
}

fn sort_records(records: &mut [MemoryRecord], field: &str, ascending: bool) {
    records.sort_by(|a, b| order_values(a.resolve_field(field).as_ref(), b.resolve_field(field).as_ref(), ascending));
}

/// Convert JSON value to AQL value
fn json_to_aql_value(v: &Value) -> aql_parser::Value {
    match v {
        Value::Null | Value::Object(_) => aql_parser::Value::Null,
        Value::Bool(b) => aql_parser::Value::Bool(*b),
        Value::Number(n) => n.as_i64().map(aql_parser::Value::Int).unwrap_or_else(|| aql_parser::Value::Float(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => aql_parser::Value::String(s.clone()),
        Value::Array(arr) => aql_parser::Value::Array(arr.iter().map(json_to_aql_value).collect()),
    }
}
