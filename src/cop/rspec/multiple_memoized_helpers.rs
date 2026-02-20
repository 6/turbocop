use std::collections::HashSet;

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
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max = config.get_usize("Max", 5);
        let allow_subject = config.get_bool("AllowSubject", true);

        let mut visitor = MemoizedHelperVisitor {
            cop: self,
            source,
            max,
            allow_subject,
            // Stack of ancestor helper name sets (each entry is the set of names for that group)
            ancestor_names: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MemoizedHelperVisitor<'a> {
    cop: &'a MultipleMemoizedHelpers,
    source: &'a SourceFile,
    max: usize,
    allow_subject: bool,
    /// Stack of helper name sets for each ancestor example group.
    /// Each entry contains the names defined directly in that group.
    ancestor_names: Vec<HashSet<Vec<u8>>>,
    diagnostics: Vec<Diagnostic>,
}

/// Extract the helper name from a let/let!/subject/subject! call.
/// For `let(:foo) { ... }` or `let(:foo!) { ... }`, returns "foo" or "foo!".
/// For `subject(:bar) { ... }`, returns "bar".
/// For bare `subject { ... }`, returns "subject".
fn extract_helper_name(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let method_name = call.name().as_slice();

    // For subject/subject! without args, the name is "subject"
    if util::is_rspec_subject(method_name) {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(first) = arg_list.first() {
                if let Some(sym) = first.as_symbol_node() {
                    return Some(sym.unescaped().to_vec());
                }
            }
        }
        // Bare subject/subject! — use "subject" as the name
        return Some(b"subject".to_vec());
    }

    // For let/let!, extract the symbol name from the first argument
    if is_rspec_let(method_name) {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(first) = arg_list.first() {
                if let Some(sym) = first.as_symbol_node() {
                    return Some(sym.unescaped().to_vec());
                }
            }
        }
    }

    None
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

    /// Collect the set of helper names defined directly in a block body.
    fn collect_direct_helper_names(&self, block: &ruby_prism::BlockNode<'_>) -> HashSet<Vec<u8>> {
        let mut names = HashSet::new();
        let body = match block.body() {
            Some(b) => b,
            None => return names,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return names,
        };
        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_none() {
                    let method_name = c.name().as_slice();
                    if is_rspec_let(method_name) {
                        if let Some(name) = extract_helper_name(&c) {
                            names.insert(name);
                        }
                    } else if !self.allow_subject && util::is_rspec_subject(method_name) {
                        if let Some(name) = extract_helper_name(&c) {
                            names.insert(name);
                        }
                    }
                }
            }
        }
        names
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

        // Collect direct helper names for this group
        let direct_names = self.collect_direct_helper_names(&block);

        // Total = union of all ancestor names + this group's names
        // Overrides (same name in child) don't increase the count.
        let mut all_names: HashSet<Vec<u8>> = HashSet::new();
        for ancestor_set in &self.ancestor_names {
            for name in ancestor_set {
                all_names.insert(name.clone());
            }
        }
        for name in &direct_names {
            all_names.insert(name.clone());
        }
        let total = all_names.len();

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

        // Push this group's direct names onto the ancestor stack and recurse
        self.ancestor_names.push(direct_names);
        ruby_prism::visit_call_node(self, node);
        self.ancestor_names.pop();
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

    #[test]
    fn overriding_lets_in_child_do_not_increase_count() {
        // Parent has 5 lets at the limit. Child overrides 2 of them.
        // Total unique names = 5 (not 7), so no offense.
        let source = b"describe Foo do\n  let(:a) { 1 }\n  let(:b) { 2 }\n  let(:c) { 3 }\n  let(:d) { 4 }\n  let(:e) { 5 }\n\n  context 'overrides' do\n    let(:a) { 10 }\n    let(:b) { 20 }\n    it { expect(a).to eq(10) }\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&MultipleMemoizedHelpers, source);
        assert!(diags.is_empty(), "Overriding lets should not increase count: {:?}", diags);
    }
}
