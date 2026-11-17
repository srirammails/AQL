//! AQL v0.13 parser (`spec/AQL_SPEC_v0.13.md`): query strings into the AST, with pest.

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::time::Duration;

use crate::ast::*;
use crate::error::{ParseError, ParseResult};

/// The pest parser for AQL
#[derive(Parser)]
#[grammar = "grammar/aql.pest"]
pub struct AqlParser;

type P<'a> = Pair<'a, Rule>;

/// Parse an AQL statement.
pub fn parse(input: &str) -> ParseResult<Statement> {
    let pairs = AqlParser::parse(Rule::aql, input).map_err(ParseError::from_pest)?;
    let stmt = pairs
        .flat_map(|p| p.into_inner())
        .find(|p| p.as_rule() == Rule::statement)
        .ok_or(ParseError::Empty)
        .and_then(statement)?;
    validate(&stmt)?;
    Ok(stmt)
}

/// Parse a script: statements separated by `;`. Each `USE "ns"` gives the statements
/// after it that name no namespace that namespace; the USE statements themselves are
/// not returned. `default` is a session's namespace in force before the script.
pub fn parse_script(input: &str, default: Option<&str>) -> ParseResult<Vec<Statement>> {
    let pairs = AqlParser::parse(Rule::script, input).map_err(ParseError::from_pest)?;
    let mut current = default.map(String::from);
    let mut out = Vec::new();
    for p in pairs.flat_map(|p| p.into_inner()).filter(|p| p.as_rule() == Rule::statement) {
        let stmt = statement(p)?;
        validate(&stmt)?;
        match stmt {
            Statement::Use(ns) => current = Some(ns),
            s => out.push(match &current {
                Some(ns) => s.with_default_namespace(ns),
                None => s,
            }),
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- statements

fn statement(pair: P) -> ParseResult<Statement> {
    let inner = pair.into_inner().next().ok_or(ParseError::Empty)?;
    Ok(match inner.as_rule() {
        Rule::use_stmt => Statement::Use(string(first(inner)?)?),
        Rule::pipeline_stmt => Statement::Pipeline(pipeline(inner)?),
        Rule::recall_stmt => Statement::Recall(recall(inner)?),
        Rule::store_stmt => Statement::Store(store(inner)?),
        Rule::update_stmt => Statement::Update(update(inner)?),
        Rule::forget_stmt => Statement::Forget(forget(inner)?),
        Rule::link_stmt => Statement::Link(link(inner)?),
        Rule::unlink_stmt => Statement::Unlink(unlink(inner)?),
        Rule::branch_stmt => Statement::Branch(namespace_stmt(inner)?),
        Rule::fork_stmt => Statement::Fork(namespace_stmt(inner)?),
        Rule::merge_stmt => Statement::Merge(merge(inner)?),
        Rule::subscribe_stmt => Statement::Subscribe(subscribe(inner)?),
        Rule::unsubscribe_stmt => Statement::Unsubscribe(unsubscribe(inner)?),
        Rule::publish_stmt => Statement::Publish(publish(inner)?),
        r => return Err(ParseError::UnexpectedRule(format!("{r:?}"))),
    })
}

fn pipeline(pair: P) -> ParseResult<PipelineStmt> {
    let mut p = PipelineStmt { name: String::new(), timeout: None, stages: Vec::new() };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::pipeline_name => p.name = item.as_str().trim().to_string(),
            Rule::timeout_mod => p.timeout = Some(duration(first(item)?)?),
            Rule::pipeline_stage => {
                let s = first(item)?;
                p.stages.push(match s.as_rule() {
                    Rule::recall_stmt => Statement::Recall(recall(s)?),
                    Rule::store_stmt => Statement::Store(store(s)?),
                    Rule::update_stmt => Statement::Update(update(s)?),
                    r => return Err(ParseError::UnexpectedRule(format!("{r:?}"))),
                });
            }
            _ => {}
        }
    }
    Ok(p)
}

fn recall(pair: P) -> ParseResult<RecallStmt> {
    let mut memory_type = MemoryType::All;
    let mut predicate = Predicate::All;
    let mut modifiers = Modifiers::default();
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::memory_type => memory_type = memory(item)?,
            Rule::in_scope_mod => modifiers.in_scope = Some(string(first(item)?)?),
            Rule::predicate => predicate = predicate_of(first(item)?)?,
            Rule::modifier => modifier(item, &mut modifiers)?,
            _ => {}
        }
    }
    Ok(RecallStmt { memory_type, predicate, modifiers })
}

