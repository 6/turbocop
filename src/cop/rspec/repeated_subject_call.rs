use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/RepeatedSubjectCall: Flag calling `subject` multiple times in the same example
/// when at least one call is inside a block (expect { subject }).
///
/// ## FP fix (2026-03): align with RuboCop's `detect_offense` filtering
///
/// RuboCop only flags a repeated `subject` call when ALL three conditions hold:
/// 1. `subject` is NOT chained (`subject.foo` — skip, subject is a receiver)
/// 2. `subject`'s parent is NOT a send/call node (e.g., `expect(subject)` — skip,
///    subject is an argument)
/// 3. `subject` is inside an `expect { ... }` block (not just any block)
///
/// The previous implementation only tracked `in_block` (any block) and didn't
/// check condition 2. This caused 42 FPs from patterns like:
///   `expect(subject).to be_allowed(resource)` inside `.each` loops —
///   subject as an argument was incorrectly counted as a flaggable call.
///
/// ## FN fix (2026-03): track named subject aliases
///
/// RSpec allows `subject(:name) { ... }` which creates a method alias for `subject`.
/// The cop only matched literal `subject` calls but not named aliases. All 17 FNs
/// were from named subjects like `subject(:job)` where the example calls `job`.
/// Refactored from `check_node` to `check_source` with a visitor that collects
/// `subject(:name)` definitions per example group scope, then uses the alias set
/// when scanning examples for repeated calls.
///
/// ## FP fix (2026-03-20): always group subject calls by name
///
/// When multiple named subjects exist in scope (e.g., `subject(:metric)` at an outer
/// group and `subject(:track)` at an inner group), calls to different subject names
/// were being lumped together and counted as "repeated". For example, in:
///   `metric.track(value); expect { track }.to change { metric.values }`
/// the chained `metric` call (from `metric.values`) was counted as the "first" subject
/// call, making the bare `track` call appear as the "second" — triggering a false
/// positive even though `track` only appears once. Fixed by always grouping subject
/// calls by name (matching RuboCop's per-`method_name` tracking in
/// `detect_offenses_in_example`).
///
/// ## FN fix (2026-03-20): handle ConstantPathNode for subject usage
///
/// When a named subject is used as a constant path parent (e.g., `mod::Params` where
/// `mod` is `subject(:mod)`), the `mod` call is inside a `ConstantPathNode`. The
/// previous implementation only recursed into `CallNode` children, missing subject
/// calls nested inside constant paths. Added `ConstantPathNode` handling in
/// `collect_subject_calls` to recurse into the parent node. The subject call is NOT
/// marked as chained (it's a constant path parent, not a method chain receiver).
///
/// ## FN fix (2026-03-30): recurse through transparent wrappers and anchor multiline offenses
///
/// Corpus misses fell into three buckets:
/// 1. subject aliases defined inside transparent wrappers like `pending do` were not
///    inherited by nested groups/examples;
/// 2. nested examples inside `pending` / `shared_examples_for` were never visited;
/// 3. wrapped subject calls (`(subject)`, `subject rescue nil`) and multiline
///    `expect do ... end` blocks were either skipped or reported on the inner call line.
///
/// A separate corpus FP came from top-level group discovery: RuboCop's
/// `TopLevelGroup` mixin only unwraps `module` / `class` wrappers when they are
/// the entire root node. If a file has sibling top-level statements like
/// `require "rails_helper"` before a module-wrapped `RSpec.describe`, RuboCop
/// never reaches that spec group. nitrocop now matches that root-only
/// unwrapping behavior.
///
/// Fixed by:
/// - collecting subject aliases recursively through wrapper blocks while still
///   treating nested example groups as scope boundaries;
/// - recursing into nested examples/shared examples to find real examples inside
///   wrappers like `pending` and `shared_examples_for`;
/// - traversing `ParenthesesNode` / `RescueModifierNode` and preserving the outer
///   `expect` location for multiline blocks;
/// - matching RuboCop's top-level-group discovery quirk for module/class wrappers.
///
/// ## FN fix (2026-03-30): only direct call arguments are exempt
///
/// RuboCop exempts direct call arguments such as `expect(subject)` and
/// `create(subject)`, but it still flags repeated bare subject calls nested inside
/// wrapper nodes under an argument, such as keyword-hash values:
///   `expect { create(token: token) } ...`
/// The previous implementation marked every subject found anywhere in a call's
/// argument subtree as "argument to a call", suppressing valid offenses. The
/// traversal now marks only the immediate argument node, matching RuboCop.
pub struct RepeatedSubjectCall;

