//! LanceDB backend implementation for all memory types

use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use async_trait::async_trait;
use chrono::Utc;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{Connection, Table};
use serde_json::Value;

use super::MemoryBackend;
use crate::error::{ClawError, ClawResult};
use crate::types::{MemoryRecord, MemoryType, Metadata};
use aql_parser::{Condition, LogicalOp, Modifiers, Operator};

/// LanceDB backend for a specific memory type
pub struct LanceBackend {
    table: Table,
    memory_type: MemoryType,
}

impl LanceBackend {
    /// Create or open a LanceDB table for the given memory type
    pub async fn open(conn: &Connection, memory_type: MemoryType) -> ClawResult<Self> {
        let table_name = memory_type.table_name();
        let schema = Self::create_schema(memory_type);

        // Try to open existing table, or create new one
        let table = match conn.open_table(table_name).execute().await {
            Ok(t) => t,
            Err(_) => {
                // Create empty table with schema
                let batch = RecordBatch::new_empty(Arc::new(schema.clone()));
                let batches = RecordBatchIterator::new(vec![Ok(batch)], Arc::new(schema));
                conn.create_table(table_name, Box::new(batches))
                    .execute()
                    .await
                    .map_err(|e| ClawError::LanceDb(e.to_string()))?
            }
        };

        Ok(Self { table, memory_type })
    }

    /// Create the Arrow schema for a memory type
    fn create_schema(memory_type: MemoryType) -> Schema {
        let mut fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("data", DataType::Utf8, false), // JSON string
            Field::new("namespace", DataType::Utf8, false),
            Field::new("scope", DataType::Utf8, false),
            Field::new("created_at", DataType::Int64, false),
            Field::new("accessed_at", DataType::Int64, false),
            Field::new("version", DataType::Int64, false),
            Field::new("ttl_ms", DataType::Int64, true),
        ];

