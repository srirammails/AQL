//! AQL Statement Executor for ClawDB

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use aql_parser::{self, Modifiers, Predicate, Statement, MemoryType as AqlMemoryType};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::backends::{LanceBackend, MemoryBackend};
use crate::config::Config;
use crate::db::{Link, LinksStorage};
use crate::error::{ClawError, ClawResult};
use crate::types::{MemoryRecord, MemoryType, QueryData, QueryMetadata, QueryResult};

/// AQL Executor
pub struct Executor {
    config: Config,
}

impl Executor {
    /// Create a new executor
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Execute an AQL query string
    pub async fn execute(
        &self,
        aql: &str,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
    ) -> ClawResult<QueryResult> {
        self.execute_with_vars(aql, backends, links, HashMap::new()).await
    }

    /// Execute an AQL query with variables
    pub async fn execute_with_vars(
        &self,
        aql: &str,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        variables: HashMap<String, Value>,
    ) -> ClawResult<QueryResult> {
        let start = Instant::now();

        // Parse the query
        let stmt = aql_parser::parse(aql)?;

        // Execute the statement
        let result = self.execute_statement(&stmt, backends, links, &variables).await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(data) => Ok(QueryResult {
                success: true,
                data: Some(data),
                error: None,
                metadata: QueryMetadata {
                    execution_time_ms,
                    records_scanned: None,
                    records_returned: None,
                },
            }),
            Err(e) => Ok(QueryResult {
                success: false,
                data: None,
                error: Some(e.to_string()),
                metadata: QueryMetadata {
                    execution_time_ms,
                    ..Default::default()
                },
            }),
        }
    }

    /// Execute a parsed statement
    async fn execute_statement(
        &self,
        stmt: &Statement,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        match stmt {
            Statement::Store(s) => self.execute_store(s, backends, variables).await,
            Statement::Recall(r) => self.execute_recall(r, backends, links, variables).await,
            Statement::Lookup(l) => self.execute_lookup(l, backends, variables).await,
            Statement::Load(l) => self.execute_load(l, backends, variables).await,
            Statement::Scan(s) => self.execute_scan(s, backends).await,
            Statement::Forget(f) => self.execute_forget(f, backends, variables).await,
            Statement::Update(u) => self.execute_update(u, backends, variables).await,
            Statement::Link(l) => self.execute_link(l, backends, links, variables).await,
            Statement::Reflect(r) => self.execute_reflect(r, backends, links, variables).await,
            Statement::Pipeline(p) => self.execute_pipeline(p, backends, links, variables).await,
        }
    }

    /// Execute STORE statement
    async fn execute_store(
        &self,
        stmt: &aql_parser::StoreStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // Validate: Cannot STORE INTO ALL
        if matches!(stmt.memory_type, AqlMemoryType::All) {
            return Err(ClawError::InvalidQuery("Cannot STORE INTO ALL".into()));
        }

        let memory_type: MemoryType = stmt.memory_type.into();
        let backend = backends
            .get(&memory_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))?;

        // Build data from payload
        let mut data = serde_json::Map::new();
        for assignment in &stmt.payload {
            let value = self.resolve_value(&assignment.value, variables);
            data.insert(assignment.field.clone(), value);
        }

        // Create record
        let mut record = MemoryRecord::new(memory_type, Value::Object(data));

        // Apply modifiers
        if let Some(ns) = &stmt.modifiers.namespace {
            record = record.with_namespace(ns);
        } else {
            record = record.with_namespace(&self.config.default_namespace);
        }

        if let Some(scope) = &stmt.modifiers.scope {
            record = record.with_scope(format!("{:?}", scope).to_lowercase());
        } else {
            record = record.with_scope(&self.config.default_scope);
        }

        if let Some(ttl) = &stmt.modifiers.ttl {
            record = record.with_ttl_ms(ttl.as_millis() as i64);
        }

        // Store the record
        let stored = backend.store(record).await?;

        Ok(QueryData::Stored { record: stored })
    }

    /// Execute RECALL statement
    async fn execute_recall(
        &self,
        stmt: &aql_parser::RecallStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // Handle FROM ALL
        if matches!(stmt.memory_type, AqlMemoryType::All) {
            return self.execute_recall_all(backends, &stmt.predicate, &stmt.modifiers, variables).await;
        }

        let memory_type: MemoryType = stmt.memory_type.into();
        let backend = backends
            .get(&memory_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))?;

        let conditions = self.extract_conditions(&stmt.predicate, variables)?;
        let mut records = backend.query(&conditions, &stmt.modifiers).await?;

        // Handle FOLLOW LINKS
        if let Some(follow) = &stmt.modifiers.follow_links {
            records = self.follow_links(&records, backends, links, &follow.link_type, follow.depth).await?;
        }

        let count = records.len();

        // Apply RETURN field projection
        let records = self.apply_return_fields(records, &stmt.modifiers);

        Ok(QueryData::Records { records, count })
    }

    /// Follow links from source records to get linked records
    async fn follow_links(
        &self,
        source_records: &[MemoryRecord],
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        link_type: &str,
        _depth: Option<u32>,
    ) -> ClawResult<Vec<MemoryRecord>> {
        let links_guard = links.read().await;
        let source_ids: Vec<&str> = source_records.iter().map(|r| r.id.as_str()).collect();

        // Find links from source records with matching type
        let matching_links: Vec<&Link> = links_guard
            .iter()
            .filter(|l| source_ids.contains(&l.source_id.as_str()) && l.link_type == link_type)
            .collect();

        if matching_links.is_empty() {
            return Ok(Vec::new());
        }

        // Get target records
        let mut result_records = Vec::new();
        for link in matching_links {
            // Query the target backend for the linked record
            if let Some(backend) = backends.get(&link.target_type) {
                let conditions = vec![aql_parser::Condition::Simple {
                    field: "id".to_string(),
                    operator: aql_parser::Operator::Eq,
                    value: aql_parser::Value::String(link.target_id.clone()),
                    logical_op: None,
                }];
                if let Ok(records) = backend.query(&conditions, &Modifiers::default()).await {
                    result_records.extend(records);
                }
            }
        }

        Ok(result_records)
    }

    /// Execute RECALL FROM ALL
    async fn execute_recall_all(
        &self,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        predicate: &Predicate,
        modifiers: &Modifiers,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let conditions = self.extract_conditions(predicate, variables)?;
        let mut all_records = Vec::new();

        for backend in backends.values() {
            if let Ok(records) = backend.query(&conditions, modifiers).await {
                all_records.extend(records);
            }
        }

        // Apply ORDER BY if specified
        if let Some(order) = &modifiers.order_by {
            all_records.sort_by(|a, b| {
                let va = a.resolve_field(&order.field);
                let vb = b.resolve_field(&order.field);
                let cmp = compare_json_values(&va, &vb);
                if order.ascending { cmp } else { cmp.reverse() }
            });
        }

        // Apply LIMIT
        if let Some(limit) = modifiers.limit {
            all_records.truncate(limit);
        }

        let count = all_records.len();
        let records = self.apply_return_fields(all_records, modifiers);

        Ok(QueryData::Records { records, count })
    }

    /// Execute LOOKUP statement
    async fn execute_lookup(
        &self,
        stmt: &aql_parser::LookupStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // Validation: LOOKUP only valid on SEMANTIC, PROCEDURAL, TOOLS
        match stmt.memory_type {
            AqlMemoryType::Working => {
                return Err(ClawError::InvalidQuery("LOOKUP not valid on WORKING memory. Use SCAN or RECALL instead.".into()));
            }
            AqlMemoryType::Episodic => {
                return Err(ClawError::InvalidQuery("LOOKUP not valid on EPISODIC memory. Use RECALL instead.".into()));
            }
            _ => {}
        }

        let memory_type: MemoryType = stmt.memory_type.into();
        let backend = backends
            .get(&memory_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))?;

        let conditions = self.extract_conditions(&stmt.predicate, variables)?;
        let records = backend.query(&conditions, &stmt.modifiers).await?;
        let count = records.len();

        let records = self.apply_return_fields(records, &stmt.modifiers);

        Ok(QueryData::Records { records, count })
    }

    /// Execute LOAD statement (TOOLS only by design)
    async fn execute_load(
        &self,
        stmt: &aql_parser::LoadStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // LOAD is always TOOLS-only by definition (no memory_type field in LoadStmt)
        let backend = backends
            .get(&MemoryType::Tools)
            .ok_or_else(|| ClawError::InvalidMemoryType("TOOLS".into()))?;

        let conditions = self.extract_conditions(&stmt.predicate, variables)?;
        let records = backend.query(&conditions, &stmt.modifiers).await?;
        let count = records.len();

        let records = self.apply_return_fields(records, &stmt.modifiers);

        Ok(QueryData::Records { records, count })
    }

    /// Execute SCAN statement (WORKING only)
    async fn execute_scan(
        &self,
        stmt: &aql_parser::ScanStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
    ) -> ClawResult<QueryData> {
        let backend = backends
            .get(&MemoryType::Working)
            .ok_or_else(|| ClawError::InvalidMemoryType("WORKING".into()))?;

        let mut modifiers = stmt.modifiers.clone();

        // Apply WINDOW modifier
        if let Some(window) = &stmt.window {
            modifiers.window = Some(window.clone());
        }

        let mut records = backend.scan(&modifiers).await?;

        // Handle WINDOW TOP BY
        if let Some(aql_parser::Window::TopBy { count, field }) = &modifiers.window {
            records.sort_by(|a, b| {
                let va = a.resolve_field(field);
                let vb = b.resolve_field(field);
                compare_json_values(&vb, &va) // DESC order for TOP
            });
            records.truncate(*count);
        }

        let count = records.len();
        let records = self.apply_return_fields(records, &stmt.modifiers);

        Ok(QueryData::Records { records, count })
    }

    /// Execute FORGET statement
    async fn execute_forget(
        &self,
        stmt: &aql_parser::ForgetStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // Handle FROM ALL
        if matches!(stmt.memory_type, AqlMemoryType::All) {
            let conditions = self.bind_conditions(&stmt.conditions, variables);
            let mut total_count = 0;
            for backend in backends.values() {
                if let Ok(count) = backend.delete(&conditions).await {
                    total_count += count;
                }
            }
            return Ok(QueryData::Affected { count: total_count });
        }

        let memory_type: MemoryType = stmt.memory_type.into();
        let backend = backends
            .get(&memory_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))?;

        let conditions = self.bind_conditions(&stmt.conditions, variables);
        let count = backend.delete(&conditions).await?;

        Ok(QueryData::Affected { count })
    }

    /// Execute UPDATE statement
    async fn execute_update(
        &self,
        stmt: &aql_parser::UpdateStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let memory_type: MemoryType = stmt.memory_type.into();
        let backend = backends
            .get(&memory_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))?;

        // Build updates from payload
        let mut updates = serde_json::Map::new();
        for assignment in &stmt.payload {
            let value = self.resolve_value(&assignment.value, variables);
            updates.insert(assignment.field.clone(), value);
        }

        let conditions = self.bind_conditions(&stmt.conditions, variables);
        let count = backend.update(&conditions, Value::Object(updates)).await?;

        Ok(QueryData::Affected { count })
    }

    /// Execute LINK statement
    async fn execute_link(
        &self,
        stmt: &aql_parser::LinkStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // Validation: Cannot LINK FROM ALL
        if matches!(stmt.from_type, AqlMemoryType::All) {
            return Err(ClawError::InvalidQuery("Cannot LINK FROM ALL".into()));
        }

        let source_type: MemoryType = stmt.from_type.into();
        let target_type: MemoryType = stmt.to_type.into();

        let source_backend = backends
            .get(&source_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(source_type.to_string()))?;
        let target_backend = backends
            .get(&target_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(target_type.to_string()))?;

        // Find source records
        let source_conditions = self.bind_conditions(&stmt.from_conditions, variables);
        let source_records = source_backend.query(&source_conditions, &Modifiers::default()).await?;

        if source_records.is_empty() {
            return Err(ClawError::InvalidQuery("No source records found for LINK".into()));
        }

        // Find target records
        let target_conditions = self.bind_conditions(&stmt.to_conditions, variables);
        let target_records = target_backend.query(&target_conditions, &Modifiers::default()).await?;

        if target_records.is_empty() {
            return Err(ClawError::InvalidQuery("No target records found for LINK".into()));
        }

        // Store links
        let mut links_guard = links.write().await;
        let mut link_count = 0;
        for source in &source_records {
            for target in &target_records {
                links_guard.push(Link {
                    source_id: source.id.clone(),
                    source_type,
                    target_id: target.id.clone(),
                    target_type,
                    link_type: stmt.link_type.clone(),
                    weight: stmt.weight,
                });
                link_count += 1;
            }
        }

        Ok(QueryData::Message {
            message: format!(
                "Created {} links of type '{}' between {} source and {} target records",
                link_count,
                stmt.link_type,
                source_records.len(),
                target_records.len()
            ),
        })
    }

    /// Execute REFLECT statement
    async fn execute_reflect(
        &self,
        stmt: &aql_parser::ReflectStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let mut all_records = Vec::new();

        // Check if any source is ALL
        let from_all = stmt.sources.iter().any(|s| matches!(s.memory_type, AqlMemoryType::All));

        // Handle FROM ALL
        if from_all {
            for backend in backends.values() {
                if let Ok(records) = backend.query(&[], &Modifiers::default()).await {
                    all_records.extend(records);
                }
            }
        } else {
            // Query each source
            for source in &stmt.sources {
                let memory_type: MemoryType = source.memory_type.into();
                if let Some(backend) = backends.get(&memory_type) {
                    let conditions = if let Some(pred) = &source.predicate {
                        self.extract_conditions(pred, variables)?
                    } else {
                        vec![]
                    };
                    if let Ok(records) = backend.query(&conditions, &Modifiers::default()).await {
                        all_records.extend(records);
                    }
                }
            }
        }

        let count = all_records.len();

        // Handle THEN clause (use Box::pin to avoid async recursion issue)
        if let Some(then_stmt) = &stmt.then_clause {
            // Execute the THEN statement with context
            let _then_result = Box::pin(self.execute_statement(then_stmt, backends, links, variables)).await?;
        }

        Ok(QueryData::Records { records: all_records, count })
    }

    /// Execute PIPELINE statement
    async fn execute_pipeline(
        &self,
        stmt: &aql_parser::PipelineStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        links: &Arc<RwLock<LinksStorage>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        // Validate: TIMEOUT is required
        if stmt.timeout.is_none() {
            return Err(ClawError::InvalidQuery("PIPELINE requires TIMEOUT".into()));
        }

        let timeout = stmt.timeout.unwrap();
        let timeout_ms = timeout.as_millis() as u64;
        let start = Instant::now();
        let mut stage_results = Vec::new();
        let mut current_vars = variables.clone();

        for (i, stage) in stmt.stages.iter().enumerate() {
            // Check timeout
            let elapsed = start.elapsed().as_millis() as u64;
            if elapsed > timeout_ms {
                return Ok(QueryData::Message {
                    message: format!(
                        "Pipeline '{}' timed out after {}ms at stage {}",
                        stmt.name, elapsed, i + 1
                    ),
                });
            }

            // Execute stage (use Box::pin to avoid async recursion issue)
            let result = Box::pin(self.execute_statement(stage, backends, links, &current_vars)).await;

            match result {
                Ok(data) => {
                    // Extract values for variable binding
                    if let QueryData::Records { records, .. } = &data {
                        if let Some(record) = records.first() {
                            if let Value::Object(obj) = &record.data {
                                for (k, v) in obj {
                                    current_vars.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                    stage_results.push(json!({
                        "stage": i + 1,
                        "success": true,
                        "data": serde_json::to_value(&data).unwrap_or(Value::Null)
                    }));
                }
                Err(e) => {
                    stage_results.push(json!({
                        "stage": i + 1,
                        "success": false,
                        "error": e.to_string()
                    }));
                    return Ok(QueryData::Message {
                        message: format!(
                            "Pipeline '{}' failed at stage {}: {}",
                            stmt.name, i + 1, e
                        ),
                    });
                }
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok(QueryData::Message {
            message: format!(
                "Pipeline '{}' completed {} stages in {}ms",
                stmt.name,
                stmt.stages.len(),
                elapsed
            ),
        })
    }

    /// Extract conditions from a predicate
    fn extract_conditions(
        &self,
        predicate: &Predicate,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<Vec<aql_parser::Condition>> {
        match predicate {
            Predicate::Where { conditions } => Ok(self.bind_conditions(conditions, variables)),
            Predicate::Key { field, value } => {
                Ok(vec![aql_parser::Condition::Simple {
                    field: field.clone(),
                    operator: aql_parser::Operator::Eq,
                    value: value.clone(),
                    logical_op: None,
                }])
            }
            Predicate::All => Ok(vec![]),
            Predicate::Like { .. } => {
                Err(ClawError::InvalidQuery("LIKE requires semantic memory with embeddings".into()))
            }
            Predicate::Pattern { .. } => {
                Err(ClawError::InvalidQuery("PATTERN matching not yet supported".into()))
            }
        }
    }

    /// Bind variables in conditions
    fn bind_conditions(
        &self,
        conditions: &[aql_parser::Condition],
        variables: &HashMap<String, Value>,
    ) -> Vec<aql_parser::Condition> {
        conditions
            .iter()
            .map(|c| self.bind_condition(c, variables))
            .collect()
    }

    /// Bind variables in a single condition
    fn bind_condition(
        &self,
        condition: &aql_parser::Condition,
        variables: &HashMap<String, Value>,
    ) -> aql_parser::Condition {
        match condition {
            aql_parser::Condition::Simple { field, operator, value, logical_op } => {
                let bound_value = self.bind_value(value, variables);
                aql_parser::Condition::Simple {
                    field: field.clone(),
                    operator: *operator,
                    value: bound_value,
                    logical_op: *logical_op,
                }
            }
            aql_parser::Condition::Group { conditions, logical_op } => {
                aql_parser::Condition::Group {
                    conditions: self.bind_conditions(conditions, variables),
                    logical_op: *logical_op,
                }
            }
        }
    }

    /// Bind a variable in a value
    fn bind_value(
        &self,
        value: &aql_parser::Value,
        variables: &HashMap<String, Value>,
    ) -> aql_parser::Value {
        match value {
            aql_parser::Value::Variable(name) => {
                if let Some(v) = variables.get(name) {
                    json_to_aql_value(v)
                } else {
                    value.clone()
                }
            }
            aql_parser::Value::Array(arr) => {
                aql_parser::Value::Array(
                    arr.iter().map(|v| self.bind_value(v, variables)).collect()
                )
            }
            _ => value.clone(),
        }
    }

    /// Resolve a value (including variable substitution)
    fn resolve_value(&self, value: &aql_parser::Value, variables: &HashMap<String, Value>) -> Value {
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
            aql_parser::Value::Variable(name) => {
                variables.get(name).cloned().unwrap_or(Value::Null)
            }
            aql_parser::Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.resolve_value(v, variables)).collect())
            }
        }
    }

    /// Apply RETURN field projection to records
    fn apply_return_fields(
        &self,
        records: Vec<MemoryRecord>,
        modifiers: &Modifiers,
    ) -> Vec<MemoryRecord> {
        // If no RETURN fields specified or "*" is in the list, return full records
        let return_fields = match &modifiers.return_fields {
            Some(fields) => {
                if fields.iter().any(|f| f == "*") {
                    return records;
                }
                fields
            }
            None => return records,
        };

        // Project each record to only include specified fields
        records.into_iter().map(|mut record| {
            let mut projected_data = serde_json::Map::new();

            for field in return_fields {
                if field.starts_with("metadata.") {
                    // Handle metadata fields
                    let meta_field = &field[9..]; // Remove "metadata." prefix
                    let value = match meta_field {
                        "namespace" => Value::String(record.metadata.namespace.clone()),
                        "scope" => Value::String(record.metadata.scope.clone()),
                        "version" => Value::Number(record.metadata.version.into()),
                        "created_at" => Value::String(record.metadata.created_at.to_rfc3339()),
                        "accessed_at" => Value::String(record.metadata.accessed_at.to_rfc3339()),
                        _ => Value::Null,
                    };
                    // Store with original dotted key name
                    projected_data.insert(field.clone(), value);
                } else {
                    // Handle regular data fields
                    if let Some(value) = record.resolve_field(field) {
                        projected_data.insert(field.clone(), value);
                    }
                }
            }

            record.data = Value::Object(projected_data);
            record
        }).collect()
    }
}

/// Convert JSON value to AQL value
fn json_to_aql_value(v: &Value) -> aql_parser::Value {
    match v {
        Value::Null => aql_parser::Value::Null,
        Value::Bool(b) => aql_parser::Value::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                aql_parser::Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                aql_parser::Value::Float(f)
            } else {
                aql_parser::Value::Null
            }
        }
        Value::String(s) => aql_parser::Value::String(s.clone()),
        Value::Array(arr) => {
            aql_parser::Value::Array(arr.iter().map(json_to_aql_value).collect())
        }
        Value::Object(_) => aql_parser::Value::Null, // Objects not directly supported
    }
}

/// Compare two optional JSON values for sorting
fn compare_json_values(a: &Option<Value>, b: &Option<Value>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(va), Some(vb)) => {
            match (va, vb) {
                (Value::Number(na), Value::Number(nb)) => {
                    let fa = na.as_f64().unwrap_or(0.0);
                    let fb = nb.as_f64().unwrap_or(0.0);
                    fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
                }
                (Value::String(sa), Value::String(sb)) => sa.cmp(sb),
                _ => std::cmp::Ordering::Equal
            }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