impl Cop for RepeatedSubjectCall {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedSubjectCall"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match parse_result.node().as_program_node() {
            Some(p) => p,
            None => return,
        };
        let body = program.statements();
        let top_level_nodes = collect_top_level_group_candidates_from_statements(&body);
        for node in top_level_nodes {
            process_top_level_group_candidate(
                source,
                &node,
                &[b"subject".to_vec()],
                diagnostics,
                self,
            );
        }
    }
}

/// Check if a call has an RSpec receiver (e.g., `RSpec.describe`).
fn is_rspec_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"RSpec";
        }
        if let Some(cp) = recv.as_constant_path_node() {
            if cp.parent().is_none() {
                if let Some(name) = cp.name() {
                    return name.as_slice() == b"RSpec";
                }
            }
        }
    }
    false
}

/// Match RuboCop's TopLevelGroup mixin:
/// - if the root is a module/class wrapper, unwrap it;
/// - if the root is a begin/statements node, inspect only its direct children.
fn collect_top_level_group_candidates_from_statements<'a>(
    statements: &ruby_prism::StatementsNode<'a>,
) -> Vec<ruby_prism::Node<'a>> {
    let body: Vec<_> = statements.body().iter().collect();
    if body.len() == 1 {
        return collect_top_level_group_candidates_from_node(body.into_iter().next().unwrap());
    }

    body
}

fn collect_top_level_group_candidates_from_node<'a>(
    node: ruby_prism::Node<'a>,
) -> Vec<ruby_prism::Node<'a>> {
    if let Some(module_node) = node.as_module_node() {
        if let Some(statements) = module_node
            .body()
            .and_then(|body| body.as_statements_node())
        {
            return collect_top_level_group_candidates_from_statements(&statements);
        }
        return Vec::new();
    }

    if let Some(class_node) = node.as_class_node() {
        if let Some(statements) = class_node.body().and_then(|body| body.as_statements_node()) {
            return collect_top_level_group_candidates_from_statements(&statements);
        }
        return Vec::new();
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(statements) = begin_node.statements() {
            return statements.body().iter().collect();
        }
        return Vec::new();
    }

    vec![node]
}

/// Process a top-level candidate that may be an RSpec example group.
fn process_top_level_group_candidate(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    inherited_subjects: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &RepeatedSubjectCall,
) {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_eg = (call.receiver().is_none() && is_rspec_example_group(name))
            || (is_rspec_receiver(&call) && is_rspec_example_group(name));

        if is_eg {
            if let Some(block) = call.block() {
                if let Some(bn) = block.as_block_node() {
                    process_example_group(source, &bn, inherited_subjects, diagnostics, cop);
                }
            }
        }
    }
}

/// Process an example-group-like block: collect subject names, check examples,
/// recurse into nested groups/shared examples/examples inside transparent wrappers.
fn process_example_group(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
    inherited_subjects: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &RepeatedSubjectCall,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    if body.as_statements_node().is_none() {
        return;
    }

    // Collect subject names defined anywhere in this example group's scope,
    // including transparent wrappers like `pending do`, but not nested example
    // groups (their subjects belong to the nested group).
    let mut subject_names: Vec<Vec<u8>> = inherited_subjects.to_vec();
    collect_subject_names_in_group_scope(&body, &mut subject_names);
    process_rspec_blocks_in_body(source, &body, &subject_names, diagnostics, cop);
}

