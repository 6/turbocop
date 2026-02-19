use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};

pub struct WhereExists;

impl Cop for WhereExists {
    fn name(&self) -> &'static str {
        "Rails/WhereExists"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyle", "exists");

        let result = match style {
            "where" => self.check_where_style(source, node),
            _ => self.check_exists_style(source, node),
        };
        diagnostics.extend(result);
    }
}

impl WhereExists {
    /// "exists" style: flag `where(...).exists?`, suggest `exists?(...)`
    fn check_exists_style(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if chain.outer_method != b"exists?" {
            return Vec::new();
        }

        if chain.inner_method != b"where" {
            return Vec::new();
        }

        // The inner `where` call should have arguments
        if chain.inner_call.arguments().is_none() {
            return Vec::new();
        }

        // The outer `exists?` should NOT have arguments — if it does, the
        // developer is already passing conditions to exists? and this is a
        // different pattern (e.g., `where(a: 1).exists?(['sql', val])`)
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if outer_call.arguments().is_some() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `exists?(...)` instead of `where(...).exists?`.".to_string(),
        )]
    }

    /// "where" style: flag `exists?(...)` with arguments, suggest `where(...).exists?`
    fn check_where_style(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"exists?" {
            return Vec::new();
        }

        // Must have arguments (exists? with args => should be where(...).exists?)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        // Only flag hash-like or array args (not bare integers like exists?(1))
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check that the arg is a hash, keyword hash, or array
        let first = &arg_list[0];
        let is_convertible = first.as_hash_node().is_some()
            || first.as_keyword_hash_node().is_some()
            || first.as_array_node().is_some()
            || arg_list.len() > 1;

        if !is_convertible {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `where(...).exists?` instead of `exists?(...)`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhereExists, "cops/rails/where_exists");

    #[test]
    fn where_style_flags_exists_with_hash_arg() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("where".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.exists?(name: 'john')\n";
        let diags = run_cop_full_with_config(&WhereExists, source, config);
        assert!(!diags.is_empty(), "where style should flag exists? with hash args");
    }

    #[test]
    fn where_style_allows_where_exists() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".to_string(),
                serde_yml::Value::String("where".to_string()),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.where(name: 'john').exists?\n";
        assert_cop_no_offenses_full_with_config(&WhereExists, source, config);
    }
}
