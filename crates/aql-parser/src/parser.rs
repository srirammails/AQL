//! AQL Parser Implementation
//!
//! Parses AQL query strings into AST using pest.

use pest::Parser;
use pest_derive::Parser;
use std::time::Duration;

use crate::ast::*;
use crate::error::{ParseError, ParseResult};

/// The pest parser for AQL
#[derive(Parser)]
#[grammar = "grammar/aql.pest"]
pub struct AqlParser;

/// Parse an AQL query string into a Statement
pub fn parse(input: &str) -> ParseResult<Statement> {
    let pairs = AqlParser::parse(Rule::aql, input).map_err(|e| ParseError::from_pest(e))?;

    // Get the statement from aql rule
    for pair in pairs {
        if pair.as_rule() == Rule::aql {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::statement {
                    return parse_statement(inner);
                }
            }
        }
    }

    Err(ParseError::Empty)
}

/// Parse a statement
fn parse_statement(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let inner = pair.into_inner().next().ok_or(ParseError::Empty)?;

    match inner.as_rule() {
        Rule::pipeline_stmt => parse_pipeline(inner),
        Rule::reflect_stmt => parse_reflect(inner),
        Rule::scan_stmt => parse_scan(inner),
        Rule::recall_stmt => parse_recall(inner),
        Rule::lookup_stmt => parse_lookup(inner),
        Rule::load_stmt => parse_load(inner),
        Rule::store_stmt => parse_store(inner),
        Rule::update_stmt => parse_update(inner),
        Rule::forget_stmt => parse_forget(inner),
        Rule::link_stmt => parse_link(inner),
        _ => Err(ParseError::UnexpectedRule(format!("{:?}", inner.as_rule()))),
    }
}

/// Parse PIPELINE statement
fn parse_pipeline(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut name = String::from("_anonymous");
    let mut timeout = None;
    let mut stages = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::pipeline_name => {
                // Extract the identifier from pipeline_name
                if let Some(ident) = item.into_inner().find(|p| p.as_rule() == Rule::identifier) {
                    name = ident.as_str().to_string();
                }
            }
            Rule::timeout_mod => {
                timeout = Some(parse_duration_from_mod(item)?);
            }
            Rule::pipeline_stage => {
                let stage_inner = item.into_inner().next().ok_or(ParseError::Empty)?;
                stages.push(parse_statement_inner(stage_inner)?);
            }
            _ => {}
        }
    }

    Ok(Statement::Pipeline(PipelineStmt {
        name,
        timeout,
        stages,
    }))
}

/// Parse inner statement (for pipeline stages)
fn parse_statement_inner(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    match pair.as_rule() {
        Rule::scan_stmt => parse_scan(pair),
        Rule::recall_stmt => parse_recall(pair),
        Rule::lookup_stmt => parse_lookup(pair),
        Rule::load_stmt => parse_load(pair),
        Rule::reflect_stmt => parse_reflect(pair),
        _ => Err(ParseError::UnexpectedRule(format!("{:?}", pair.as_rule()))),
    }
}

/// Parse REFLECT statement
fn parse_reflect(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut sources = Vec::new();
    let mut with_links = None;
    let mut follow_links = None;
    let mut then_clause = None;

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::reflect_source => {
                sources.push(parse_reflect_source(item)?);
            }
            Rule::with_links_mod => {
                with_links = Some(parse_with_links(item)?);
            }
            Rule::follow_links_mod => {
                follow_links = Some(parse_follow_links(item)?);
            }
            Rule::then_clause => {
                let inner = item.into_inner().next().ok_or(ParseError::Empty)?;
                // then_clause contains store_stmt or update_stmt directly
                let stmt = match inner.as_rule() {
                    Rule::store_stmt => parse_store(inner)?,
                    Rule::update_stmt => parse_update(inner)?,
                    _ => return Err(ParseError::UnexpectedRule(format!("{:?}", inner.as_rule()))),
                };
                then_clause = Some(Box::new(stmt));
            }
            _ => {}
        }
    }

    Ok(Statement::Reflect(ReflectStmt {
        sources,
        with_links,
        follow_links,
        then_clause,
    }))
}

/// Parse reflect source
fn parse_reflect_source(pair: pest::iterators::Pair<Rule>) -> ParseResult<ReflectSource> {
    let mut inner = pair.into_inner();

    let memory_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("memory type"))?)?;
    let predicate = inner.next().map(parse_predicate).transpose()?;

    Ok(ReflectSource {
        memory_type,
        predicate,
    })
}