/// Check an example body for repeated subject calls.
///
/// Always groups calls by subject name (matching RuboCop's per-method_name tracking
/// in `detect_offenses_in_example`). This prevents false positives when multiple
/// named subjects coexist (e.g., `subject(:metric)` and `subject(:track)`) — a
/// chained `metric.values` call should not count as a repeat of `track`.
fn check_example_body(
    source: &SourceFile,
    body: &ruby_prism::Node<'_>,
    subject_names: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &RepeatedSubjectCall,
) {
    let mut subject_calls: Vec<SubjectCall> = Vec::new();
    collect_subject_calls(
        source,
        body,
        SubjectCallContext::default(),
        subject_names,
        &mut subject_calls,
    );

    if subject_calls.len() <= 1 {
        return;
    }

    // Group calls by name and check each group independently.
    // RuboCop uses `subjects_used[call.method_name]` which is inherently per-name.
    let unique_names: Vec<&[u8]> = {
        let mut names: Vec<&[u8]> = subject_calls.iter().map(|c| c.name.as_slice()).collect();
        names.sort_unstable();
        names.dedup();
        names
    };
    for name in unique_names {
        let calls_for_name: Vec<&SubjectCall> =
            subject_calls.iter().filter(|c| c.name == name).collect();
        if calls_for_name.len() <= 1 {
            continue;
        }
        flag_repeated_calls(&calls_for_name, source, diagnostics, cop);
    }
}

/// Flag the 2nd+ subject calls that are in expect blocks and not chained/args.
fn flag_repeated_calls(
    calls: &[&SubjectCall],
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &RepeatedSubjectCall,
) {
    let mut seen_first = false;
    for call in calls {
        if !seen_first {
            seen_first = true;
            continue;
        }
        if call.in_expect_block && !call.is_chained && !call.is_arg_of_call {
            diagnostics.push(cop.diagnostic(
                source,
                call.diagnostic_line,
                call.diagnostic_col,
                "Calls to subject are memoized, this block is misleading".to_string(),
            ));
        }
    }
}

fn collect_subject_names_in_group_scope(
    node: &ruby_prism::Node<'_>,
    subject_names: &mut Vec<Vec<u8>>,
) {
    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            collect_subject_names_in_group_scope(&child, subject_names);
        }
        return;
    }

    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();

        // Nested example groups get their own subject scope.
        if call.receiver().is_none() && is_rspec_example_group(name) {
            return;
        }

        if (name == b"subject" || name == b"subject!") && call.receiver().is_none() {
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if let Some(sym) = arg_list.first().and_then(|arg| arg.as_symbol_node()) {
                    let alias = sym.unescaped().to_vec();
                    if !subject_names.contains(&alias) {
                        subject_names.push(alias);
                    }
                }
            }
        }

        if let Some(recv) = call.receiver() {
            collect_subject_names_in_group_scope(&recv, subject_names);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_subject_names_in_group_scope(&arg, subject_names);
            }
        }
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    collect_subject_names_in_group_scope(&body, subject_names);
                }
            }
        }
        return;
    }

    if let Some(hash) = node.as_hash_node() {
        for element in hash.elements().iter() {
            collect_subject_names_in_group_scope(&element, subject_names);
        }
        return;
    }
    if let Some(hash) = node.as_keyword_hash_node() {
        for element in hash.elements().iter() {
            collect_subject_names_in_group_scope(&element, subject_names);
        }
        return;
    }
    if let Some(assoc) = node.as_assoc_node() {
        collect_subject_names_in_group_scope(&assoc.key(), subject_names);
        collect_subject_names_in_group_scope(&assoc.value(), subject_names);
        return;
    }
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            collect_subject_names_in_group_scope(&body, subject_names);
        }
        return;
    }
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            collect_subject_names_in_group_scope(&stmts.as_node(), subject_names);
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            collect_subject_names_in_group_scope(&rescue_clause.as_node(), subject_names);
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            collect_subject_names_in_group_scope(&ensure_clause.as_node(), subject_names);
        }
        return;
    }
    if let Some(rescue_node) = node.as_rescue_node() {
        if let Some(stmts) = rescue_node.statements() {
            collect_subject_names_in_group_scope(&stmts.as_node(), subject_names);
        }
        if let Some(subsequent) = rescue_node.subsequent() {
            collect_subject_names_in_group_scope(&subsequent.as_node(), subject_names);
        }
        return;
    }
    if let Some(rescue_mod) = node.as_rescue_modifier_node() {
        collect_subject_names_in_group_scope(&rescue_mod.expression(), subject_names);
        collect_subject_names_in_group_scope(&rescue_mod.rescue_expression(), subject_names);
        return;
    }
    if let Some(parentheses) = node.as_parentheses_node() {
        if let Some(body) = parentheses.body() {
            collect_subject_names_in_group_scope(&body, subject_names);
        }
        return;
    }
    if let Some(array) = node.as_array_node() {
        for element in array.elements().iter() {
            collect_subject_names_in_group_scope(&element, subject_names);
        }
    }
}

