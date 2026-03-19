use ruby_prism::Visit;

use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/PendingWithoutReason
///
/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=186, FN=35.
///
/// FP=186 root cause: we flagged bare `pending`/`skip` calls without checking
/// RuboCop's parent context, so constructs like `next skip` inside examples
/// were incorrectly reported.
///
/// FN=35 root cause: we required blocks for x-prefixed skipped methods
/// (`xit`, `xdescribe`, etc.), but RuboCop flags no-arg forms too.
///
/// Fix: implement RuboCop-aligned parent-context logic:
/// - In spec-group context, flag no-arg skipped/pending methods and block forms.
/// - In example context, flag only no-arg skipped/pending calls.
/// - Flag skipped example-group methods with "skip" message, including
///   top-level explicit `RSpec.xdescribe`.
///
/// ## Corpus investigation (2026-03-18)
///
/// Corpus oracle reported FP=0, FN=7. All 7 FN are top-level `RSpec.xdescribe`
/// calls (no enclosing spec group block). The SKIPPED_GROUP_METHODS check
/// required `ParentContext::SpecGroup`, but top-level calls have `Other` context.
///
/// Fix: mirror RuboCop's `block_node_example_group?` logic — also flag
/// SKIPPED_GROUP_METHODS when the call itself has an explicit `RSpec` receiver
/// and a block, regardless of parent context.
pub struct PendingWithoutReason;

/// Skipped example-group methods (`ExampleGroups::Skipped` in RuboCop).
const SKIPPED_GROUP_METHODS: &[&[u8]] = &[b"xdescribe", b"xcontext", b"xfeature"];

/// Skipped/pending example methods (`Examples::Skipped` + `Examples::Pending` in RuboCop).
const SKIPPED_OR_PENDING_EXAMPLE_METHODS: &[&[u8]] = &[
    b"skip",
    b"pending",
    b"xit",
    b"xspecify",
    b"xexample",
    b"xscenario",
];

/// x-prefixed methods used by this cop that should report `skip` as label.
const SKIP_LABEL_METHODS: &[&[u8]] = &[b"xcontext", b"xdescribe", b"xfeature"];

impl Cop for PendingWithoutReason {
    fn name(&self) -> &'static str {
        "RSpec/PendingWithoutReason"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = PendingWithoutReasonVisitor {
            source,
            cop: self,
            diagnostics,
            ancestors: Vec::new(),
        };
        visitor.visit(&parse_result.node());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParentContext {
    SpecGroup,
    Example,
    Other,
}

struct PendingWithoutReasonVisitor<'a, 'pr> {
    source: &'a SourceFile,
    cop: &'a PendingWithoutReason,
    diagnostics: &'a mut Vec<Diagnostic>,
    ancestors: Vec<ruby_prism::Node<'pr>>,
}

impl<'a, 'pr> PendingWithoutReasonVisitor<'a, 'pr> {
    fn parent_context(&self) -> ParentContext {
        // Current node is the last ancestor (a CallNode while in visit_call_node).
        let Some(mut idx) = self.ancestors.len().checked_sub(2) else {
            return ParentContext::Other;
        };

        // RuboCop un-wraps begin wrappers while finding parent context.
        while self.ancestors[idx].as_statements_node().is_some()
            || is_transparent_begin(&self.ancestors[idx])
        {
            let Some(next_idx) = idx.checked_sub(1) else {
                return ParentContext::Other;
            };
            idx = next_idx;
        }

        let Some(_parent_block) = self.ancestors[idx].as_block_node() else {
            return ParentContext::Other;
        };

        let Some(call_idx) = idx.checked_sub(1) else {
            return ParentContext::Other;
        };
        let Some(enclosing_call) = self.ancestors[call_idx].as_call_node() else {
            return ParentContext::Other;
        };

        if is_spec_group_call(&enclosing_call) {
            ParentContext::SpecGroup
        } else if is_example_call(&enclosing_call) {
            ParentContext::Example
        } else {
            ParentContext::Other
        }
    }
}

impl<'a, 'pr> Visit<'pr> for PendingWithoutReasonVisitor<'a, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.ancestors.push(node);
    }

    fn visit_branch_node_leave(&mut self) {
        self.ancestors.pop();
    }

    fn visit_leaf_node_enter(&mut self, _node: ruby_prism::Node<'pr>) {}

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        if is_metadata_target_call(node) {
            if let Some(label) = metadata_without_reason_label(node) {
                add_reason_offense(self.cop, self.source, self.diagnostics, node, label);
            }
        }

        let context = self.parent_context();
        let no_args = node.arguments().is_none();
        let has_block = node.block().and_then(|b| b.as_block_node()).is_some();

        // RuboCop: in spec-group context, flag no-arg skipped/pending calls and block forms.
        if node.receiver().is_none()
            && context == ParentContext::SpecGroup
            && SKIPPED_OR_PENDING_EXAMPLE_METHODS.contains(&method_name)
            && (no_args || has_block)
        {
            let label = method_label(method_name);
            add_reason_offense(self.cop, self.source, self.diagnostics, node, label);
        }

        // RuboCop: in example context, flag only no-arg skipped/pending calls.
        if node.receiver().is_none()
            && context == ParentContext::Example
            && SKIPPED_OR_PENDING_EXAMPLE_METHODS.contains(&method_name)
            && no_args
        {
            let label = method_label(method_name);
            add_reason_offense(self.cop, self.source, self.diagnostics, node, label);
        }

        // RuboCop: skipped example-group methods report "skip".
        // Flagged when inside a spec group OR when the call itself is an
        // example group with an explicit RSpec receiver and block (top-level
        // `RSpec.xdescribe` case — mirrors RuboCop's `block_node_example_group?`).
        if SKIPPED_GROUP_METHODS.contains(&method_name)
            && has_rspec_receiver(node.receiver())
            && (context == ParentContext::SpecGroup
                || (has_block && is_explicit_rspec_receiver(node.receiver())))
        {
            add_reason_offense(self.cop, self.source, self.diagnostics, node, "skip");
        }

        ruby_prism::visit_call_node(self, node);
    }
}