/// Parse SCAN statement
fn parse_scan(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut window = None;
    let mut modifiers = Modifiers::default();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::window_mod => {
                window = Some(parse_window(item)?);
            }
            Rule::modifier => {
                parse_modifier_into(item, &mut modifiers)?;
            }
            _ => {}
        }
    }

    Ok(Statement::Scan(ScanStmt { window, modifiers }))
}

/// Parse RECALL statement
fn parse_recall(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let memory_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("memory type"))?)?;

    // Predicate is optional - defaults to All if not specified
    let mut predicate = Predicate::All;
    let mut modifiers = Modifiers::default();

    for item in inner {
        match item.as_rule() {
            Rule::predicate => {
                predicate = parse_predicate(item)?;
            }
            Rule::modifier => {
                parse_modifier_into(item, &mut modifiers)?;
            }
            _ => {}
        }
    }

    Ok(Statement::Recall(RecallStmt {
        memory_type,
        predicate,
        modifiers,
    }))
}

/// Parse LOOKUP statement
fn parse_lookup(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let memory_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("memory type"))?)?;
    let predicate = parse_predicate(inner.next().ok_or(ParseError::Missing("predicate"))?)?;

    let mut modifiers = Modifiers::default();
    for item in inner {
        if item.as_rule() == Rule::modifier {
            parse_modifier_into(item, &mut modifiers)?;
        }
    }

    Ok(Statement::Lookup(LookupStmt {
        memory_type,
        predicate,
        modifiers,
    }))
}

/// Parse LOAD statement
fn parse_load(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let predicate = parse_predicate(inner.next().ok_or(ParseError::Missing("predicate"))?)?;

    let mut modifiers = Modifiers::default();
    for item in inner {
        if item.as_rule() == Rule::modifier {
            parse_modifier_into(item, &mut modifiers)?;
        }
    }

    Ok(Statement::Load(LoadStmt {
        predicate,
        modifiers,
    }))
}

/// Parse STORE statement
fn parse_store(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let memory_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("memory type"))?)?;
    let payload = parse_payload(inner.next().ok_or(ParseError::Missing("payload"))?)?;

    let mut modifiers = Modifiers::default();
    for item in inner {
        if item.as_rule() == Rule::modifier {
            parse_modifier_into(item, &mut modifiers)?;
        }
    }

    Ok(Statement::Store(StoreStmt {
        memory_type,
        payload,
        modifiers,
    }))
}

/// Parse UPDATE statement
fn parse_update(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let memory_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("memory type"))?)?;
    let conditions = parse_condition_list(inner.next().ok_or(ParseError::Missing("conditions"))?)?;
    let payload = parse_payload(inner.next().ok_or(ParseError::Missing("payload"))?)?;

    let mut modifiers = Modifiers::default();
    for item in inner {
        if item.as_rule() == Rule::modifier {
            parse_modifier_into(item, &mut modifiers)?;
        }
    }

    Ok(Statement::Update(UpdateStmt {
        memory_type,
        conditions,
        payload,
        modifiers,
    }))
}

/// Parse FORGET statement
fn parse_forget(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let memory_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("memory type"))?)?;
    let conditions = parse_condition_list(inner.next().ok_or(ParseError::Missing("conditions"))?)?;

    let mut modifiers = Modifiers::default();
    for item in inner {
        if item.as_rule() == Rule::modifier {
            parse_modifier_into(item, &mut modifiers)?;
        }
    }

    Ok(Statement::Forget(ForgetStmt {
        memory_type,
        conditions,
        modifiers,
    }))
}

/// Parse LINK statement
fn parse_link(pair: pest::iterators::Pair<Rule>) -> ParseResult<Statement> {
    let mut inner = pair.into_inner();

    let from_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("from type"))?)?;
    let from_conditions =
        parse_condition_list(inner.next().ok_or(ParseError::Missing("from conditions"))?)?;
    let to_type = parse_memory_type(inner.next().ok_or(ParseError::Missing("to type"))?)?;
    let to_conditions =
        parse_condition_list(inner.next().ok_or(ParseError::Missing("to conditions"))?)?;
    let link_type = parse_string_literal(inner.next().ok_or(ParseError::Missing("link type"))?)?;

    let weight = inner.next().map(|p| parse_float(p)).transpose()?;

    Ok(Statement::Link(LinkStmt {
        from_type,
        from_conditions,
        to_type,
        to_conditions,
        link_type,
        weight,
    }))
}

/// Parse memory type
fn parse_memory_type(pair: pest::iterators::Pair<Rule>) -> ParseResult<MemoryType> {
    MemoryType::from_str(pair.as_str())
        .ok_or_else(|| ParseError::InvalidMemoryType(pair.as_str().to_string()))
}