fn process_rspec_blocks_in_body(
    source: &SourceFile,
    body: &ruby_prism::Node<'_>,
    subject_names: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &RepeatedSubjectCall,
) {
    let Some(stmts) = body.as_statements_node() else {
        return;
    };

    for stmt in stmts.body().iter() {
        let Some(call) = stmt.as_call_node() else {
            continue;
        };

        if call.receiver().is_some() {
            continue;
        }

        let call_name = call.name().as_slice();
        let Some(block) = call.block().and_then(|b| b.as_block_node()) else {
            continue;
        };
        let Some(block_body) = block.body() else {
            continue;
        };

        if is_rspec_example(call_name) {
            check_example_body(source, &block_body, subject_names, diagnostics, cop);
            continue;
        }

        if is_rspec_example_group(call_name) || is_rspec_shared_group(call_name) {
            process_example_group(source, &block, subject_names, diagnostics, cop);
        }
    }
}

struct SubjectCall {
    name: Vec<u8>,
    diagnostic_line: usize,
    diagnostic_col: usize,
    in_expect_block: bool,
    is_chained: bool,
    is_arg_of_call: bool,
}

#[derive(Clone, Copy, Default)]
struct SubjectCallContext {
    in_expect_block: bool,
    expect_anchor: Option<(usize, usize)>,
    direct_receiver: bool,
    direct_arg_of_call: bool,
}

impl SubjectCallContext {
    fn with_receiver(self) -> Self {
        Self {
            direct_receiver: true,
            direct_arg_of_call: false,
            ..self
        }
    }

    fn with_arg(self) -> Self {
        Self {
            direct_receiver: false,
            direct_arg_of_call: true,
            ..self
        }
    }

    fn reset_position(self) -> Self {
        Self {
            direct_receiver: false,
            direct_arg_of_call: false,
            ..self
        }
    }

    fn enter_block(self, anchor: Option<(usize, usize)>) -> Self {
        Self {
            in_expect_block: self.in_expect_block || anchor.is_some(),
            expect_anchor: self.expect_anchor.or(anchor),
            direct_receiver: false,
            direct_arg_of_call: false,
        }
    }
}

