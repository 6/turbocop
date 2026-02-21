//! NodePattern DSL support — lexer, parser, mapping table, and interpreter.
//!
//! This module extracts the shared infrastructure from the `node_pattern_codegen`
//! binary into reusable library code. The codegen binary imports from here.

pub mod extract;
pub mod interpreter;
pub mod lexer;
pub mod mapping;
pub mod parser;
pub mod pattern_db;

pub use extract::{extract_patterns, ExtractedPattern, PatternKind};
pub use interpreter::interpret_pattern;
pub use lexer::{Lexer, Token};
pub use mapping::{build_mapping_table, NodeMapping};
pub use parser::{pattern_summary, Parser, PatternNode};