/// Parse predicate
fn parse_predicate(pair: pest::iterators::Pair<Rule>) -> ParseResult<Predicate> {
    let inner = pair.into_inner().next().ok_or(ParseError::Empty)?;

    match inner.as_rule() {
        Rule::where_pred => {
            let conditions = parse_condition_list(
                inner.into_inner().next().ok_or(ParseError::Empty)?,
            )?;
            Ok(Predicate::Where { conditions })
        }
        Rule::key_pred => {
            let mut parts = inner.into_inner();
            let field = parts
                .next()
                .ok_or(ParseError::Missing("field"))?
                .as_str()
                .to_string();
            let value = parse_value(parts.next().ok_or(ParseError::Missing("value"))?)?;
            Ok(Predicate::Key { field, value })
        }
        Rule::like_pred => {
            let var = inner
                .into_inner()
                .next()
                .ok_or(ParseError::Missing("variable"))?;
            let variable = parse_variable_name(var)?;
            Ok(Predicate::Like { variable })
        }
        Rule::pattern_pred => {
            let mut parts = inner.into_inner();
            let var = parts.next().ok_or(ParseError::Missing("variable"))?;
            let variable = parse_variable_name(var)?;
            let threshold = parts.next().map(|p| parse_float(p)).transpose()?;
            Ok(Predicate::Pattern { variable, threshold })
        }
        Rule::all_pred => Ok(Predicate::All),
        _ => Err(ParseError::UnexpectedRule(format!("{:?}", inner.as_rule()))),
    }
}

/// Parse condition list
fn parse_condition_list(pair: pest::iterators::Pair<Rule>) -> ParseResult<Vec<Condition>> {
    let mut conditions = Vec::new();
    let mut pending_logical_op: Option<LogicalOp> = None;

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::condition_atom => {
                let mut cond = parse_condition_atom(item)?;
                cond.set_logical_op(pending_logical_op.take());
                conditions.push(cond);
            }
            Rule::simple_condition => {
                let mut condition = parse_simple_condition(item)?;
                condition.set_logical_op(pending_logical_op.take());
                conditions.push(condition);
            }
            Rule::logical_op => {
                pending_logical_op = LogicalOp::from_str(item.as_str());
            }
            _ => {
                // Try to parse AND/OR keywords directly
                let s = item.as_str().to_uppercase();
                if s == "AND" || s == "OR" {
                    pending_logical_op = LogicalOp::from_str(&s);
                }
            }
        }
    }

    Ok(conditions)
}

/// Parse a condition atom (either a parenthesized group or a simple condition)
/// Returns a single Condition (either Simple or Group)
fn parse_condition_atom(pair: pest::iterators::Pair<Rule>) -> ParseResult<Condition> {
    let inner = pair.into_inner().next();
    match inner {
        Some(item) => match item.as_rule() {
            Rule::paren_condition => parse_paren_condition(item),
            Rule::simple_condition => parse_simple_condition(item),
            _ => Err(ParseError::UnexpectedRule(format!("in condition_atom: {:?}", item.as_rule()))),
        },
        None => Err(ParseError::Empty),
    }
}

/// Parse a parenthesized condition group - returns a Group condition
fn parse_paren_condition(pair: pest::iterators::Pair<Rule>) -> ParseResult<Condition> {
    // paren_condition = { "(" ~ condition_list ~ ")" }
    // Returns a Group condition containing the nested conditions
    for item in pair.into_inner() {
        if item.as_rule() == Rule::condition_list {
            let conditions = parse_condition_list(item)?;
            return Ok(Condition::Group {
                conditions,
                logical_op: None, // Will be set by caller
            });
        }
    }
    Ok(Condition::Group {
        conditions: vec![],
        logical_op: None,
    })
}

/// Parse a simple condition (field op value)
fn parse_simple_condition(pair: pest::iterators::Pair<Rule>) -> ParseResult<Condition> {
    let mut inner = pair.into_inner();

    let field = inner
        .next()
        .ok_or(ParseError::Missing("field"))?
        .as_str()
        .to_string();
    let operator = parse_operator(inner.next().ok_or(ParseError::Missing("operator"))?)?;
    let value = parse_value(inner.next().ok_or(ParseError::Missing("value"))?)?;

    Ok(Condition::Simple {
        field,
        operator,
        value,
        logical_op: None, // Will be set by parse_condition_list
    })
}

/// Parse operator
fn parse_operator(pair: pest::iterators::Pair<Rule>) -> ParseResult<Operator> {
    Operator::from_str(pair.as_str())
        .ok_or_else(|| ParseError::InvalidOperator(pair.as_str().to_string()))
}