fn collect_subject_calls(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    context: SubjectCallContext,
    subject_names: &[Vec<u8>],
    results: &mut Vec<SubjectCall>,
) {
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_subject_calls(
                source,
                &stmt,
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    // Handle ConstantPathNode (e.g., `mod::Params`) — the parent of a constant path
    // may be a subject call (e.g., `mod` is a named subject). RuboCop's recursive
    // `def_node_search :subject_calls` finds `(send nil? %)` inside constant paths.
    // The subject call here is NOT chained (parent is a const, not a send).
    if let Some(const_path) = node.as_constant_path_node() {
        if let Some(parent) = const_path.parent() {
            collect_subject_calls(
                source,
                &parent,
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(parentheses) = node.as_parentheses_node() {
        if let Some(body) = parentheses.body() {
            collect_subject_calls(
                source,
                &body,
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(hash) = node.as_hash_node() {
        for element in hash.elements().iter() {
            collect_subject_calls(
                source,
                &element,
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        for element in hash.elements().iter() {
            collect_subject_calls(
                source,
                &element,
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(assoc) = node.as_assoc_node() {
        collect_subject_calls(
            source,
            &assoc.key(),
            context.reset_position(),
            subject_names,
            results,
        );
        collect_subject_calls(
            source,
            &assoc.value(),
            context.reset_position(),
            subject_names,
            results,
        );
        return;
    }

    if let Some(array) = node.as_array_node() {
        for element in array.elements().iter() {
            collect_subject_calls(
                source,
                &element,
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            collect_subject_calls(
                source,
                &stmts.as_node(),
                context.reset_position(),
                subject_names,
                results,
            );
        }
        if let Some(rescue_clause) = begin_node.rescue_clause() {
            collect_subject_calls(
                source,
                &rescue_clause.as_node(),
                context.reset_position(),
                subject_names,
                results,
            );
        }
        if let Some(ensure_clause) = begin_node.ensure_clause() {
            collect_subject_calls(
                source,
                &ensure_clause.as_node(),
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(rescue_node) = node.as_rescue_node() {
        if let Some(stmts) = rescue_node.statements() {
            collect_subject_calls(
                source,
                &stmts.as_node(),
                context.reset_position(),
                subject_names,
                results,
            );
        }
        if let Some(subsequent) = rescue_node.subsequent() {
            collect_subject_calls(
                source,
                &subsequent.as_node(),
                context.reset_position(),
                subject_names,
                results,
            );
        }
        return;
    }

    if let Some(rescue_mod) = node.as_rescue_modifier_node() {
        collect_subject_calls(
            source,
            &rescue_mod.expression(),
            context.reset_position(),
            subject_names,
            results,
        );
        collect_subject_calls(
            source,
            &rescue_mod.rescue_expression(),
            context.reset_position(),
            subject_names,
            results,
        );
        return;
    }

    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();

        // Check if this is a bare subject call (no receiver) matching any known subject name.
        if call.receiver().is_none() && subject_names.iter().any(|s| s == name) {
            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            let (diagnostic_line, diagnostic_col) =
                if let Some((expect_line, expect_col)) = context.expect_anchor {
                    if context.in_expect_block && expect_line != line {
                        (expect_line, expect_col)
                    } else {
                        (line, col)
                    }
                } else {
                    (line, col)
                };
            results.push(SubjectCall {
                name: name.to_vec(),
                diagnostic_line,
                diagnostic_col,
                in_expect_block: context.in_expect_block,
                is_chained: context.direct_receiver,
                is_arg_of_call: context.direct_arg_of_call,
            });
        }

        // Recurse into receiver, marking it as a receiver context
        if let Some(recv) = call.receiver() {
            collect_subject_calls(
                source,
                &recv,
                context.with_receiver(),
                subject_names,
                results,
            );
        }

        // Only direct call arguments are exempt. Nested wrappers like
        // keyword-hash values are handled by resetting the direct-arg flag when
        // descending through non-call container nodes.
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_subject_calls(source, &arg, context.with_arg(), subject_names, results);
            }
        }

        // Check the call's own block — track if this is an `expect` block
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    let expect_anchor = if call.receiver().is_none() && name == b"expect" {
                        let loc = call.location();
                        Some(source.offset_to_line_col(loc.start_offset()))
                    } else {
                        None
                    };
                    collect_subject_calls(
                        source,
                        &body,
                        context.enter_block(expect_anchor),
                        subject_names,
                        results,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedSubjectCall, "cops/rspec/repeated_subject_call");
}