fn is_spec_group_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block().and_then(|b| b.as_block_node()).is_some()
        && has_rspec_receiver(call.receiver())
        && is_rspec_example_group(call.name().as_slice())
}

fn is_example_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block().and_then(|b| b.as_block_node()).is_some()
        && call.receiver().is_none()
        && is_rspec_example(call.name().as_slice())
}

fn is_non_shared_example_group_method(name: &[u8]) -> bool {
    is_rspec_example_group(name) && !util::is_rspec_shared_group(name)
}

fn is_explicit_rspec_receiver(receiver: Option<ruby_prism::Node<'_>>) -> bool {
    receiver.is_some_and(|recv| util::constant_name(&recv).is_some_and(|n| n == b"RSpec"))
}

fn has_rspec_receiver(receiver: Option<ruby_prism::Node<'_>>) -> bool {
    receiver.is_none() || is_explicit_rspec_receiver(receiver)
}

fn is_transparent_begin(node: &ruby_prism::Node<'_>) -> bool {
    let Some(begin_node) = node.as_begin_node() else {
        return false;
    };
    begin_node.begin_keyword_loc().is_none()
        && begin_node.rescue_clause().is_none()
        && begin_node.else_clause().is_none()
        && begin_node.ensure_clause().is_none()
}

fn is_metadata_target_call(call: &ruby_prism::CallNode<'_>) -> bool {
    has_rspec_receiver(call.receiver())
        && (is_non_shared_example_group_method(call.name().as_slice())
            || is_rspec_example(call.name().as_slice()))
}

fn metadata_without_reason_label(call: &ruby_prism::CallNode<'_>) -> Option<&'static str> {
    let args = call.arguments()?;
    for arg in args.arguments().iter() {
        if let Some(sym) = arg.as_symbol_node() {
            let val = sym.unescaped();
            if val == b"skip" {
                return Some("skip");
            }
            if val == b"pending" {
                return Some("pending");
            }
        }

        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                let Some(assoc) = elem.as_assoc_node() else {
                    continue;
                };
                let Some(key_sym) = assoc.key().as_symbol_node() else {
                    continue;
                };
                let key = key_sym.unescaped();
                if (key == b"skip" || key == b"pending") && assoc.value().as_true_node().is_some() {
                    if key == b"skip" {
                        return Some("skip");
                    }
                    return Some("pending");
                }
            }
        }
    }
    None
}

fn method_label(method_name: &[u8]) -> &str {
    if SKIP_LABEL_METHODS.contains(&method_name) {
        "skip"
    } else {
        std::str::from_utf8(method_name).unwrap_or("skip")
    }
}

fn add_reason_offense(
    cop: &PendingWithoutReason,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    call: &ruby_prism::CallNode<'_>,
    label: &str,
) {
    let loc = call.location();
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Give the reason for {label}."),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PendingWithoutReason, "cops/rspec/pending_without_reason");
}
