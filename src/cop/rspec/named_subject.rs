use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_hook};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct NamedSubject;

/// Flags usage of bare `subject` inside examples/hooks when it should be named.
///
/// EnforcedStyle:
/// - `always` (default): flag every bare `subject` reference in examples/hooks
/// - `named_only`: only flag when the nearest enclosing subject definition is named
impl Cop for NamedSubject {
    fn name(&self) -> &'static str {
        "RSpec/NamedSubject"
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
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "always");
        let named_only = style == "named_only";
        // Config: IgnoreSharedExamples — skip shared example groups
        let ignore_shared = config.get_bool("IgnoreSharedExamples", true);

        // Walk the AST to find bare `subject` references
        let mut finder = BareSubjectFinder {
            source,
            cop: self,
            ignore_shared,
            named_only,
            in_shared: false,
            in_example_or_hook: false,
            // Stack tracking whether the nearest enclosing scope has a named
            // subject definition. `None` = no subject defined in this scope,
            // `Some(true)` = named, `Some(false)` = unnamed.
            subject_named_stack: Vec::new(),
            diags: Vec::new(),
        };
        finder.visit(&parse_result.node());
        diagnostics.extend(finder.diags);
    }
}

/// Check whether any direct child statement of a block body is a `subject`
/// definition (a call to `subject` with a block). Returns:
/// - `Some(true)` if the subject has arguments (named: `subject(:foo) { ... }`)
/// - `Some(false)` if the subject has no arguments (unnamed: `subject { ... }`)
/// - `None` if no subject definition is found
fn find_subject_in_block(block_node: &ruby_prism::BlockNode<'_>) -> Option<bool> {
    let body = block_node.body()?;
    let stmts = body.as_statements_node()?;
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            if call.name().as_slice() == b"subject"
                && call.receiver().is_none()
                && call.block().is_some()
            {
                return Some(call.arguments().is_some());
            }
        }
    }
    None
}

/// Check if a call node is a shared example group definition, including
/// both receiverless (`shared_examples`) and qualified (`RSpec.shared_examples`).
fn is_shared_group_call(node: &ruby_prism::CallNode<'_>) -> bool {
    let name = node.name().as_slice();
    let is_shared_name =
        name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context";
    if !is_shared_name {
        return false;
    }
    // Receiverless or RSpec.shared_*
    if node.receiver().is_none() {
        return true;
    }
    node.receiver().is_some_and(|r| {
        crate::cop::util::constant_name(&r)
            .is_some_and(|n| n == b"RSpec" || n.starts_with(b"RSpec::"))
    })
}

struct BareSubjectFinder<'a> {
    source: &'a SourceFile,
    cop: &'a NamedSubject,
    ignore_shared: bool,
    named_only: bool,
    in_shared: bool,
    in_example_or_hook: bool,
    /// Stack of subject-named states for enclosing blocks.
    /// Each entry is `Some(true)` (named subject), `Some(false)` (unnamed), or
    /// `None` (no subject definition in that scope).
    subject_named_stack: Vec<Option<bool>>,
    diags: Vec<Diagnostic>,
}

impl BareSubjectFinder<'_> {
    /// Check if the nearest enclosing subject definition is named.
    /// Walks the stack from top to bottom, returning `true` if the nearest
    /// scope with a subject definition has a named subject.
    fn nearest_subject_is_named(&self) -> bool {
        for named in self.subject_named_stack.iter().rev().flatten() {
            return *named;
        }
        false
    }

    fn should_flag(&self) -> bool {
        if self.in_shared || !self.in_example_or_hook {
            return false;
        }
        if self.named_only {
            // Only flag if nearest enclosing subject definition is named
            self.nearest_subject_is_named()
        } else {
            // `always` style: always flag bare subject
            true
        }
    }
}