fn predicate_of(pair: P) -> ParseResult<Predicate> {
    match pair.as_rule() {
        Rule::where_pred => Ok(Predicate::Where { conditions: conditions(first(pair)?)? }),
        Rule::like_pred => {
            let v = first(pair)?;
            Ok(match v.as_rule() {
                Rule::variable => Predicate::Like { variable: variable(v), text: None },
                _ => {
                    let text = string(v)?;
                    Predicate::Like { variable: text.clone(), text: Some(text) }
                }
            })
        }
        r => Err(ParseError::UnexpectedRule(format!("{r:?}"))),
    }
}

fn store(pair: P) -> ParseResult<StoreStmt> {
    let mut s = StoreStmt { memory_type: MemoryType::All, payload: Vec::new(), modifiers: Modifiers::default() };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::memory_type => s.memory_type = memory(item)?,
            Rule::payload => s.payload = payload(item)?,
            Rule::modifier => modifier(item, &mut s.modifiers)?,
            _ => {}
        }
    }
    Ok(s)
}

fn update(pair: P) -> ParseResult<UpdateStmt> {
    let mut u = UpdateStmt { memory_type: MemoryType::All, conditions: Vec::new(), payload: Vec::new(), modifiers: Modifiers::default() };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::memory_type => u.memory_type = memory(item)?,
            Rule::where_pred => u.conditions = conditions(first(item)?)?,
            Rule::payload => u.payload = payload(item)?,
            Rule::modifier => modifier(item, &mut u.modifiers)?,
            _ => {}
        }
    }
    Ok(u)
}

fn forget(pair: P) -> ParseResult<ForgetStmt> {
    let mut f = ForgetStmt { memory_type: MemoryType::All, conditions: Vec::new(), modifiers: Modifiers::default() };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::memory_type => f.memory_type = memory(item)?,
            Rule::where_pred => f.conditions = conditions(first(item)?)?,
            Rule::modifier => modifier(item, &mut f.modifiers)?,
            _ => {}
        }
    }
    Ok(f)
}

/// `FROM <memory> WHERE … INTO <memory> WHERE …`
fn ends(pair: P) -> ParseResult<(MemoryType, Vec<Condition>, MemoryType, Vec<Condition>)> {
    let mut types = Vec::new();
    let mut conds = Vec::new();
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::memory_type => types.push(memory(item)?),
            Rule::where_pred => conds.push(conditions(first(item)?)?),
            _ => {}
        }
    }
    let (Some(to_c), Some(from_c)) = (conds.pop(), conds.pop()) else { return Err(ParseError::Missing("the link's ends")) };
    let (Some(to_t), Some(from_t)) = (types.pop(), types.pop()) else { return Err(ParseError::Missing("the link's ends")) };
    Ok((from_t, from_c, to_t, to_c))
}

fn link(pair: P) -> ParseResult<LinkStmt> {
    let mut l = LinkStmt {
        from_type: MemoryType::All,
        from_conditions: Vec::new(),
        to_type: MemoryType::All,
        to_conditions: Vec::new(),
        link_type: String::new(),
        weight: None,
        namespace: None,
    };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::link_ends => (l.from_type, l.from_conditions, l.to_type, l.to_conditions) = ends(item)?,
            Rule::string_literal => l.link_type = string(item)?,
            Rule::number => l.weight = Some(number(item)? as f32),
            Rule::namespace_mod => l.namespace = Some(string(first(item)?)?),
            _ => {}
        }
    }
    Ok(l)
}

fn unlink(pair: P) -> ParseResult<UnlinkStmt> {
    let mut u = UnlinkStmt {
        from_type: MemoryType::All,
        from_conditions: Vec::new(),
        to_type: MemoryType::All,
        to_conditions: Vec::new(),
        link_type: None,
        namespace: None,
    };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::link_ends => (u.from_type, u.from_conditions, u.to_type, u.to_conditions) = ends(item)?,
            Rule::string_literal => u.link_type = Some(string(item)?),
            Rule::namespace_mod => u.namespace = Some(string(first(item)?)?),
            _ => {}
        }
    }
    Ok(u)
}

/// `FROM "a" INTO "b"`
fn from_into(pair: P) -> ParseResult<(String, String)> {
    let mut it = pair.into_inner();
    let from = string(it.next().ok_or(ParseError::Missing("FROM namespace"))?)?;
    let into = string(it.next().ok_or(ParseError::Missing("INTO namespace"))?)?;
    Ok((from, into))
}