/// Parse value
fn parse_value(pair: pest::iterators::Pair<Rule>) -> ParseResult<Value> {
    let inner = pair.into_inner().next().ok_or(ParseError::Empty)?;

    match inner.as_rule() {
        Rule::null_literal => Ok(Value::Null),
        Rule::bool_literal => {
            let b = inner.as_str().eq_ignore_ascii_case("true");
            Ok(Value::Bool(b))
        }
        Rule::float_literal => {
            let f: f64 = inner
                .as_str()
                .parse()
                .map_err(|_| ParseError::InvalidFloat(inner.as_str().to_string()))?;
            Ok(Value::Float(f))
        }
        Rule::integer_literal => {
            let i: i64 = inner
                .as_str()
                .parse()
                .map_err(|_| ParseError::InvalidInteger(inner.as_str().to_string()))?;
            Ok(Value::Int(i))
        }
        Rule::string_literal => Ok(Value::String(parse_string_literal(inner)?)),
        Rule::variable => Ok(Value::Variable(parse_variable_name(inner)?)),
        Rule::array_literal => {
            let values: ParseResult<Vec<Value>> = inner.into_inner().map(parse_value).collect();
            Ok(Value::Array(values?))
        }
        _ => Err(ParseError::UnexpectedRule(format!("{:?}", inner.as_rule()))),
    }
}

/// Parse payload
fn parse_payload(pair: pest::iterators::Pair<Rule>) -> ParseResult<Vec<FieldAssignment>> {
    pair.into_inner().map(parse_field_assignment).collect()
}

/// Parse field assignment
fn parse_field_assignment(pair: pest::iterators::Pair<Rule>) -> ParseResult<FieldAssignment> {
    let mut inner = pair.into_inner();

    let field = inner
        .next()
        .ok_or(ParseError::Missing("field"))?
        .as_str()
        .to_string();
    let value = parse_value(inner.next().ok_or(ParseError::Missing("value"))?)?;

    Ok(FieldAssignment { field, value })
}

/// Parse modifier into modifiers struct
fn parse_modifier_into(
    pair: pest::iterators::Pair<Rule>,
    mods: &mut Modifiers,
) -> ParseResult<()> {
    let inner = pair.into_inner().next().ok_or(ParseError::Empty)?;

    match inner.as_rule() {
        Rule::limit_mod => {
            let n = parse_integer(inner.into_inner().next().ok_or(ParseError::Empty)?)?;
            mods.limit = Some(n as usize);
        }
        Rule::order_mod => {
            let mut parts = inner.into_inner();
            let field = parts
                .next()
                .ok_or(ParseError::Missing("field"))?
                .as_str()
                .to_string();
            // order_direction rule captures ASC or DESC
            let ascending = parts
                .next()
                .map(|p| !p.as_str().eq_ignore_ascii_case("DESC"))
                .unwrap_or(true);
            mods.order_by = Some(OrderBy { field, ascending });
        }
        Rule::return_mod => {
            let fields: Vec<String> = inner
                .into_inner()
                .map(|p| p.as_str().to_string())
                .collect();
            mods.return_fields = Some(fields);
        }
        Rule::timeout_mod => {
            mods.timeout = Some(parse_duration_from_mod(inner)?);
        }
        Rule::min_confidence_mod => {
            let f = parse_float(inner.into_inner().next().ok_or(ParseError::Empty)?)?;
            mods.min_confidence = Some(f);
        }
        Rule::scope_mod => {
            // scope_value rule captures private, shared, or cluster
            let scope_pair = inner
                .into_inner()
                .next()
                .ok_or(ParseError::Missing("scope value"))?;
            let scope_str = scope_pair.as_str();
            let scope = match scope_str.to_lowercase().as_str() {
                "private" => Scope::Private,
                "shared" => Scope::Shared,
                "cluster" => Scope::Cluster,
                _ => return Err(ParseError::InvalidScope(scope_str.to_string())),
            };
            mods.scope = Some(scope);
        }
        Rule::namespace_mod => {
            let ns =
                parse_string_literal(inner.into_inner().next().ok_or(ParseError::Empty)?)?;
            mods.namespace = Some(ns);
        }
        Rule::ttl_mod => {
            mods.ttl = Some(parse_duration_from_mod(inner)?);
        }
        Rule::aggregate_mod => {
            let funcs: ParseResult<Vec<AggregateFunc>> =
                inner.into_inner().map(parse_aggregate_func).collect();
            mods.aggregate = Some(funcs?);
        }
        Rule::having_mod => {
            let conditions =
                parse_condition_list(inner.into_inner().next().ok_or(ParseError::Empty)?)?;
            mods.having = Some(conditions);
        }
        Rule::with_links_mod => {
            mods.with_links = Some(parse_with_links(inner)?);
        }
        Rule::follow_links_mod => {
            mods.follow_links = Some(parse_follow_links(inner)?);
        }
        Rule::window_mod => {
            mods.window = Some(parse_window(inner)?);
        }
        _ => {}
    }

    Ok(())
}