        // Add embedding field for semantic memory
        if memory_type == MemoryType::Semantic {
            fields.push(Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    384, // Default embedding dimension
                ),
                true,
            ));
        }

        Schema::new(fields)
    }

    /// Convert a MemoryRecord to a RecordBatch for insertion
    fn record_to_batch(&self, record: &MemoryRecord) -> ClawResult<RecordBatch> {
        let schema = Self::create_schema(self.memory_type);

        let id_array: ArrayRef = Arc::new(StringArray::from(vec![record.id.as_str()]));
        let data_array: ArrayRef = Arc::new(StringArray::from(vec![
            serde_json::to_string(&record.data)?,
        ]));
        let namespace_array: ArrayRef =
            Arc::new(StringArray::from(vec![record.metadata.namespace.as_str()]));
        let scope_array: ArrayRef =
            Arc::new(StringArray::from(vec![record.metadata.scope.as_str()]));
        let created_at_array: ArrayRef = Arc::new(Int64Array::from(vec![
            record.metadata.created_at.timestamp_millis(),
        ]));
        let accessed_at_array: ArrayRef = Arc::new(Int64Array::from(vec![
            record.metadata.accessed_at.timestamp_millis(),
        ]));
        let version_array: ArrayRef = Arc::new(Int64Array::from(vec![record.metadata.version]));
        let ttl_array: ArrayRef = Arc::new(Int64Array::from(vec![record.metadata.ttl_ms]));

        let mut columns: Vec<ArrayRef> = vec![
            id_array,
            data_array,
            namespace_array,
            scope_array,
            created_at_array,
            accessed_at_array,
            version_array,
            ttl_array,
        ];

        // Add embedding for semantic memory
        if self.memory_type == MemoryType::Semantic {
            // For now, use a placeholder - embeddings would come from external model
            let embedding_array: ArrayRef = Arc::new(
                arrow_array::FixedSizeListArray::from_iter_primitive::<
                    arrow_array::types::Float32Type,
                    _,
                    _,
                >(vec![Some(vec![Some(0.0f32); 384])], 384),
            );
            columns.push(embedding_array);
        }

        Ok(RecordBatch::try_new(Arc::new(schema), columns)?)
    }

    /// Convert a RecordBatch row to a MemoryRecord
    fn batch_to_record(&self, batch: &RecordBatch, row: usize) -> ClawResult<MemoryRecord> {
        let id = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .ok_or_else(|| ClawError::Other("Missing id column".into()))?;

        let data_str = batch
            .column_by_name("data")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row))
            .ok_or_else(|| ClawError::Other("Missing data column".into()))?;

        let data: Value = serde_json::from_str(data_str)?;

        let namespace = batch
            .column_by_name("namespace")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_else(|| "default".to_string());

        let scope = batch
            .column_by_name("scope")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_else(|| "private".to_string());

        let created_at_ms = batch
            .column_by_name("created_at")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .map(|a| a.value(row))
            .unwrap_or(0);

        let accessed_at_ms = batch
            .column_by_name("accessed_at")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .map(|a| a.value(row))
            .unwrap_or(0);

        let version = batch
            .column_by_name("version")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .map(|a| a.value(row))
            .unwrap_or(1);

        let ttl_ms = batch
            .column_by_name("ttl_ms")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .and_then(|a| {
                if a.is_null(row) {
                    None
                } else {
                    Some(a.value(row))
                }
            });

        let created_at =
            chrono::DateTime::from_timestamp_millis(created_at_ms).unwrap_or_else(Utc::now);
        let accessed_at =
            chrono::DateTime::from_timestamp_millis(accessed_at_ms).unwrap_or_else(Utc::now);

        Ok(MemoryRecord {
            id,
            memory_type: self.memory_type,
            data,
            metadata: Metadata {
                namespace,
                scope,
                created_at,
                accessed_at,
                version,
                ttl_ms,
            },
            embedding: None, // TODO: Extract embedding for semantic
        })
    }

    /// Build a LanceDB filter string from AQL conditions
    fn build_filter(&self, conditions: &[Condition]) -> Option<String> {
        if conditions.is_empty() {
            return None;
        }

        let mut filter_parts = Vec::new();

        for (i, condition) in conditions.iter().enumerate() {
            let part = self.condition_to_filter(condition)?;

            if i == 0 {
                filter_parts.push(part);
            } else {
                let op = match condition.logical_op() {
                    Some(LogicalOp::Or) => "OR",
                    Some(LogicalOp::And) | None => "AND",
                };
                filter_parts.push(format!("{} {}", op, part));
            }
        }

        Some(filter_parts.join(" "))
    }

    /// Convert a single condition to a filter string
    fn condition_to_filter(&self, condition: &Condition) -> Option<String> {
        match condition {
            Condition::Simple {
                field,
                operator,
                value,
                ..
            } => {
                // Map field paths to LanceDB columns
                let column = self.map_field_to_column(field);
                let op_str = match operator {
                    Operator::Eq => "=",
                    Operator::Ne => "!=",
                    Operator::Gt => ">",
                    Operator::Gte => ">=",
                    Operator::Lt => "<",
                    Operator::Lte => "<=",
                    Operator::Contains => {
                        return Some(format!(
                            "{} LIKE '%{}%'",
                            column,
                            value_to_string(value)
                        ))
                    }
                    Operator::StartsWith => {
                        return Some(format!("{} LIKE '{}%'", column, value_to_string(value)))
                    }
                    Operator::EndsWith => {
                        return Some(format!("{} LIKE '%{}'", column, value_to_string(value)))
                    }
                    Operator::In => {
                        // IN operator needs special handling
                        if let aql_parser::Value::Array(arr) = value {
                            let values: Vec<String> = arr.iter().map(value_to_sql).collect();
                            return Some(format!("{} IN ({})", column, values.join(", ")));
                        }
                        return None;
                    }
                };

                Some(format!("{} {} {}", column, op_str, value_to_sql(value)))
            }
            Condition::Group { conditions, .. } => {
                let inner = self.build_filter(conditions)?;
                Some(format!("({})", inner))
            }
        }
    }

    /// Map AQL field paths to LanceDB column names
    fn map_field_to_column(&self, field: &str) -> String {
        match field {
            "id" => "id".to_string(),
            "namespace" | "metadata.namespace" => "namespace".to_string(),
            "scope" | "metadata.scope" => "scope".to_string(),
            "created_at" | "metadata.created_at" => "created_at".to_string(),
            "accessed_at" | "metadata.accessed_at" => "accessed_at".to_string(),
            // For data fields, we need to use JSON extraction
            // LanceDB doesn't support JSON path queries directly,
            // so we'll filter in Rust after fetching
            _ => field.to_string(),
        }
    }

    /// Check if a record matches the given conditions (for JSON field filtering)
    fn record_matches_conditions(&self, record: &MemoryRecord, conditions: &[Condition]) -> bool {
        if conditions.is_empty() {
            return true;
        }

        let mut result = self.evaluate_condition(record, &conditions[0]);

        for condition in &conditions[1..] {
            let cond_result = self.evaluate_condition(record, condition);
            match condition.logical_op() {
                Some(LogicalOp::Or) => result = result || cond_result,
                Some(LogicalOp::And) | None => result = result && cond_result,
            }
        }

        result
    }

    /// Evaluate a single condition against a record
    fn evaluate_condition(&self, record: &MemoryRecord, condition: &Condition) -> bool {
        match condition {
            Condition::Simple {
                field,
                operator,
                value,
                ..
            } => {
                let field_value = record.resolve_field(field);
                match field_value {
                    Some(fv) => compare_values(&fv, operator, value),
                    None => false,
                }
            }
            Condition::Group { conditions, .. } => {
                self.record_matches_conditions(record, conditions)
            }
        }
    }

    /// Build a simple filter for columns that LanceDB can handle directly
    fn build_simple_filter(&self, conditions: &[Condition]) -> Option<String> {
        let simple_fields = ["id", "namespace", "scope", "created_at", "accessed_at"];

        let mut parts = Vec::new();

        for condition in conditions {
            if let Condition::Simple { field, .. } = condition {
                // Only include conditions on simple fields
                let base_field = if field.starts_with("metadata.") {
                    &field[9..]
                } else {
                    field.as_str()
                };

                if simple_fields.contains(&base_field) {
                    if let Some(filter) = self.condition_to_filter(condition) {
                        parts.push(filter);
                    }
                }
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" AND "))
        }
    }
}