fn namespace_stmt(pair: P) -> ParseResult<NamespaceStmt> {
    let mut n = NamespaceStmt { from: String::new(), into: String::new(), as_of: None };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::ns_from_into => (n.from, n.into) = from_into(item)?,
            Rule::as_of_mod => n.as_of = Some(time_point(first(item)?)?),
            _ => {}
        }
    }
    Ok(n)
}

fn merge(pair: P) -> ParseResult<MergeStmt> {
    let mut m = MergeStmt { from: String::new(), into: String::new(), prefer: None };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::ns_from_into => (m.from, m.into) = from_into(item)?,
            Rule::prefer_clause => {
                m.prefer = Some(match first(item)?.as_rule() {
                    Rule::prefer_source => Prefer::Source,
                    _ => Prefer::Target,
                })
            }
            _ => {}
        }
    }
    Ok(m)
}

fn subscribe(pair: P) -> ParseResult<SubscribeStmt> {
    let inner = first(pair)?;
    match inner.as_rule() {
        Rule::recall_stmt => Ok(SubscribeStmt { target: SubscribeTarget::Query(Box::new(recall(inner)?)), filter: None, namespace: None }),
        _ => {
            let mut s = SubscribeStmt { target: SubscribeTarget::Topic(String::new()), filter: None, namespace: None };
            for item in inner.into_inner() {
                match item.as_rule() {
                    Rule::topic_target => s.target = SubscribeTarget::Topic(string(first(item)?)?),
                    Rule::where_pred => s.filter = Some(conditions(first(item)?)?),
                    Rule::namespace_mod => s.namespace = Some(string(first(item)?)?),
                    _ => {}
                }
            }
            Ok(s)
        }
    }
}

fn unsubscribe(pair: P) -> ParseResult<UnsubscribeStmt> {
    let mut u = UnsubscribeStmt { subscription: String::new(), namespace: None };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::handle => u.subscription = handle(item)?,
            Rule::namespace_mod => u.namespace = Some(string(first(item)?)?),
            _ => {}
        }
    }
    Ok(u)
}

fn publish(pair: P) -> ParseResult<PublishStmt> {
    let mut p = PublishStmt { topic: String::new(), payload: Vec::new(), modifiers: Modifiers::default() };
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::topic_target => p.topic = string(first(item)?)?,
            Rule::payload => p.payload = payload(item)?,
            Rule::modifier => modifier(item, &mut p.modifiers)?,
            _ => {}
        }
    }
    Ok(p)
}

// ---------------------------------------------------------------- validation

/// What the grammar doesn't say (spec §8): HAVING and GROUP BY need AGGREGATE;
/// FORGET FROM ALL without WHERE names its NAMESPACE; BRANCH / FORK / MERGE name two
/// different namespaces; a pipeline writes only in its last stage.
fn validate(stmt: &Statement) -> ParseResult<()> {
    let bad = |m: &str| Err(ParseError::Validation(m.into()));
    match stmt {
        Statement::Pipeline(p) => {
            for (i, s) in p.stages.iter().enumerate() {
                if i + 1 < p.stages.len() && !matches!(s, Statement::Recall(_)) {
                    return bad("only a pipeline's last stage may write (STORE or UPDATE)");
                }
                validate(s)?;
            }
            Ok(())
        }
        Statement::Store(StoreStmt { memory_type: MemoryType::All, .. }) | Statement::Update(UpdateStmt { memory_type: MemoryType::All, .. }) => {
            bad("STORE and UPDATE name one memory type, not ALL")
        }
        Statement::Link(LinkStmt { from_type, to_type, .. }) | Statement::Unlink(UnlinkStmt { from_type, to_type, .. })
            if *from_type == MemoryType::All || *to_type == MemoryType::All =>
        {
            bad("a link's ends are in one memory type each, not ALL")
        }
        Statement::Recall(r) => aggregates(&r.modifiers),
        Statement::Subscribe(SubscribeStmt { target: SubscribeTarget::Query(r), .. }) => aggregates(&r.modifiers),
        Statement::Forget(f) if f.conditions.is_empty() && f.modifiers.namespace.is_none() => {
            bad("FORGET without WHERE forgets everything: name its NAMESPACE")
        }
        Statement::Branch(n) | Statement::Fork(n) if n.from == n.into || n.from.is_empty() || n.into.is_empty() => {
            bad("BRANCH / FORK need two different namespaces")
        }
        Statement::Merge(m) if m.from == m.into || m.from.is_empty() || m.into.is_empty() => bad("MERGE needs two different namespaces"),
        Statement::Use(ns) if ns.is_empty() => bad("USE names a namespace"),
        _ => Ok(()),
    }
}

