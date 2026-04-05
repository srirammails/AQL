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
//! let stmt = parse("SCAN FROM WORKING WINDOW LAST 10").unwrap();
//! println!("{:?}", stmt);
//! ```

pub mod ast;
pub mod error;
pub mod parser;

// Re-exports for convenience
pub use ast::*;
pub use error::{ParseError, ParseResult};
pub use parser::parse;
