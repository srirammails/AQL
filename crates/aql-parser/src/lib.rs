//! AQL Parser - Agent Query Language parser
//!
//! This crate provides a parser for AQL (Agent Query Language) that converts
//! query strings into an abstract syntax tree (AST) for execution.
//!
//! # Example
//!
//! ```
//! use aql_parser::parse;
//!
//! let stmt = parse("RECALL FROM WORKING WINDOW LAST 10").unwrap();
//! println!("{:?}", stmt);
//! ```

pub mod ast;
pub mod error;
pub mod parser;
pub mod print;

// Re-exports for convenience
pub use ast::*;
pub use error::{ParseError, ParseResult};
pub use parser::{is_script, parse, parse_script, script_steps};

/// This crate's release (`0.N.x`), which parses AQL language version `v0.N`
/// ([`LANGUAGE_VERSION`]). See `CHANGELOG.md`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The AQL language version this parser implements (`spec/AQL_SPEC_v0.13.md`).
pub const LANGUAGE_VERSION: &str = "v0.13";

/// The AQL grammar (pest), for tools and LLM clients that need the language definition.
pub const GRAMMAR: &str = include_str!("grammar/aql.pest");

#[cfg(test)]
mod version_tests {
    #[test]
    fn the_release_matches_the_language_version() {
        // Release 0.N.x parses language v0.N.
        let mut parts = super::VERSION.split('.');
        let release = format!("v{}.{}", parts.next().unwrap(), parts.next().unwrap());
        assert_eq!(release, super::LANGUAGE_VERSION);
        assert!(super::GRAMMAR.contains(&format!("Agent Query Language {}", super::LANGUAGE_VERSION)), "the grammar's header names the version");
    }
}
