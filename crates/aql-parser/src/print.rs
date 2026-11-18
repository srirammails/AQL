//! AQL text from the AST: `Statement`'s `Display`. `parse(&stmt.to_string())` gives
//! the same statement back, so a server can pass statements on as text (a script's
//! statements with its USE applied, a router's call to a node).

use std::fmt::{self, Display, Formatter, Write};
use std::time::Duration;

use crate::ast::*;

fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn duration(d: &Duration) -> String {
    let ms = d.as_millis();
    if ms % 86_400_000 == 0 && ms > 0 {
        format!("{}d", ms / 86_400_000)
    } else if ms % 3_600_000 == 0 && ms > 0 {
        format!("{}h", ms / 3_600_000)
    } else if ms % 60_000 == 0 && ms > 0 {
        format!("{}m", ms / 60_000)
    } else if ms % 1000 == 0 && ms > 0 {
        format!("{}s", ms / 1000)
    } else {
        format!("{ms}ms")
    }
}

/// A float that reads back as a float (`1.0`, not `1`).
fn float(f: f64) -> String {
    let s = f.to_string();
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => f.write_str(&float(*x)),
            Value::String(s) => f.write_str(&quoted(s)),
            Value::Variable(v) => write!(f, "${v}"),
            Value::Array(a) => {
                f.write_char('[')?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_char(']')
            }
        }
    }
}

impl Display for TimePoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TimePoint::At { time } => f.write_str(&quoted(time)),
            TimePoint::Variable { name } => write!(f, "${name}"),
        }
    }
}

fn operator(op: Operator) -> &'static str {
    match op {
        Operator::Eq => "=",
        Operator::Ne => "!=",
        Operator::Gt => ">",
        Operator::Gte => ">=",
        Operator::Lt => "<",
        Operator::Lte => "<=",
        Operator::Contains => "CONTAINS",
        Operator::StartsWith => "STARTS_WITH",
        Operator::EndsWith => "ENDS_WITH",
        Operator::In => "IN",
        Operator::Between => "BETWEEN",
    }
}

fn conditions(cs: &[Condition]) -> String {
    let mut out = String::new();
    for (i, c) in cs.iter().enumerate() {
        if i > 0 {
            out.push_str(match c.logical_op() {
                Some(LogicalOp::Or) => " OR ",
                _ => " AND ",
            });
        }
        match c {
            Condition::Group { conditions: g, .. } => {
                let _ = write!(out, "({})", conditions(g));
            }
            Condition::Simple { field, operator: Operator::Between, value: Value::Array(ends), .. } if ends.len() == 2 => {
                let _ = write!(out, "{field} BETWEEN {} AND {}", ends[0], ends[1]);
            }
            Condition::Simple { field, operator: Operator::In, value: Value::Array(list), .. } => {
                let items: Vec<String> = list.iter().map(|v| v.to_string()).collect();
                let _ = write!(out, "{field} IN ({})", items.join(", "));
            }
            Condition::Simple { field, operator: op, value, .. } => {
                let _ = write!(out, "{field} {} {value}", operator(*op));
            }
        }
    }
    out
}

fn payload(p: &[FieldAssignment]) -> String {
    let items: Vec<String> = p.iter().map(|a| format!("{} = {}", a.field, a.value)).collect();
    format!("({})", items.join(", "))
}

fn link_select(s: &LinkSelect) -> String {
    let mut out = String::new();
    match s.types.as_slice() {
        [] => {}
        [one] => {
            let _ = write!(out, " TYPE {}", quoted(one));
        }
        many => {
            let items: Vec<String> = many.iter().map(|t| quoted(t)).collect();
            let _ = write!(out, " TYPE ({})", items.join(", "));
        }
    }
    if let Some(w) = s.min_weight {
        let _ = write!(out, " WEIGHT {}", float(w));
    }
    out
}