fn aggregates(m: &Modifiers) -> ParseResult<()> {
    if (m.having.is_some() || m.group_by.is_some()) && m.aggregate.is_none() {
        return Err(ParseError::Validation("HAVING and GROUP BY need AGGREGATE".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------- modifiers

fn modifier(pair: P, m: &mut Modifiers) -> ParseResult<()> {
    let item = first(pair)?;
    match item.as_rule() {
        Rule::namespace_mod => m.namespace = Some(string(first(item)?)?),
        Rule::in_scope_mod => m.in_scope = Some(string(first(item)?)?),
        Rule::limit_mod => m.limit = Some(integer(first(item)?)?.max(0) as usize),
        Rule::order_mod => {
            let mut it = item.into_inner();
            let field = it.next().ok_or(ParseError::Missing("ORDER BY field"))?.as_str().to_string();
            let ascending = it.next().is_none_or(|d| !d.as_str().eq_ignore_ascii_case("DESC"));
            m.order_by = Some(OrderBy { field, ascending });
        }
        Rule::return_mod => m.return_fields = Some(item.into_inner().map(|f| f.as_str().to_string()).collect()),
        Rule::ttl_mod => m.ttl = Some(duration(first(item)?)?),
        Rule::confidence_mod => m.confidence = Some(number(first(item)?)? as f32),
        Rule::aggregate_mod => m.aggregate = Some(item.into_inner().map(aggregate).collect::<ParseResult<_>>()?),
        Rule::group_by_mod => m.group_by = Some(item.into_inner().map(|f| f.as_str().to_string()).collect()),
        Rule::having_mod => m.having = Some(conditions(first(item)?)?),
        Rule::window_mod => {
            let v = first(item)?;
            m.window = Some(match v.as_rule() {
                Rule::duration_literal => Window::LastDuration { duration: duration(v)? },
                _ => Window::LastN { count: integer(v)?.max(0) as usize },
            });
        }
        Rule::with_links_mod => m.with_links = Some(link_select(item.into_inner())?),
        Rule::follow_links_mod => {
            let mut depth = None;
            let mut parts = Vec::new();
            for p in item.into_inner() {
                match p.as_rule() {
                    Rule::integer_literal => depth = Some(integer(p)?.max(0) as u32),
                    _ => parts.push(p),
                }
            }
            m.follow_links = Some(FollowLinks { select: link_select(parts.into_iter())?, depth });
        }
        Rule::as_of_mod => m.as_of = Some(time_point(first(item)?)?),
        Rule::with_history_mod => {
            let mut points = item.into_inner().map(time_point).collect::<ParseResult<Vec<_>>>()?;
            let between = match (points.pop(), points.pop()) {
                (Some(to), Some(from)) => Some((from, to)),
                _ => None,
            };
            m.with_history = Some(History { between });
        }
        r => return Err(ParseError::UnexpectedRule(format!("{r:?}"))),
    }
    Ok(())
}

fn link_select<'a>(parts: impl Iterator<Item = P<'a>>) -> ParseResult<LinkSelect> {
    let mut s = LinkSelect::default();
    for p in parts {
        match p.as_rule() {
            Rule::link_types => s.types = p.into_inner().map(string).collect::<ParseResult<_>>()?,
            Rule::weight_min => s.min_weight = Some(number(first(p)?)?),
            _ => {}
        }
    }
    Ok(s)
}

fn aggregate(pair: P) -> ParseResult<AggregateFunc> {
    let mut func = None;
    let mut field = None;
    let mut alias = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::agg_func_name => func = AggregateFuncType::from_str(p.as_str()),
            Rule::field_path => field = Some(p.as_str().to_string()),
            Rule::agg_alias => alias = p.into_inner().find(|i| i.as_rule() == Rule::identifier).map(|i| i.as_str().to_string()),
            _ => {}
        }
    }
    Ok(AggregateFunc { func: func.ok_or(ParseError::Missing("aggregate function"))?, field, alias })
}

// ---------------------------------------------------------------- conditions

fn conditions(pair: P) -> ParseResult<Vec<Condition>> {
    let mut out = Vec::new();
    let mut op = None;
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::logical_op => op = LogicalOp::from_str(p.as_str()),
            Rule::condition_atom => {
                let a = first(p)?;
                let mut c = match a.as_rule() {
                    Rule::paren_condition => Condition::group(conditions(first(a)?)?),
                    _ => simple(a)?,
                };
                c.set_logical_op(op.take());
                out.push(c);
            }
            _ => {}
        }
    }
    Ok(out)
}

