use crate::cop::util::{self, is_rspec_example_group, is_rspec_let, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let max = config.get_usize("Max", 5);
        let allow_subject = config.get_bool("AllowSubject", true);

        let mut visitor = MemoizedHelperVisitor {
            cop: self,
            source,
            max,
            allow_subject,
            // Stack of ancestor let counts (each entry is the direct count for that group)
            ancestor_counts: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct MemoizedHelperVisitor<'a> {
    cop: &'a MultipleMemoizedHelpers,
    source: &'a SourceFile,
    max: usize,
    allow_subject: bool,
    /// Stack of direct let/subject counts for each ancestor example group.
    ancestor_counts: Vec<usize>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> MemoizedHelperVisitor<'a> {
    /// Check if a call node is an example group (describe, context, etc.)
    fn is_example_group_call(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let method_name = call.name().as_slice();
        if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        }
    }

    /// Count direct let/let!/subject/subject! declarations in a block body.
    fn count_direct_helpers(&self, block: &ruby_prism::BlockNode<'_>) -> usize {
        let body = match block.body() {
            Some(b) => b,
            None => return 0,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return 0,
        };
        let mut count = 0;
        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_none() {
                    let name = c.name().as_slice();
                    if is_rspec_let(name) {
                        count += 1;
                    } else if !self.allow_subject && util::is_rspec_subject(name) {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}

impl<'pr> Visit<'pr> for MemoizedHelperVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if !self.is_example_group_call(node) {
            // Not an example group — just continue visiting children
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let block = match node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            },
            None => {
                ruby_prism::visit_call_node(self, node);
                return;
            }
        };

        // Count direct helpers for this group
        let direct_count = self.count_direct_helpers(&block);

        // Total = own + all ancestors
        let ancestor_total: usize = self.ancestor_counts.iter().sum();
        let total = ancestor_total + direct_count;

        if total > self.max {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "Example group has too many memoized helpers [{total}/{}]",
                    self.max
                ),
            ));
        }

        // Push this group's direct count onto the ancestor stack and recurse
        self.ancestor_counts.push(direct_count);
        ruby_prism::visit_call_node(self, node);
        self.ancestor_counts.pop();
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

    #[test]
    fn nested_context_inherits_parent_lets() {
        // Parent has 4 lets, nested context has 2 lets = 6 total, exceeds max of 5
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n\n  context 'nested' do\n    let(:e) { 5 }\n    let(:f) { 6 }\n    it { expect(true).to be true }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        // The nested context should fire because 4 + 2 = 6 > 5
        // The parent describe should NOT fire (4 <= 5)
        assert_eq!(diags.len(), 1, "Should fire on nested context with 6 total helpers");
        assert!(diags[0].message.contains("[6/5]"));
    }
}