/// The modifiers, each with a leading space (IN SCOPE is printed by the caller where
/// the statement puts it).
fn modifiers(m: &Modifiers, in_scope: bool) -> String {
    let mut out = String::new();
    if in_scope {
        if let Some(s) = &m.in_scope {
            let _ = write!(out, " IN SCOPE {}", quoted(s));
        }
    }
    if let Some(w) = &m.window {
        match w {
            Window::LastN { count } => {
                let _ = write!(out, " WINDOW LAST {count}");
            }
            Window::LastDuration { duration: d } => {
                let _ = write!(out, " WINDOW LAST {}", duration(d));
            }
        }
    }
    if let Some(a) = &m.aggregate {
        let items: Vec<String> = a
            .iter()
            .map(|f| {
                let name = match f.func {
                    AggregateFuncType::Count => "COUNT",
                    AggregateFuncType::Sum => "SUM",
                    AggregateFuncType::Avg => "AVG",
                    AggregateFuncType::Min => "MIN",
                    AggregateFuncType::Max => "MAX",
                };
                let mut s = format!("{name}({})", f.field.as_deref().unwrap_or("*"));
                if let Some(alias) = &f.alias {
                    let _ = write!(s, " AS {alias}");
                }
                s
            })
            .collect();
        let _ = write!(out, " AGGREGATE {}", items.join(", "));
    }
    if let Some(g) = &m.group_by {
        let _ = write!(out, " GROUP BY {}", g.join(", "));
    }
    if let Some(h) = &m.having {
        let _ = write!(out, " HAVING {}", conditions(h));
    }
    if let Some(w) = &m.with_links {
        let _ = write!(out, " WITH LINKS{}", link_select(w));
    }
    if let Some(fl) = &m.follow_links {
        let _ = write!(out, " FOLLOW LINKS{}", link_select(&fl.select));
        if let Some(d) = fl.depth {
            let _ = write!(out, " DEPTH {d}");
        }
    }
    if let Some(t) = &m.as_of {
        let _ = write!(out, " AS OF {t}");
    }
    if let Some(h) = &m.with_history {
        out.push_str(" WITH HISTORY");
        if let Some((a, b)) = &h.between {
            let _ = write!(out, " BETWEEN {a} AND {b}");
        }
    }
    if let Some(c) = m.confidence {
        let _ = write!(out, " CONFIDENCE {}", float(c as f64));
    }
    if let Some(o) = &m.order_by {
        let _ = write!(out, " ORDER BY {} {}", o.field, if o.ascending { "ASC" } else { "DESC" });
    }
    if let Some(n) = m.limit {
        let _ = write!(out, " LIMIT {n}");
    }
    if let Some(r) = &m.return_fields {
        let _ = write!(out, " RETURN {}", r.join(", "));
    }
    if let Some(t) = &m.ttl {
        let _ = write!(out, " TTL {}", duration(t));
    }
    match (&m.namespace, m.namespaces.as_slice()) {
        (_, [_, ..]) => {
            let items: Vec<String> = m.namespaces.iter().map(|n| quoted(n)).collect();
            let _ = write!(out, " NAMESPACE ({})", items.join(", "));
        }
        (Some(ns), []) => {
            let _ = write!(out, " NAMESPACE {}", quoted(ns));
        }
        (None, []) => {}
    }
    out
}

fn recall(r: &RecallStmt) -> String {
    let mut out = format!("RECALL FROM {}", r.memory_type);
    if let Some(s) = &r.modifiers.in_scope {
        let _ = write!(out, " IN SCOPE {}", quoted(s));
    }
    match &r.predicate {
        Predicate::Where { conditions: c } if !c.is_empty() => {
            let _ = write!(out, " WHERE {}", conditions(c));
        }
        Predicate::Like { text: Some(t), .. } => {
            let _ = write!(out, " LIKE {}", quoted(t));
        }
        Predicate::Like { variable, text: None } => {
            let _ = write!(out, " LIKE ${variable}");
        }
        _ => {}
    }
    out.push_str(&modifiers(&r.modifiers, false));
    out
}

fn ends(from: MemoryType, fc: &[Condition], to: MemoryType, tc: &[Condition]) -> String {
    format!("FROM {from} WHERE {} INTO {to} WHERE {}", conditions(fc), conditions(tc))
}