fn simple(pair: P) -> ParseResult<Condition> {
    let mut it = pair.into_inner();
    let field = it.next().ok_or(ParseError::Missing("field"))?.as_str().to_string();
    let next = it.next().ok_or(ParseError::Missing("operator"))?;
    Ok(match next.as_rule() {
        Rule::between_op => {
            let vals = next.into_inner().map(value).collect::<ParseResult<Vec<_>>>()?;
            Condition::simple(field, Operator::Between, Value::Array(vals))
        }
        Rule::in_op => {
            let v = first(next)?;
            let list = match v.as_rule() {
                Rule::value_list | Rule::array_literal => Value::Array(v.into_inner().map(value).collect::<ParseResult<_>>()?),
                _ => Value::Variable(variable(v)),
            };
            Condition::simple(field, Operator::In, list)
        }
        _ => {
            let op = Operator::from_str(next.as_str()).ok_or_else(|| ParseError::InvalidOperator(next.as_str().into()))?;
            let v = value(it.next().ok_or(ParseError::Missing("value"))?)?;
            Condition::simple(field, op, v)
        }
    })
}

fn payload(pair: P) -> ParseResult<Vec<FieldAssignment>> {
    pair.into_inner()
        .map(|a| {
            let mut it = a.into_inner();
            let field = it.next().ok_or(ParseError::Missing("field"))?.as_str().to_string();
            let value = value(it.next().ok_or(ParseError::Missing("value"))?)?;
            Ok(FieldAssignment { field, value })
        })
        .collect()
}

// ---------------------------------------------------------------- primitives

fn first(pair: P) -> ParseResult<P> {
    pair.into_inner().next().ok_or(ParseError::Empty)
}

fn memory(pair: P) -> ParseResult<MemoryType> {
    MemoryType::from_str(pair.as_str()).ok_or_else(|| ParseError::InvalidMemoryType(pair.as_str().into()))
}

fn value(pair: P) -> ParseResult<Value> {
    let p = if pair.as_rule() == Rule::value { first(pair)? } else { pair };
    Ok(match p.as_rule() {
        Rule::null_literal => Value::Null,
        Rule::bool_literal => Value::Bool(p.as_str().eq_ignore_ascii_case("true")),
        Rule::integer_literal => Value::Int(integer(p)?),
        Rule::float_literal => Value::Float(p.as_str().parse().map_err(|_| ParseError::InvalidFloat(p.as_str().into()))?),
        Rule::string_literal => Value::String(string(p)?),
        Rule::variable => Value::Variable(variable(p)),
        Rule::array_literal => Value::Array(p.into_inner().map(value).collect::<ParseResult<_>>()?),
        r => return Err(ParseError::UnexpectedRule(format!("{r:?}"))),
    })
}

fn number(pair: P) -> ParseResult<f64> {
    let p = if pair.as_rule() == Rule::number { first(pair)? } else { pair };
    p.as_str().parse().map_err(|_| ParseError::InvalidFloat(p.as_str().into()))
}

fn integer(pair: P) -> ParseResult<i64> {
    pair.as_str().parse().map_err(|_| ParseError::InvalidInteger(pair.as_str().into()))
}

/// `$name` or `{name}`: the name.
fn variable(pair: P) -> String {
    let s = pair.as_str();
    s.strip_prefix('$').or_else(|| s.strip_prefix('{').and_then(|v| v.strip_suffix('}'))).unwrap_or(s).to_string()
}

fn handle(pair: P) -> ParseResult<String> {
    let p = first(pair)?;
    Ok(match p.as_rule() {
        Rule::string_literal => string(p)?,
        Rule::variable => format!("${}", variable(p)),
        _ => p.as_str().to_string(),
    })
}

fn time_point(pair: P) -> ParseResult<TimePoint> {
    let p = if pair.as_rule() == Rule::time_point { first(pair)? } else { pair };
    Ok(match p.as_rule() {
        Rule::variable => TimePoint::Variable { name: variable(p) },
        _ => TimePoint::At { time: string(p)? },
    })
}

fn duration(pair: P) -> ParseResult<Duration> {
    let s = pair.as_str();
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let n: u64 = s[..split].parse().map_err(|_| ParseError::InvalidDuration(s.into()))?;
    Ok(match &s[split..] {
        "ms" => Duration::from_millis(n),
        "s" => Duration::from_secs(n),
        "m" => Duration::from_secs(n * 60),
        "h" => Duration::from_secs(n * 3600),
        "d" => Duration::from_secs(n * 86_400),
        _ => return Err(ParseError::InvalidDuration(s.into())),
    })
}