#[async_trait]
impl MemoryBackend for LanceBackend {
    async fn store(&self, record: MemoryRecord) -> ClawResult<MemoryRecord> {
        let batch = self.record_to_batch(&record)?;
        let batches = RecordBatchIterator::new(vec![Ok(batch.clone())], batch.schema());

        self.table
            .add(Box::new(batches))
            .execute()
            .await
            .map_err(|e| ClawError::LanceDb(e.to_string()))?;

        Ok(record)
    }

    async fn query(
        &self,
        conditions: &[Condition],
        modifiers: &Modifiers,
    ) -> ClawResult<Vec<MemoryRecord>> {
        // Build filter for simple column conditions
        let simple_filter = self.build_simple_filter(conditions);

        // Apply limit (fetch more to account for post-filtering)
        // If ORDER BY is specified, we need all records before sorting, so use larger limit
        // If there are conditions that require JSON filtering, also fetch more
        let has_json_conditions = conditions.iter().any(|c| {
            if let aql_parser::Condition::Simple { field, .. } = c {
                !["id", "namespace", "scope", "created_at", "accessed_at"].contains(&field.as_str())
            } else {
                true
            }
        });
        let fetch_limit = if modifiers.order_by.is_some() || has_json_conditions {
            // Need all matching records for ORDER BY or JSON filtering
            modifiers.limit.map(|l| l.max(100) * 10).unwrap_or(1000)
        } else {
            modifiers.limit.map(|l| l * 2).unwrap_or(1000)
        };

        // Build and execute query using proper LanceDB API
        let batches: Vec<RecordBatch> = if let Some(filter) = simple_filter {
            self.table
                .query()
                .only_if(filter)
                .limit(fetch_limit)
                .execute()
                .await
                .map_err(|e| ClawError::LanceDb(format!("Failed to query: {}", e)))?
                .try_collect()
                .await
                .map_err(|e| ClawError::LanceDb(format!("Failed to collect results: {}", e)))?
        } else {
            self.table
                .query()
                .limit(fetch_limit)
                .execute()
                .await
                .map_err(|e| ClawError::LanceDb(format!("Failed to query: {}", e)))?
                .try_collect()
                .await
                .map_err(|e| ClawError::LanceDb(format!("Failed to collect results: {}", e)))?
        };

        let mut records = Vec::new();

        // Process batches
        for batch in batches {
            for row in 0..batch.num_rows() {
                if let Ok(record) = self.batch_to_record(&batch, row) {
                    // Check TTL expiry
                    if record.metadata.is_expired() {
                        continue;
                    }
                    // Apply SCOPE filtering from modifiers
                    if let Some(scope) = &modifiers.scope {
                        let scope_str = format!("{:?}", scope).to_lowercase();
                        if record.metadata.scope != scope_str {
                            continue;
                        }
                    }
                    // Apply NAMESPACE filtering from modifiers
                    if let Some(namespace) = &modifiers.namespace {
                        if record.metadata.namespace != *namespace {
                            continue;
                        }
                    }
                    // Apply full condition filtering (for JSON fields)
                    if self.record_matches_conditions(&record, conditions) {
                        records.push(record);
                    }
                }
            }
        }

        // Apply ORDER BY
        if let Some(order) = &modifiers.order_by {
            records.sort_by(|a, b| {
                let va = a.resolve_field(&order.field);
                let vb = b.resolve_field(&order.field);
                let cmp = compare_json_values(&va, &vb);
                if order.ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }

        // Apply LIMIT
        if let Some(limit) = modifiers.limit {
            records.truncate(limit);
        }

        Ok(records)
    }

    async fn delete(&self, conditions: &[Condition]) -> ClawResult<usize> {
        // First, query to find matching records
        let records = self.query(conditions, &Modifiers::default()).await?;
        let ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();

        if ids.is_empty() {
            return Ok(0);
        }

        // Delete by ID
        let id_list: Vec<String> = ids.iter().map(|id| format!("'{}'", id)).collect();
        let filter = format!("id IN ({})", id_list.join(", "));

        self.table
            .delete(&filter)
            .await
            .map_err(|e| ClawError::LanceDb(format!("Failed to delete: {}", e)))?;

        Ok(ids.len())
    }

    async fn update(&self, conditions: &[Condition], updates: Value) -> ClawResult<usize> {
        // Query matching records
        let records = self.query(conditions, &Modifiers::default()).await?;
        let count = records.len();

        if count == 0 {
            return Ok(0);
        }

        // For each record, update and re-insert
        // (LanceDB doesn't have native update, so we delete and re-add)
        for mut record in records {
            // Merge updates into data
            if let Value::Object(update_map) = &updates {
                if let Value::Object(ref mut data_map) = record.data {
                    for (k, v) in update_map {
                        data_map.insert(k.clone(), v.clone());
                    }
                }
            }

            // Update metadata
            record.metadata.accessed_at = Utc::now();
            record.metadata.version += 1;

            // Delete old record
            let filter = format!("id = '{}'", record.id);
            self.table
                .delete(&filter)
                .await
                .map_err(|e| ClawError::LanceDb(format!("Failed to delete for update: {}", e)))?;

            // Insert updated record
            let batch = self.record_to_batch(&record)?;
            let batches = RecordBatchIterator::new(vec![Ok(batch.clone())], batch.schema());
            self.table
                .add(Box::new(batches))
                .execute()
                .await
                .map_err(|e| ClawError::LanceDb(format!("Failed to add updated record: {}", e)))?;
        }

        Ok(count)
    }

    async fn scan(&self, modifiers: &Modifiers) -> ClawResult<Vec<MemoryRecord>> {
        // Scan is query with no conditions
        self.query(&[], modifiers).await
    }

    async fn get_by_id(&self, id: &str) -> ClawResult<Option<MemoryRecord>> {
        let filter = format!("id = '{}'", id);

        let batches: Vec<RecordBatch> = self
            .table
            .query()
            .only_if(filter)
            .limit(1)
            .execute()
            .await
            .map_err(|e| ClawError::LanceDb(format!("Failed to query by id: {}", e)))?
            .try_collect()
            .await
            .map_err(|e| ClawError::LanceDb(format!("Failed to collect results: {}", e)))?;

        if batches.is_empty() || batches[0].num_rows() == 0 {
            return Ok(None);
        }

        Ok(Some(self.batch_to_record(&batches[0], 0)?))
    }

    fn memory_type(&self) -> MemoryType {
        self.memory_type
    }
}

/// Convert AQL value to SQL literal string
fn value_to_sql(value: &aql_parser::Value) -> String {
    match value {
        aql_parser::Value::Null => "NULL".to_string(),
        aql_parser::Value::Bool(b) => {
            if *b {
                "TRUE"
            } else {
                "FALSE"
            }
            .to_string()
        }
        aql_parser::Value::Int(i) => i.to_string(),
        aql_parser::Value::Float(f) => f.to_string(),
        aql_parser::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        aql_parser::Value::Variable(v) => format!("${}", v),
        aql_parser::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_sql).collect();
            format!("[{}]", items.join(", "))
        }
    }
}

