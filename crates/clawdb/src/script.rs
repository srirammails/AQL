//! Statement positions in a script (`USE "ns"; RECALL …; …`), for errors that name
//! the statement ("statement 3: …"). `aql_parser::parse_script` drops the USE
//! statements, so the positions as written are found here: the script split at the
//! `;` outside strings and comments. (The same as clawdb-wasm/src/script.rs.)

/// The statements of `input` as written (USE included), split at top-level `;`.
pub fn split(input: &str) -> Vec<&str> {
    let b = input.as_bytes();
    let (mut out, mut start, mut i) = (Vec::new(), 0, 0);
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'-' if b.get(i + 1) == Some(&b'-') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b';' => {
                out.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&input[start.min(input.len())..]);
    out.into_iter().filter(|p| !code(p).is_empty()).collect()
}

/// The statement without leading whitespace and comment lines
fn code(p: &str) -> &str {
    let mut s = p.trim_start();
    while s.starts_with("--") {
        s = s.find('\n').map_or("", |n| &s[n + 1..]).trim_start();
    }
    s
}

fn is_use(p: &str) -> bool {
    let s = code(p);
    s.len() > 3 && s[..3].eq_ignore_ascii_case("USE") && s[3..].starts_with(|c: char| c.is_whitespace() || c == '"')
}

/// The 1-based positions (USE counted) of the statements `parse_script` returns, and
/// the number of statements written. Falls back to 1..=n if the split disagrees.
pub fn positions(input: &str, parsed: usize) -> (Vec<usize>, usize) {
    let pieces = split(input);
    let pos: Vec<usize> = pieces.iter().enumerate().filter(|(_, p)| !is_use(p)).map(|(i, _)| i + 1).collect();
    if pos.len() == parsed {
        (pos, pieces.len())
    } else {
        ((1..=parsed).collect(), pieces.len().max(parsed))
    }
}

/// A script that doesn't parse: name the first statement that doesn't, when there
/// are several.
pub fn parse_failure(input: &str, err: &aql_parser::ParseError) -> String {
    let pieces = split(input);
    if pieces.len() > 1 {
        for (i, p) in pieces.iter().enumerate() {
            if let Err(e) = aql_parser::parse(p) {
                return format!("statement {}: Parse error: {}", i + 1, e);
            }
        }
    }
    format!("Parse error: {}", err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_outside_strings_and_comments() {
        let s = "USE \"a;b\"; -- c;d\nRECALL FROM WORKING WHERE x = \"1;2\";\nSTORE INTO WORKING (x = 1);";
        assert_eq!(split(s).len(), 3);
        assert_eq!(positions(s, 2), (vec![2, 3], 3));
    }
}