fn string(pair: P) -> ParseResult<String> {
    let s = pair.as_str();
    let inner = s.strip_prefix('"').and_then(|v| v.strip_suffix('"')).ok_or_else(|| ParseError::Pest(format!("not a string: {s}")))?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(q: &str) -> Statement {
        parse(q).unwrap_or_else(|e| panic!("{q}: {e}"))
    }
    fn recall_of(q: &str) -> RecallStmt {
        match ok(q) {
            Statement::Recall(r) => r,
            other => panic!("{q}: {other:?}"),
        }
    }

    #[test]
    fn recall_is_every_read() {
        let r = recall_of(r#"RECALL FROM SEMANTIC WHERE key = "person:ada" NAMESPACE "crm""#);
        assert_eq!(r.memory_type, MemoryType::Semantic);
        assert!(matches!(&r.predicate, Predicate::Where { conditions } if conditions[0].field() == Some("key")));
        assert_eq!(r.modifiers.namespace.as_deref(), Some("crm"));
        assert_eq!(recall_of("RECALL FROM TOOLS WHERE task = \"bidding\" LIMIT 3").modifiers.limit, Some(3));
        assert_eq!(recall_of("RECALL FROM WORKING WINDOW LAST 30s").modifiers.window, Some(Window::LastDuration { duration: Duration::from_secs(30) }));
        assert_eq!(recall_of("RECALL FROM WORKING WINDOW LAST 10").modifiers.window, Some(Window::LastN { count: 10 }));
        assert_eq!(recall_of("RECALL FROM EPISODIC").predicate, Predicate::All);
        let r = recall_of(r#"RECALL FROM PROCEDURAL LIKE "pod OOMKilled restart" CONFIDENCE 0.7"#);
        assert_eq!(r.predicate, Predicate::Like { variable: "pod OOMKilled restart".into(), text: Some("pod OOMKilled restart".into()) });
        assert_eq!(r.modifiers.confidence, Some(0.7));
        assert_eq!(recall_of("RECALL FROM SEMANTIC LIKE $q").predicate, Predicate::Like { variable: "q".into(), text: None });
        assert_eq!(recall_of(r#"RECALL FROM SEMANTIC IN SCOPE "team:x" WHERE a = 1"#).modifiers.in_scope.as_deref(), Some("team:x"));
    }

    #[test]
    fn conditions_between_in_and_groups() {
        let r = recall_of(r#"RECALL FROM EPISODIC WHERE created_at BETWEEN "2026-11-01" AND "2026-11-15" AND (pod = "a" OR pod IN ("b", "c"))"#);
        let Predicate::Where { conditions } = r.predicate else { panic!() };
        assert_eq!(conditions[0].operator(), Some(Operator::Between));
        assert_eq!(conditions[0].value(), Some(&Value::Array(vec![Value::String("2026-11-01".into()), Value::String("2026-11-15".into())])));
        let Condition::Group { conditions: g, logical_op } = &conditions[1] else { panic!() };
        assert_eq!(*logical_op, Some(LogicalOp::And));
        assert_eq!(g[1].operator(), Some(Operator::In));
        assert_eq!(g[1].logical_op(), Some(LogicalOp::Or));
    }

    #[test]
    fn writes() {
        let Statement::Store(s) = ok(r#"STORE INTO EPISODIC (incident = "inc-1", n = 2, tags = ["a"]) TTL 7d NAMESPACE "ops""#) else { panic!() };
        assert_eq!(s.payload.len(), 3);
        assert_eq!(s.modifiers.ttl, Some(Duration::from_secs(7 * 86_400)));
        let Statement::Update(u) = ok(r#"UPDATE INTO SEMANTIC WHERE key = "person:ada" (aliases = ["person:ada-lovelace"], age = null)"#) else { panic!() };
        assert_eq!(u.payload[1].value, Value::Null);
        let Statement::Forget(f) = ok(r#"FORGET FROM ALL NAMESPACE "crm-try""#) else { panic!() };
        assert!(f.conditions.is_empty());
        assert!(parse("FORGET FROM ALL").unwrap_err().to_string().contains("NAMESPACE"));
        assert!(parse("UPDATE INTO SEMANTIC (a = 1)").is_err(), "UPDATE needs WHERE");
        assert!(parse(r#"STORE INTO ALL (a = 1)"#).unwrap_err().to_string().contains("ALL"));
        assert!(parse(r#"LINK FROM ALL WHERE a = 1 INTO SEMANTIC WHERE b = 2 TYPE "t""#).unwrap_err().to_string().contains("ALL"));
    }

    #[test]
    fn links() {
        let Statement::Link(l) = ok(r#"LINK FROM SEMANTIC WHERE key = "person:ada" INTO SEMANTIC WHERE key = "org:acme" TYPE "works_at" WEIGHT 0.9 NAMESPACE "crm""#) else { panic!() };
        assert_eq!((l.link_type.as_str(), l.weight, l.namespace.as_deref()), ("works_at", Some(0.9), Some("crm")));
        assert_eq!(l.to_type, MemoryType::Semantic);
        let Statement::Unlink(u) = ok(r#"UNLINK FROM SEMANTIC WHERE key = "a" INTO EPISODIC WHERE key = "b""#) else { panic!() };
        assert_eq!((u.to_type, u.link_type), (MemoryType::Episodic, None));
        assert!(parse(r#"LINK FROM SEMANTIC WHERE key = "a" TO SEMANTIC WHERE key = "b" TYPE "t""#).is_err(), "INTO, not TO");
        let r = recall_of(r#"RECALL FROM SEMANTIC WHERE key = "org:acme" FOLLOW LINKS TYPE ("works_at", "owns") WEIGHT 0.5 DEPTH 2"#);
        let f = r.modifiers.follow_links.unwrap();
        assert_eq!((f.select.types.len(), f.select.min_weight, f.depth), (2, Some(0.5), Some(2)));
        let r = recall_of(r#"RECALL FROM SEMANTIC WHERE key = "a" WITH LINKS"#);
        assert_eq!(r.modifiers.with_links, Some(LinkSelect::default()));
    }

    #[test]
    fn namespaces() {
        assert_eq!(ok(r#"BRANCH FROM "crm" INTO "crm-try""#), Statement::Branch(NamespaceStmt { from: "crm".into(), into: "crm-try".into(), as_of: None }));
        let Statement::Fork(f) = ok(r#"FORK FROM "crm" INTO "crm-nov" AS OF "2026-11-01T00:00:00Z""#) else { panic!() };
        assert_eq!(f.as_of, Some(TimePoint::At { time: "2026-11-01T00:00:00Z".into() }));
        assert_eq!(ok(r#"MERGE FROM "crm-try" INTO "crm" PREFER SOURCE"#), Statement::Merge(MergeStmt { from: "crm-try".into(), into: "crm".into(), prefer: Some(Prefer::Source) }));
        assert!(parse(r#"BRANCH FROM "a" INTO "a""#).is_err());
        assert!(parse(r#"MERGE FROM "a" INTO "a""#).is_err());
    }

    #[test]
    fn history() {
        let r = recall_of(r#"RECALL FROM SEMANTIC WHERE key = "a" AS OF "2026-11-01T09:00:00Z""#);
        assert_eq!(r.modifiers.as_of, Some(TimePoint::At { time: "2026-11-01T09:00:00Z".into() }));
        let r = recall_of(r#"RECALL FROM SEMANTIC WHERE key = "a" WITH HISTORY BETWEEN "2026-11-01" AND $to"#);
        assert_eq!(r.modifiers.with_history, Some(History { between: Some((TimePoint::At { time: "2026-11-01".into() }, TimePoint::Variable { name: "to".into() })) }));
        assert_eq!(recall_of(r#"RECALL FROM SEMANTIC WITH HISTORY"#).modifiers.with_history, Some(History::default()));
        assert!(parse(r#"RECALL FROM SEMANTIC AS OF 2d AGO"#).is_err(), "no AGO");
    }

    #[test]
    fn flow() {
        let Statement::Pipeline(p) = ok("PIPELINE bid TIMEOUT 80ms RECALL FROM TOOLS WHERE task = \"bidding\" LIMIT 3 | RECALL FROM SEMANTIC WHERE key = {url} | STORE INTO EPISODIC (url = {url})") else { panic!() };
        assert_eq!((p.name.as_str(), p.timeout, p.stages.len()), ("bid", Some(Duration::from_millis(80)), 3));
        let e = parse("PIPELINE STORE INTO EPISODIC (a = 1) | RECALL FROM EPISODIC").unwrap_err().to_string();
        assert!(e.contains("last stage"), "{e}");
        let Statement::Subscribe(s) = ok(r#"SUBSCRIBE TOPIC "orders" WHERE status = "paid" NAMESPACE "shop""#) else { panic!() };
        assert_eq!((s.target, s.namespace.as_deref()), (SubscribeTarget::Topic("orders".into()), Some("shop")));
        let Statement::Subscribe(s) = ok(r#"SUBSCRIBE RECALL FROM SEMANTIC WHERE kind = "customer" LIMIT 20 NAMESPACE "crm""#) else { panic!() };
        assert!(matches!(s.target, SubscribeTarget::Query(r) if r.modifiers.namespace.as_deref() == Some("crm")));
        assert_eq!(ok(r#"UNSUBSCRIBE "sub-7f3a""#), Statement::Unsubscribe(UnsubscribeStmt { subscription: "sub-7f3a".into(), namespace: None }));
        let Statement::Publish(p) = ok(r#"PUBLISH TOPIC "orders" (order_id = "o-1") TTL 7d NAMESPACE "shop""#) else { panic!() };
        assert_eq!(p.modifiers.namespace.as_deref(), Some("shop"));
    }

    #[test]
    fn aggregates_and_case() {
        let r = recall_of("recall from episodic where pod = \"a\" aggregate count(*) as n, avg(ms) as avg_ms group by pod having n > 2 order by n desc");
        assert_eq!(r.modifiers.aggregate.as_ref().map(|a| a.len()), Some(2));
        assert_eq!(r.modifiers.order_by, Some(OrderBy { field: "n".into(), ascending: false }));
        assert!(parse("RECALL FROM EPISODIC GROUP BY pod").unwrap_err().to_string().contains("AGGREGATE"));
    }

    #[test]
    fn use_sets_the_namespace_of_what_follows() {
        assert_eq!(ok(r#"USE "crm""#), Statement::Use("crm".into()));
        let s = parse_script(r#"USE "crm"; RECALL FROM SEMANTIC WHERE key = "a"; STORE INTO EPISODIC (e = 1) NAMESPACE "ops"; LINK FROM SEMANTIC WHERE key = "a" INTO SEMANTIC WHERE key = "b" TYPE "t";"#, None).unwrap();
        assert_eq!(s.len(), 3, "the USE itself isn't returned");
        let Statement::Recall(r) = &s[0] else { panic!() };
        assert_eq!(r.modifiers.namespace.as_deref(), Some("crm"));
        let Statement::Store(st) = &s[1] else { panic!() };
        assert_eq!(st.modifiers.namespace.as_deref(), Some("ops"), "a statement's own NAMESPACE wins");
        let Statement::Link(l) = &s[2] else { panic!() };
        assert_eq!(l.namespace.as_deref(), Some("crm"));
        // A session's namespace before the script; a later USE changes it.
        let s = parse_script(r#"RECALL FROM WORKING; USE "b"; RECALL FROM WORKING"#, Some("a")).unwrap();
        let ns = |i: usize| match &s[i] { Statement::Recall(r) => r.modifiers.namespace.clone(), _ => None };
        assert_eq!((ns(0).as_deref(), ns(1).as_deref()), (Some("a"), Some("b")));
        // BRANCH names its own namespaces.
        let s = parse_script(r#"USE "crm"; BRANCH FROM "crm" INTO "crm-try""#, None).unwrap();
        assert_eq!(s[0], Statement::Branch(NamespaceStmt { from: "crm".into(), into: "crm-try".into(), as_of: None }));
        assert!(parse("USE crm").is_err(), "a namespace is a string");
    }

    #[test]
    fn what_was_removed_does_not_parse() {
        for q in [
            r#"LOOKUP FROM SEMANTIC KEY id = "a""#,
            "SCAN FROM WORKING",
            r#"LOAD FROM TOOLS WHERE task = "x""#,
            r#"STORE INTO SEMANTIC FACT (subject = "a", predicate = "p", object = "b")"#,
            r#"MERGE ENTITY "a" INTO "b""#,
            r#"RECALL FROM SEMANTIC WHERE a = 1 BRANCH "b""#,
            r#"RECALL FROM SEMANTIC WHERE a = 1 VALID AT "2026-01-01""#,
            r#"RECALL FROM SEMANTIC LIKE $q MIN_CONFIDENCE 0.7"#,
            r#"ACK "s" OFFSET 1"#,
            r#"WATCH RECALL FROM SEMANTIC"#,
            r#"RECALL FROM SEMANTIC WHERE a = 1 SCOPE private"#,
            r#"RECALL FROM EPISODIC WHERE a = 1 TIMEOUT 5s"#,
        ] {
            assert!(parse(q).is_err(), "{q} should not parse");
        }
    }
}
