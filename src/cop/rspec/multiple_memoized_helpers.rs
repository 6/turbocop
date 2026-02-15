use crate::cop::util::{self, is_rspec_example_group, is_rspec_let, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MultipleMemoizedHelpers;

impl Cop for MultipleMemoizedHelpers {
    fn name(&self) -> &'static str {
        "RSpec/MultipleMemoizedHelpers"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for example group calls (describe, context, etc., including ::RSpec.describe)
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let max = config.get_usize("Max", 5);
        // Config: AllowSubject — when true (default), don't count subject declarations
        let allow_subject = config.get_bool("AllowSubject", true);

        // Count direct let/let!/subject/subject! declarations in this block
        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut count = 0;
        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_none() {
                    let name = c.name().as_slice();
                    if is_rspec_let(name) {
                        count += 1;
                    } else if !allow_subject && util::is_rspec_subject(name) {
                        count += 1;
                    }
                }
            }
        }

        if count > max {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            vec![self.diagnostic(
                source,
                line,
                column,
                format!("Example group has too many memoized helpers [{count}/{max}]"),
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleMemoizedHelpers, "cops/rspec/multiple_memoized_helpers");

    #[test]
    fn allow_subject_false_counts_subject() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowSubject".into(), serde_yml::Value::Bool(false)),
                ("Max".into(), serde_yml::Value::Number(serde_yml::Number::from(2))),
            ]),
            ..CopConfig::default()
        };
        // 2 lets + 1 subject = 3 helpers, max is 2
        let source = b"describe Foo do\n  subject(:bar) { 1 }\n  let(:a) { 1 }\n  let(:b) { 2 }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&MultipleMemoizedHelpers, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn allow_subject_true_does_not_count_subject() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowSubject".into(), serde_yml::Value::Bool(true)),
                ("Max".into(), serde_yml::Value::Number(serde_yml::Number::from(2))),
            ]),
            ..CopConfig::default()
        };
        // 2 lets + 1 subject = 2 counted helpers (subject excluded), max is 2
        let source = b"describe Foo do\n  subject(:bar) { 1 }\n  let(:a) { 1 }\n  let(:b) { 2 }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&MultipleMemoizedHelpers, source, config);
        assert!(diags.is_empty());
    }
}
