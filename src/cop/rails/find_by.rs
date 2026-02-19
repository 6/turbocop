use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FindBy;

impl Cop for FindBy {
    fn name(&self) -> &'static str {
        "Rails/FindBy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let ignore_where_first = config.get_bool("IgnoreWhereFirst", true);

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        let is_first = chain.outer_method == b"first";
        let is_take = chain.outer_method == b"take";

        if !is_first && !is_take {
            return;
        }

        if chain.inner_method != b"where" {
            return;
        }

        // IgnoreWhereFirst: when true, skip `where(...).first`
        if ignore_where_first && is_first {
            return;
        }

        let method_name = if is_first { "first" } else { "take" };
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `find_by` instead of `where.{method_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindBy, "cops/rails/find_by");

    #[test]
    fn ignore_where_first_true_skips_first() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;

        // Default config (IgnoreWhereFirst: true) should NOT flag where.first
        let config = CopConfig::default();
        let source = b"User.where(name: 'foo').first\n";
        let diags = run_cop_full_with_config(&FindBy, source, config);
        assert!(diags.is_empty(), "IgnoreWhereFirst:true should skip where.first");
    }

    #[test]
    fn ignore_where_first_true_flags_take() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;

        // Default config should still flag where.take
        let config = CopConfig::default();
        let source = b"User.where(name: 'foo').take\n";
        let diags = run_cop_full_with_config(&FindBy, source, config);
        assert!(!diags.is_empty(), "IgnoreWhereFirst:true should still flag where.take");
    }

    #[test]
    fn ignore_where_first_false_flags_first() {
        use crate::cop::CopConfig;
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreWhereFirst".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"User.where(name: 'foo').first\n";
        let diags = run_cop_full_with_config(&FindBy, source, config);
        assert!(!diags.is_empty(), "IgnoreWhereFirst:false should flag where.first");
    }
}