/// Parse aggregate function
fn parse_aggregate_func(pair: pest::iterators::Pair<Rule>) -> ParseResult<AggregateFunc> {
    let mut inner = pair.into_inner();

    let func_name = inner.next().ok_or(ParseError::Missing("function name"))?;
    let func = AggregateFuncType::from_str(func_name.as_str())
        .ok_or_else(|| ParseError::InvalidAggregateFunc(func_name.as_str().to_string()))?;

    // The next element could be field_path (for fields) or agg_alias (when using *)
    // because * is a literal that doesn't appear in the parse tree
    let mut field = None;
    let mut alias = None;

    for p in inner {
        match p.as_rule() {
            Rule::field_path => {
                field = Some(p.as_str().to_string());
            }
            Rule::agg_alias => {
                // agg_alias contains the identifier after AS
                alias = p.into_inner()
                    .find(|inner_p| inner_p.as_rule() == Rule::identifier)
                    .map(|inner_p| inner_p.as_str().to_string());
            }
            _ => {}
        }
    }

    Ok(AggregateFunc { func, field, alias })
}

/// Parse WITH LINKS modifier
fn parse_with_links(pair: pest::iterators::Pair<Rule>) -> ParseResult<WithLinks> {
    let inner = pair.into_inner().next();

    match inner {
        Some(p) if p.as_rule() == Rule::string_literal => {
            Ok(WithLinks::Type {
                link_type: parse_string_literal(p)?,
            })
        }
        _ => Ok(WithLinks::All),
    }
}

/// Parse FOLLOW LINKS modifier
fn parse_follow_links(pair: pest::iterators::Pair<Rule>) -> ParseResult<FollowLinks> {
    let mut inner = pair.into_inner();

    let link_type = parse_string_literal(inner.next().ok_or(ParseError::Missing("link type"))?)?;
    let depth = inner.next().map(|p| parse_integer(p)).transpose()?.map(|n| n as u32);

    Ok(FollowLinks { link_type, depth })
}

/// Parse WINDOW modifier
fn parse_window(pair: pest::iterators::Pair<Rule>) -> ParseResult<Window> {
    let mut inner = pair.into_inner();

    let first = inner.next().ok_or(ParseError::Empty)?;

    match first.as_rule() {
        Rule::duration_literal => {
            let duration = parse_duration(first)?;
            Ok(Window::LastDuration { duration })
        }
        Rule::integer_literal => {
            let count = parse_integer(first)? as usize;
            // Check if there's a BY clause (for TOP)
            if let Some(by_field) = inner.next() {
                Ok(Window::TopBy {
                    count,
                    field: by_field.as_str().to_string(),
                })
            } else {
                Ok(Window::LastN { count })
            }
        }
        Rule::simple_condition => {
            let condition = parse_simple_condition(first)?;
            Ok(Window::Since { condition })
        }
        _ => Err(ParseError::UnexpectedRule(format!("{:?}", first.as_rule()))),
    }
}

/// Parse duration from modifier
fn parse_duration_from_mod(pair: pest::iterators::Pair<Rule>) -> ParseResult<Duration> {
    let inner = pair.into_inner().next().ok_or(ParseError::Empty)?;
    parse_duration(inner)
}

/// Parse duration literal
fn parse_duration(pair: pest::iterators::Pair<Rule>) -> ParseResult<Duration> {
    let s = pair.as_str();

    // Find where the number ends
    let num_end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());

    let num: u64 = s[..num_end]
        .parse()
        .map_err(|_| ParseError::InvalidDuration(s.to_string()))?;
    let unit = &s[num_end..];

    let duration = match unit {
        "ms" => Duration::from_millis(num),
        "s" => Duration::from_secs(num),
        "m" => Duration::from_secs(num * 60),
        "h" => Duration::from_secs(num * 3600),
        "d" => Duration::from_secs(num * 86400),
        _ => return Err(ParseError::InvalidDuration(s.to_string())),
    };

    Ok(duration)
}

/// Parse string literal (remove quotes)
fn parse_string_literal(pair: pest::iterators::Pair<Rule>) -> ParseResult<String> {
    let s = pair.as_str();
    // Remove surrounding quotes
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        let inner = &s[1..s.len() - 1];
        // Handle escape sequences
        let mut result = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(escaped) = chars.next() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        other => {
                            result.push('\\');
                            result.push(other);
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        Ok(result)
    } else {
        Ok(s.to_string())
    }
}

