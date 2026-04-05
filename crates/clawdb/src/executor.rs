//! AQL Statement Executor for ClawDB

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use aql_parser::{self, Modifiers, Predicate, Statement};
use serde_json::Value;

use crate::backends::{LanceBackend, MemoryBackend};
use crate::config::Config;
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
    ) -> ClawResult<QueryResult> {
        self.execute_with_vars(aql, backends, HashMap::new()).await
    }

    /// Execute an AQL query with variables
    pub async fn execute_with_vars(
        &self,
        aql: &str,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: HashMap<String, Value>,
    ) -> ClawResult<QueryResult> {
        let start = Instant::now();

        // Parse the query
        let stmt = aql_parser::parse(aql)?;

        // Execute the statement
        let result = self.execute_statement(&stmt, backends, &variables).await;

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
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        match stmt {
            Statement::Store(s) => self.execute_store(s, backends, variables).await,
            Statement::Recall(r) => self.execute_recall(r, backends, variables).await,
            Statement::Lookup(l) => self.execute_lookup(l, backends, variables).await,
            Statement::Load(l) => self.execute_load(l, backends, variables).await,
            Statement::Scan(s) => self.execute_scan(s, backends).await,
            Statement::Forget(f) => self.execute_forget(f, backends, variables).await,
            Statement::Update(u) => self.execute_update(u, backends, variables).await,
            Statement::Link(_) => Err(ClawError::InvalidQuery("LINK not yet supported".into())),
            Statement::Reflect(_) => Err(ClawError::InvalidQuery("REFLECT not yet supported".into())),
            Statement::Pipeline(_) => Err(ClawError::InvalidQuery("PIPELINE not yet supported".into())),
        }
    }

    /// Execute STORE statement
    async fn execute_store(
        &self,
        stmt: &aql_parser::StoreStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
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
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
        let memory_type: MemoryType = stmt.memory_type.into();
        let backend = backends
            .get(&memory_type)
            .ok_or_else(|| ClawError::InvalidMemoryType(memory_type.to_string()))?;

        let conditions = self.extract_conditions(&stmt.predicate, variables)?;
        let records = backend.query(&conditions, &stmt.modifiers).await?;
        let count = records.len();

        // Apply RETURN field projection
        let records = self.apply_return_fields(records, &stmt.modifiers);

        Ok(QueryData::Records { records, count })
    }

    /// Execute LOOKUP statement
    async fn execute_lookup(
        &self,
        stmt: &aql_parser::LookupStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
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

    /// Execute LOAD statement (TOOLS only)
    async fn execute_load(
        &self,
        stmt: &aql_parser::LoadStmt,
        backends: &HashMap<MemoryType, Arc<LanceBackend>>,
        variables: &HashMap<String, Value>,
    ) -> ClawResult<QueryData> {
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

        let records = backend.scan(&modifiers).await?;
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
        // If no RETURN fields specified, return full records
        if modifiers.return_fields.is_none() {
            return records;
        }

        // TODO: Implement field projection
        // For now, return full records
        records
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