/// Convert AQL value to string (for LIKE patterns)
fn value_to_string(value: &aql_parser::Value) -> String {
    match value {
        aql_parser::Value::String(s) => s.clone(),
        _ => format!("{:?}", value),
    }
}

/// Compare a JSON value against an AQL value using an operator
fn compare_values(json_val: &Value, operator: &Operator, aql_val: &aql_parser::Value) -> bool {
    let target = aql_value_to_json(aql_val);

    match operator {
        Operator::Eq => json_equals(json_val, &target),
        Operator::Ne => !json_equals(json_val, &target),
        Operator::Gt => json_compare(json_val, &target).map_or(false, |c| c > 0),
        Operator::Gte => json_compare(json_val, &target).map_or(false, |c| c >= 0),
        Operator::Lt => json_compare(json_val, &target).map_or(false, |c| c < 0),
        Operator::Lte => json_compare(json_val, &target).map_or(false, |c| c <= 0),
        Operator::Contains => match (json_val, &target) {
            (Value::String(s), Value::String(t)) => s.contains(t.as_str()),
            (Value::Array(arr), _) => arr.contains(&target),
            _ => false,
        },
        Operator::StartsWith => match (json_val, &target) {
            (Value::String(s), Value::String(t)) => s.starts_with(t.as_str()),
            _ => false,
        },
        Operator::EndsWith => match (json_val, &target) {
            (Value::String(s), Value::String(t)) => s.ends_with(t.as_str()),
            _ => false,
        },
        Operator::In => {
            if let Value::Array(arr) = &target {
                arr.iter().any(|v| json_equals(json_val, v))
            } else {
                false
            }
        }
    }
}

/// Convert AQL value to JSON value
fn aql_value_to_json(value: &aql_parser::Value) -> Value {
    match value {
        aql_parser::Value::Null => Value::Null,
        aql_parser::Value::Bool(b) => Value::Bool(*b),
        aql_parser::Value::Int(i) => Value::Number((*i).into()),
        aql_parser::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        aql_parser::Value::String(s) => Value::String(s.clone()),
        aql_parser::Value::Variable(v) => Value::String(format!("${}", v)),
        aql_parser::Value::Array(arr) => Value::Array(arr.iter().map(aql_value_to_json).collect()),
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

/// Compare JSON values (returns ordering as i32)
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

/// Compare two optional JSON values for sorting
fn compare_json_values(a: &Option<Value>, b: &Option<Value>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(va), Some(vb)) => json_compare(va, vb)
            .map(|c| {
                if c < 0 {
                    std::cmp::Ordering::Less
                } else if c > 0 {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