/// Parse variable name (remove $ or {})
fn parse_variable_name(pair: pest::iterators::Pair<Rule>) -> ParseResult<String> {
    let s = pair.as_str();
    if s.starts_with('{') && s.ends_with('}') {
        Ok(s[1..s.len() - 1].to_string())
    } else if s.starts_with('$') {
        Ok(s[1..].to_string())
    } else {
        Ok(s.to_string())
    }
}

/// Parse integer
fn parse_integer(pair: pest::iterators::Pair<Rule>) -> ParseResult<i64> {
    pair.as_str()
        .parse()
        .map_err(|_| ParseError::InvalidInteger(pair.as_str().to_string()))
}

/// Parse float
fn parse_float(pair: pest::iterators::Pair<Rule>) -> ParseResult<f32> {
    pair.as_str()
        .parse()
        .map_err(|_| ParseError::InvalidFloat(pair.as_str().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scan() {
        let stmt = parse("SCAN FROM WORKING").unwrap();
        assert!(matches!(stmt, Statement::Scan(_)));
    }

    #[test]
    fn test_parse_scan_with_window() {
        let stmt = parse("SCAN FROM WORKING WINDOW LAST 10").unwrap();
        if let Statement::Scan(s) = stmt {
            assert!(matches!(s.window, Some(Window::LastN { count: 10 })));
        } else {
            panic!("Expected Scan statement");
        }
    }

    #[test]
    fn test_parse_recall() {
        let stmt = parse("RECALL FROM EPISODIC WHERE pod = \"payments\"").unwrap();
        if let Statement::Recall(r) = stmt {
            assert_eq!(r.memory_type, MemoryType::Episodic);
            if let Predicate::Where { conditions } = r.predicate {
                assert_eq!(conditions.len(), 1);
                assert_eq!(conditions[0].field().unwrap(), "pod");
            } else {
                panic!("Expected Where predicate");
            }
        } else {
            panic!("Expected Recall statement");
        }
    }

    #[test]
    fn test_parse_lookup_with_pattern() {
        let stmt = parse("LOOKUP FROM PROCEDURAL PATTERN $log_events THRESHOLD 0.7").unwrap();
        if let Statement::Lookup(l) = stmt {
            assert_eq!(l.memory_type, MemoryType::Procedural);
            if let Predicate::Pattern { variable, threshold } = l.predicate {
                assert_eq!(variable, "log_events");
                assert_eq!(threshold, Some(0.7));
            } else {
                panic!("Expected Pattern predicate");
            }
        } else {
            panic!("Expected Lookup statement");
        }
    }

    #[test]
    fn test_parse_load_with_modifiers() {
        let stmt = parse("LOAD FROM TOOLS WHERE task = \"bidding\" ORDER BY ranking DESC LIMIT 3")
            .unwrap();
        if let Statement::Load(l) = stmt {
            assert_eq!(l.modifiers.limit, Some(3));
            assert!(l.modifiers.order_by.is_some());
            let order = l.modifiers.order_by.unwrap();
            assert_eq!(order.field, "ranking");
            assert!(!order.ascending);
        } else {
            panic!("Expected Load statement");
        }
    }

    #[test]
    fn test_parse_store() {
        let stmt = parse(
            "STORE INTO EPISODIC (incident_id = \"inc-001\", pod = \"payments\") TTL 7d",
        )
        .unwrap();
        if let Statement::Store(s) = stmt {
            assert_eq!(s.memory_type, MemoryType::Episodic);
            assert_eq!(s.payload.len(), 2);
            assert!(s.modifiers.ttl.is_some());
        } else {
            panic!("Expected Store statement");
        }
    }

    #[test]
    fn test_parse_link() {
        let stmt = parse(
            "LINK FROM PROCEDURAL WHERE pattern_id = \"oom-fix\" \
             TO EPISODIC WHERE incident_id = \"inc-001\" \
             TYPE \"applied_to\" WEIGHT 0.95",
        )
        .unwrap();
        if let Statement::Link(l) = stmt {
            assert_eq!(l.from_type, MemoryType::Procedural);
            assert_eq!(l.to_type, MemoryType::Episodic);
            assert_eq!(l.link_type, "applied_to");
            assert_eq!(l.weight, Some(0.95));
        } else {
            panic!("Expected Link statement");
        }
    }

    #[test]
    fn test_parse_pipeline() {
        let stmt = parse(
            "PIPELINE bid_decision TIMEOUT 80ms \
             LOAD FROM TOOLS WHERE task = \"bidding\" LIMIT 3 \
             | LOOKUP FROM SEMANTIC KEY url = {url}",
        )
        .unwrap();
        if let Statement::Pipeline(p) = stmt {
            assert_eq!(p.name, "bid_decision");
            assert_eq!(p.timeout, Some(Duration::from_millis(80)));
            assert_eq!(p.stages.len(), 2);
        } else {
            panic!("Expected Pipeline statement");
        }
    }

    #[test]
    fn test_parse_anonymous_pipeline() {
        // Anonymous pipeline (no name) - should generate a default name
        // Grammar requires TIMEOUT so we include it
        let stmt = parse("PIPELINE TIMEOUT 50ms SCAN FROM WORKING | RECALL FROM EPISODIC WHERE pod = \"test\"").unwrap();
        if let Statement::Pipeline(p) = stmt {
            // Anonymous pipelines get "_anonymous" as the default name
            assert_eq!(p.name, "_anonymous", "Anonymous pipeline should have '_anonymous' name");
            assert_eq!(p.stages.len(), 2);
            // Verify first stage is SCAN
            assert!(matches!(p.stages[0], Statement::Scan(_)));
            // Verify second stage is RECALL
            assert!(matches!(p.stages[1], Statement::Recall(_)));
        } else {
            panic!("Expected Pipeline statement");
        }
    }

    #[test]
    fn test_parse_pipeline_with_timeout() {
        // Pipeline with TIMEOUT modifier - grammar requires TIMEOUT
        let stmt = parse("PIPELINE quick_scan TIMEOUT 100ms SCAN FROM WORKING LIMIT 5");
        match stmt {
            Ok(Statement::Pipeline(p)) => {
                assert_eq!(p.name, "quick_scan");
                assert!(p.timeout.is_some(), "Pipeline should have TIMEOUT");
                assert_eq!(p.stages.len(), 1);
            }
            Ok(_) => panic!("Expected Pipeline statement"),
            Err(e) => panic!("Should parse pipeline with timeout: {}", e),
        }
    }

    #[test]
    fn test_parse_reflect() {
        let stmt = parse(
            "REFLECT FROM EPISODIC WHERE incident_id = {current}, \
             FROM PROCEDURAL WHERE pattern_id = {matched} \
             WITH LINKS TYPE \"applied_to\"",
        )
        .unwrap();
        if let Statement::Reflect(r) = stmt {
            assert_eq!(r.sources.len(), 2);
            assert!(matches!(
                r.with_links,
                Some(WithLinks::Type { link_type }) if link_type == "applied_to"
            ));
        } else {
            panic!("Expected Reflect statement");
        }
    }

    #[test]
    fn test_parse_forget() {
        let stmt = parse("FORGET FROM WORKING WHERE temp = true").unwrap();
        if let Statement::Forget(f) = stmt {
            assert_eq!(f.memory_type, MemoryType::Working);
            assert_eq!(f.conditions.len(), 1);
        } else {
            panic!("Expected Forget statement");
        }
    }

    #[test]
    fn test_parse_update() {
        let stmt =
            parse("UPDATE INTO PROCEDURAL WHERE pattern_id = \"oom-fix\" (confidence = 0.9)")
                .unwrap();
        if let Statement::Update(u) = stmt {
            assert_eq!(u.memory_type, MemoryType::Procedural);
            assert_eq!(u.conditions.len(), 1);
            assert_eq!(u.payload.len(), 1);
        } else {
            panic!("Expected Update statement");
        }
    }

    #[test]
    fn test_parse_aggregate() {
        let stmt = parse(
            "RECALL FROM EPISODIC WHERE strategy = \"tech_news\" \
             AGGREGATE COUNT(*) AS uses, AVG(ctr) AS avg_ctr \
             HAVING uses > 10",
        )
        .unwrap();
        if let Statement::Recall(r) = stmt {
            assert!(r.modifiers.aggregate.is_some());
            let aggs = r.modifiers.aggregate.unwrap();
            assert_eq!(aggs.len(), 2);
            // Verify alias is parsed correctly
            assert_eq!(aggs[0].alias, Some("uses".to_string()), "COUNT alias should be 'uses'");
            assert_eq!(aggs[1].alias, Some("avg_ctr".to_string()), "AVG alias should be 'avg_ctr'");
            assert!(r.modifiers.having.is_some());
        } else {
            panic!("Expected Recall statement");
        }
    }

    #[test]
    fn test_parse_with_namespace() {
        let stmt = parse("STORE INTO WORKING (key = \"value\") NAMESPACE \"agent-k8s\" SCOPE shared")
            .unwrap();
        if let Statement::Store(s) = stmt {
            assert_eq!(s.modifiers.namespace, Some("agent-k8s".to_string()));
            assert_eq!(s.modifiers.scope, Some(Scope::Shared));
        } else {
            panic!("Expected Store statement");
        }
    }

    #[test]
    fn test_parse_min_confidence() {
        let stmt = parse("RECALL FROM SEMANTIC LIKE $embedding MIN_CONFIDENCE 0.8 LIMIT 5").unwrap();
        if let Statement::Recall(r) = stmt {
            assert_eq!(r.modifiers.min_confidence, Some(0.8));
            assert_eq!(r.modifiers.limit, Some(5));
        } else {
            panic!("Expected Recall statement");
        }
    }

    #[test]
    fn test_parse_or_conditions() {
        // Test OR condition
        let stmt = parse(r#"RECALL FROM WORKING WHERE status = "active" OR status = "pending""#).unwrap();
        if let Statement::Recall(r) = stmt {
            if let Predicate::Where { conditions } = r.predicate {
                assert_eq!(conditions.len(), 2, "Should have 2 conditions");
                assert_eq!(conditions[0].logical_op(), None, "First condition has no preceding op");
                assert_eq!(conditions[1].logical_op(), Some(LogicalOp::Or), "Second condition should have OR");
            } else {
                panic!("Expected Where predicate");
            }
        } else {
            panic!("Expected Recall statement");
        }

        // Test AND condition
        let stmt = parse(r#"RECALL FROM WORKING WHERE status = "active" AND priority > 5"#).unwrap();
        if let Statement::Recall(r) = stmt {
            if let Predicate::Where { conditions } = r.predicate {
                assert_eq!(conditions.len(), 2);
                assert_eq!(conditions[0].logical_op(), None);
                assert_eq!(conditions[1].logical_op(), Some(LogicalOp::And));
            } else {
                panic!("Expected Where predicate");
            }
        } else {
            panic!("Expected Recall statement");
        }

        // Test mixed AND/OR
        let stmt = parse(r#"RECALL FROM WORKING WHERE a = 1 AND b = 2 OR c = 3"#).unwrap();
        if let Statement::Recall(r) = stmt {
            if let Predicate::Where { conditions } = r.predicate {
                assert_eq!(conditions.len(), 3);
                assert_eq!(conditions[0].logical_op(), None);
                assert_eq!(conditions[1].logical_op(), Some(LogicalOp::And));
                assert_eq!(conditions[2].logical_op(), Some(LogicalOp::Or));
            } else {
                panic!("Expected Where predicate");
            }
        } else {
            panic!("Expected Recall statement");
        }
    }

    #[test]
    fn test_parse_reflect_then_store() {
        // B8: REFLECT THEN STORE should parse
        let result = parse(
            r#"REFLECT FROM EPISODIC WHERE campaign = "summer_2026" THEN STORE INTO SEMANTIC (concept = "insight", confidence = 0.75)"#
        );
        match result {
            Ok(Statement::Reflect(r)) => {
                assert!(r.then_clause.is_some(), "THEN clause should be present");
                if let Some(then_stmt) = r.then_clause {
                    assert!(matches!(*then_stmt, Statement::Store(_)), "THEN clause should be STORE");
                }
            }
            Ok(_) => panic!("Expected Reflect statement"),
            Err(e) => panic!("B8: REFLECT THEN STORE failed to parse: {}", e),
        }
    }

    #[test]
    fn test_parse_or_in_forget() {
        // B12: OR in FORGET should parse
        let result = parse(r#"FORGET FROM EPISODIC WHERE campaign = "a" OR campaign = "b""#);
        match result {
            Ok(Statement::Forget(f)) => {
                assert_eq!(f.conditions.len(), 2, "Should have 2 conditions");
                assert_eq!(f.conditions[1].logical_op(), Some(LogicalOp::Or));
            }
            Ok(_) => panic!("Expected Forget statement"),
            Err(e) => panic!("B12: OR in FORGET failed to parse: {}", e),
        }
    }

    #[test]
    fn test_parse_dotted_field_path() {
        // Test dotted field path in condition
        let stmt = parse("RECALL FROM EPISODIC WHERE metadata.pod = \"payments\"").unwrap();
        if let Statement::Recall(r) = stmt {
            if let Predicate::Where { conditions } = r.predicate {
                assert_eq!(conditions[0].field().unwrap(), "metadata.pod");
            } else {
                panic!("Expected Where predicate");
            }
        } else {
            panic!("Expected Recall statement");
        }

        // Test deeply nested field path
        let stmt = parse("RECALL FROM WORKING WHERE data.nested.field = \"value\"").unwrap();
        if let Statement::Recall(r) = stmt {
            if let Predicate::Where { conditions } = r.predicate {
                assert_eq!(conditions[0].field().unwrap(), "data.nested.field");
            } else {
                panic!("Expected Where predicate");
            }
        } else {
            panic!("Expected Recall statement");
        }
    }
}