impl Display for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Use(ns) => write!(f, "USE {}", quoted(ns)),
            Statement::Pipeline(p) => {
                f.write_str("PIPELINE")?;
                if !p.name.is_empty() {
                    write!(f, " {}", p.name)?;
                }
                if let Some(t) = &p.timeout {
                    write!(f, " TIMEOUT {}", duration(t))?;
                }
                for (i, s) in p.stages.iter().enumerate() {
                    f.write_str(if i == 0 { " " } else { " | " })?;
                    write!(f, "{s}")?;
                }
                Ok(())
            }
            Statement::Recall(r) => f.write_str(&recall(r)),
            Statement::Store(s) => write!(f, "STORE INTO {} {}{}", s.memory_type, payload(&s.payload), modifiers(&s.modifiers, true)),
            Statement::Update(u) => {
                write!(f, "UPDATE INTO {} WHERE {} {}{}", u.memory_type, conditions(&u.conditions), payload(&u.payload), modifiers(&u.modifiers, true))
            }
            Statement::Forget(x) => {
                write!(f, "FORGET FROM {}", x.memory_type)?;
                if !x.conditions.is_empty() {
                    write!(f, " WHERE {}", conditions(&x.conditions))?;
                }
                f.write_str(&modifiers(&x.modifiers, true))
            }
            Statement::Link(l) => {
                write!(f, "LINK {} TYPE {}", ends(l.from_type, &l.from_conditions, l.to_type, &l.to_conditions), quoted(&l.link_type))?;
                if let Some(w) = l.weight {
                    write!(f, " WEIGHT {}", float(w))?;
                }
                if let Some(ns) = &l.namespace {
                    write!(f, " NAMESPACE {}", quoted(ns))?;
                }
                Ok(())
            }
            Statement::Unlink(u) => {
                write!(f, "UNLINK {}", ends(u.from_type, &u.from_conditions, u.to_type, &u.to_conditions))?;
                if let Some(t) = &u.link_type {
                    write!(f, " TYPE {}", quoted(t))?;
                }
                if let Some(ns) = &u.namespace {
                    write!(f, " NAMESPACE {}", quoted(ns))?;
                }
                Ok(())
            }
            Statement::Branch(n) | Statement::Fork(n) => {
                let verb = if matches!(self, Statement::Branch(_)) { "BRANCH" } else { "FORK" };
                write!(f, "{verb} FROM {} INTO {}", quoted(&n.from), quoted(&n.into))?;
                if let Some(t) = &n.as_of {
                    write!(f, " AS OF {t}")?;
                }
                Ok(())
            }
            Statement::Merge(m) => {
                write!(f, "MERGE FROM {} INTO {}", quoted(&m.from), quoted(&m.into))?;
                match m.prefer {
                    Some(Prefer::Source) => f.write_str(" PREFER SOURCE"),
                    Some(Prefer::Target) => f.write_str(" PREFER TARGET"),
                    None => Ok(()),
                }
            }
            Statement::Subscribe(s) => match &s.target {
                SubscribeTarget::Query(r) => write!(f, "SUBSCRIBE {}", recall(r)),
                SubscribeTarget::Topic(t) => {
                    write!(f, "SUBSCRIBE TOPIC {}", quoted(t))?;
                    if let Some(c) = &s.filter {
                        write!(f, " WHERE {}", conditions(c))?;
                    }
                    if let Some(ns) = &s.namespace {
                        write!(f, " NAMESPACE {}", quoted(ns))?;
                    }
                    Ok(())
                }
            },
            Statement::Unsubscribe(u) => {
                write!(f, "UNSUBSCRIBE {}", quoted(&u.subscription))?;
                if let Some(ns) = &u.namespace {
                    write!(f, " NAMESPACE {}", quoted(ns))?;
                }
                Ok(())
            }
            Statement::Publish(p) => write!(f, "PUBLISH TOPIC {} {}{}", quoted(&p.topic), payload(&p.payload), modifiers(&p.modifiers, true)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse, parse_script};

    #[test]
    fn printed_statements_parse_back_the_same() {
        for q in [
            r#"RECALL FROM SEMANTIC WHERE key = "person:ada" NAMESPACE "crm""#,
            r#"RECALL FROM SEMANTIC IN SCOPE "team:x" WHERE a = 1 AND (b > 2.5 OR c IN ("x", "y")) LIMIT 5"#,
            r#"RECALL FROM EPISODIC WHERE t BETWEEN "2026-11-01" AND $to ORDER BY t DESC LIMIT 10 RETURN a, b.c"#,
            r#"RECALL FROM PROCEDURAL LIKE "pod \"OOM\" restart" CONFIDENCE 0.7"#,
            r#"RECALL FROM SEMANTIC LIKE $q NAMESPACE ("crm", "kb") LIMIT 5"#,
            "RECALL FROM WORKING WINDOW LAST 30s",
            "RECALL FROM WORKING WINDOW LAST 10",
            r#"RECALL FROM EPISODIC WHERE x = 1 AGGREGATE COUNT(*) AS n, AVG(ms) AS m GROUP BY pod HAVING n > 2"#,
            r#"RECALL FROM SEMANTIC WHERE key = "a" WITH LINKS TYPE ("x", "y") WEIGHT 0.5"#,
            r#"RECALL FROM SEMANTIC WHERE key = "a" FOLLOW LINKS TYPE "works_at" DEPTH 2"#,
            r#"RECALL FROM SEMANTIC WHERE key = "a" AS OF "2026-11-01T09:00:00Z""#,
            r#"RECALL FROM SEMANTIC WITH HISTORY BETWEEN "2026-10-01" AND "2026-11-01""#,
            r#"RECALL FROM ALL WHERE url = "example.com""#,
            r#"STORE INTO EPISODIC (event = "login", n = 2, ok = true, none = null, tags = ["a", 1]) TTL 7d NAMESPACE "ops""#,
            r#"STORE INTO SEMANTIC (key = "k", v = 1.0) IN SCOPE "team:x""#,
            r#"UPDATE INTO SEMANTIC WHERE key = "ada" (aliases = ["ada-l"]) NAMESPACE "crm""#,
            r#"FORGET FROM ALL NAMESPACE "crm-try""#,
            r#"FORGET FROM WORKING WHERE stale = true"#,
            r#"LINK FROM SEMANTIC WHERE key = "a" INTO EPISODIC WHERE key = "b" TYPE "t" WEIGHT 0.9 NAMESPACE "p""#,
            r#"UNLINK FROM SEMANTIC WHERE key = "a" INTO SEMANTIC WHERE key = "b" TYPE "t""#,
            r#"BRANCH FROM "crm" INTO "crm-try" AS OF "2026-11-01T00:00:00Z""#,
            r#"FORK FROM "crm" INTO "crm-copy""#,
            r#"MERGE FROM "crm-try" INTO "crm" PREFER TARGET"#,
            r#"PIPELINE bid TIMEOUT 80ms RECALL FROM TOOLS WHERE task = "bidding" LIMIT 3 | RECALL FROM SEMANTIC WHERE key = {url} | STORE INTO EPISODIC (url = {url})"#,
            r#"SUBSCRIBE TOPIC "orders" WHERE status = "paid" NAMESPACE "shop""#,
            r#"SUBSCRIBE RECALL FROM SEMANTIC WHERE kind = "c" LIMIT 20 NAMESPACE "crm""#,
            r#"UNSUBSCRIBE "sub-7f3a""#,
            r#"PUBLISH TOPIC "orders" (id = "o-1") TTL 1h NAMESPACE "shop""#,
            r#"USE "crm""#,
        ] {
            let a = parse(q).unwrap_or_else(|e| panic!("{q}: {e}"));
            let text = a.to_string();
            let b = parse(&text).unwrap_or_else(|e| panic!("{q}\n printed as {text}\n: {e}"));
            assert_eq!(a, b, "{q}\n printed as {text}");
        }
        // A script's statements with USE applied, as text, carry the namespace.
        let s = parse_script(r#"USE "crm"; RECALL FROM SEMANTIC WHERE key = "a""#, None).unwrap();
        assert_eq!(s[0].to_string(), r#"RECALL FROM SEMANTIC WHERE key = "a" NAMESPACE "crm""#);
    }
}
