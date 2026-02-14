pub mod layout;
pub mod lint;
pub mod metrics;
pub mod naming;
pub mod registry;
pub mod style;
pub mod util;
pub mod walker;

use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Per-cop configuration extracted from .rubocop.yml.
#[derive(Debug, Clone)]
pub struct CopConfig {
    pub enabled: bool,
    pub severity: Option<Severity>,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub options: HashMap<String, serde_yml::Value>,
}

impl Default for CopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: None,
            exclude: Vec::new(),
            include: Vec::new(),
            options: HashMap::new(),
        }
    }
}

/// A lint rule. Implementations must be Send + Sync so they can be shared
/// across rayon worker threads.
pub trait Cop: Send + Sync {
    /// The fully-qualified cop name, e.g. "Style/FrozenStringLiteralComment".
    fn name(&self) -> &'static str;

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    /// Line-based check — runs before AST traversal.
    #[allow(unused_variables)]
    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        Vec::new()
    }

    /// Source-based check — runs once per file with full parse context and CodeMap.
    ///
    /// Use this for cops that scan raw source bytes while needing to skip
    /// non-code regions (strings, comments, regexps).
    #[allow(unused_variables)]
    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        Vec::new()
    }

    /// Node-based check — called for every AST node during traversal.
    #[allow(unused_variables)]
    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        Vec::new()
    }
}