impl<'pr> Visit<'pr> for BareSubjectFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();

        // Track if we're inside a shared example group
        if is_shared_group_call(node) && self.ignore_shared {
            let was = self.in_shared;
            self.in_shared = true;
            ruby_prism::visit_call_node(self, node);
            self.in_shared = was;
            return;
        }

        // Track if we're inside an example or hook block (it, specify, before, after, etc.)
        let is_example = node.receiver().is_none()
            && node.block().is_some()
            && (is_rspec_example(name) || is_rspec_hook(name));

        if is_example {
            let was = self.in_example_or_hook;
            self.in_example_or_hook = true;

            // Also push subject state for blocks
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let subject_state = find_subject_in_block(&block_node);
                    self.subject_named_stack.push(subject_state);
                    ruby_prism::visit_call_node(self, node);
                    self.subject_named_stack.pop();
                    self.in_example_or_hook = was;
                    return;
                }
            }

            ruby_prism::visit_call_node(self, node);
            self.in_example_or_hook = was;
            return;
        }

        // Check for `subject` reference (no receiver, no arguments).
        // RuboCop's `subject_usage` matches `(send nil? :subject)` which finds
        // ANY bare `subject` call inside example/hook blocks, including
        // `subject { ... }` (the send node inside the block node). So we don't
        // check for `node.block().is_none()` — a `subject { ... }` inside a
        // hook is still a reference, not a definition.
        if name == b"subject"
            && node.receiver().is_none()
            && node.arguments().is_none()
            && self.should_flag()
        {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diags.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Name your test subject if you need to reference it explicitly.".to_string(),
            ));
        }

        // When entering any block, check if this scope defines `subject` and
        // push that info onto the stack.
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let subject_state = find_subject_in_block(&block_node);
                self.subject_named_stack.push(subject_state);

                ruby_prism::visit_call_node(self, node);

                self.subject_named_stack.pop();
                return;
            }
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't descend into method definitions — `subject` in a def is a
        // method call, not a test subject reference.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NamedSubject, "cops/rspec/named_subject");

    #[test]
    fn named_only_style_skips_without_named_subject() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("named_only".into()),
            )]),
            ..CopConfig::default()
        };
        // File with bare `subject` but no named subject declaration
        let source =
            b"describe Foo do\n  it 'works' do\n    expect(subject).to be_valid\n  end\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NamedSubject, source, config);
        assert!(
            diags.is_empty(),
            "named_only should not flag without named subject"
        );
    }

    #[test]
    fn ignore_shared_examples_skips_shared_groups() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("IgnoreSharedExamples".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = b"shared_examples 'foo' do\n  it { subject }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NamedSubject, source, config);
        assert!(
            diags.is_empty(),
            "IgnoreSharedExamples should skip shared groups"
        );
    }

    #[test]
    fn ignore_shared_examples_false_flags_shared_groups() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "IgnoreSharedExamples".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"shared_examples 'foo' do\n  it { subject }\nend\n";
        let diags = crate::testutil::run_cop_full_with_config(&NamedSubject, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn named_only_nearest_unnamed_subject_not_flagged() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("named_only".into()),
            )]),
            ..CopConfig::default()
        };
        // File has a named subject in one context but the nearest subject
        // to the usage is unnamed — should NOT be flagged.
        let source = b"describe Foo do\n\
            \x20 describe '#bar' do\n\
            \x20   subject { described_class.new }\n\
            \x20   it 'uses subject' do\n\
            \x20     expect(subject).to be_valid\n\
            \x20   end\n\
            \x20 end\n\
            \x20 describe '#baz' do\n\
            \x20   subject(:foo) { described_class.new }\n\
            \x20   it 'uses named' do\n\
            \x20     expect(foo).to be_valid\n\
            \x20   end\n\
            \x20 end\n\
            end\n";
        let diags = crate::testutil::run_cop_full_with_config(&NamedSubject, source, config);
        assert!(
            diags.is_empty(),
            "named_only should not flag when nearest subject is unnamed, got: {diags:?}"
        );
    }

    #[test]
    fn named_only_nearest_named_subject_is_flagged() {
        use crate::cop::CopConfig;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("named_only".into()),
            )]),
            ..CopConfig::default()
        };
        // Nearest subject definition is named — SHOULD be flagged
        let source = b"describe Foo do\n\
            \x20 subject(:foo) { described_class.new }\n\
            \x20 it 'uses subject' do\n\
            \x20   expect(subject).to be_valid\n\
            \x20 end\n\
            end\n";
        let diags = crate::testutil::run_cop_full_with_config(&NamedSubject, source, config);
        assert_eq!(
            diags.len(),
            1,
            "named_only should flag when nearest subject is named"
        );
    }
}
