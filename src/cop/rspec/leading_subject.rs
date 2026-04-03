use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::PROGRAM_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_hook, is_rspec_let,
    is_rspec_subject,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/LeadingSubject checks that `subject` is declared before `let`, hooks,
/// examples, and other declarations within an example group.
///
/// RuboCop uses `InsideExampleGroup` to determine whether a `subject` node
/// should be checked. This check walks up to the file's root-level node and
/// verifies it is a spec group (describe/context/shared_examples block). When
/// the describe block is wrapped in a `module` or `class` declaration, the
/// root-level node is the module/class — NOT a spec group — so RuboCop skips
/// the cop entirely. This is a documented side-effect of `InsideExampleGroup`.
///
/// We replicate this by only checking subjects inside spec groups that are
/// at the file's top level (direct children of the program node, or within a
/// top-level `begin`). Spec groups inside module/class wrappers are skipped.
///
/// ## Investigation (2026-03-11)
///
/// **Root cause of 118 FNs:** Two issues found:
///
/// 1. Include-family blocks (it_behaves_like, include_context, include_examples,
///    it_should_behave_like) were not recursed into. RuboCop's `on_block` fires
///    on ALL blocks, so subjects inside `it_behaves_like do...end` are checked
///    independently for ordering within that block. The nitrocop code only
///    recursed into example group blocks (describe/context/shared_examples).
///    Fixed by adding `recurse_into_block()` for include-family calls.
///
/// 2. `RSpec.describe` nested inside another example group was recursed into but
///    NOT treated as an offending node (the `continue` after recursion skipped
///    the `first_relevant_name` update). RuboCop's `spec_group?` includes
///    `RSpec.describe`, so it IS offending. Fixed by setting `first_relevant_name`
///    for `RSpec.describe` calls.
///
/// ## Investigation (2026-03-15)
///
/// **Root cause of 84 FNs:** RuboCop's `on_block` fires on ALL blocks, not just
/// example groups and include-family blocks. The `parent(node)` method gets the
/// immediate block ancestor, so subjects inside arbitrary blocks (custom DSL
/// methods like `with_feature_flag do...end`, `around do...end` etc.) are
/// checked independently for ordering within that block. The nitrocop code only
/// recursed into example group and include-family blocks, missing subjects
/// inside arbitrary blocks. Fixed by recursing into ALL call nodes with blocks
/// that are children of an example group body.
///
/// ## Investigation (2026-03-18)
///
/// **Root cause of 72 FNs:** Two issues found:
///
/// 1. `is_spec_group_call()` at the top level only matched `RSpec.describe` for
///    receiver calls, missing `RSpec.shared_examples_for`, `RSpec.shared_context`,
///    `RSpec.feature`, etc. Many corpus files use `RSpec.shared_examples_for` or
///    `RSpec.shared_context` at the top level, so subjects in those blocks were
///    never checked. Fixed by matching all `RSpec.<example_group>` methods.
///
/// 2. Calls with receivers (e.g. `items.each do...end`, `hash.each_pair do...end`)
///    were completely skipped during recursion (`continue` after the
///    `RSpec.describe` check). Subjects inside iterator blocks that contain
///    nested `context`/`describe` blocks were missed. Fixed by recursing into
///    the block body of any receiver call that has a block, matching RuboCop's
///    `on_block` behavior that fires on ALL blocks.
///
/// ## Verification (2026-03-18)
///
/// Manual verification against locally available corpus repos (avo-hq, openproject,
/// diaspora) confirms all 72 FN examples from the CI oracle are now detected by the
/// current code. Patterns verified include:
/// - `include_context` without block before subject (diaspora mentioning_spec)
/// - Subject inside `.each` iterator block with destructured args (openproject users_helper)
/// - Named subject `subject(:name)` after `let` with intervening `def` method (openproject attachment_resource)
/// - `it_behaves_like` with block before subject at same level (openproject attachment_resource)
/// - Subject inside `RSpec.shared_examples_for` after `let` (openproject response_examples)
/// - `shared_let` (custom DSL, not offending) followed by `include_context` + `subject`
///
/// The commit c0bc7a5 estimated "fixes 43 of 72 (29 remain)" but actual verification
/// shows all 72 patterns are handled. The "29 remain" was a conservative estimate;
/// the CI oracle simply hasn't re-run to confirm.
///
/// ## Corpus investigation (2026-04-01)
///
/// Corpus oracle reported FP=50, FN=0.
///
/// FP=50: All from bare `subject` method calls inside `def` bodies (e.g.,
/// `def app; subject; end` in grape specs). These are method references, not
/// subject declarations. RuboCop's `on_block` only fires on blocks, so bare
/// `subject` calls are never checked. Fixed by requiring `c.block().is_some()`
/// on subject calls in both `check_block_body` and
/// `scan_non_block_container_statements`.
///
/// ## Investigation (2026-03-20)
///
/// **Root cause of 3 FNs:** `if`/`unless` control flow nodes wrapping spec groups
/// (e.g., `if linux?`, `unless ENV["CI"]`) were not traversed during block body
/// iteration. The cop only looked at `CallNode` children, so `describe`/`context`
/// blocks inside conditionals were invisible. Fixed by adding
/// `recurse_into_conditional()` which walks `IfNode`/`UnlessNode` bodies (including
/// elsif/else branches) and recurses into any block-bearing call nodes found within.
/// All 3 FN repos (guard/listen, bunny, vcr) use this pattern.
///
/// ## Investigation (2026-03-29)
///
/// **Root cause of 5 FNs:** RuboCop compares nested `subject` calls inside
/// non-block containers like `if` branches and `def self.helper` bodies against
/// the enclosing example-group body, not the container body itself. Its
/// `parent(node).each_child_node` walk never sees the nested `subject` as a
/// direct child, so it keeps scanning the example-group body and reports the
/// first direct-child offender (usually `let`) even when it appears later.
/// nitrocop only checked ordering within the conditional/method body, so these
/// nested subjects were missed. Fixed by scanning `if`/`unless`/`else` and
/// `DefNode` bodies for nested `subject` calls and reporting them against the
/// enclosing block body's first direct offending declaration, while still
/// recursing into nested block scopes normally.
///
/// ## Investigation (2026-04-01)
///
/// **Root cause of 3 FPs:** Two issues:
///
/// 1. Hooks with block_pass (e.g. `around(&rspec_around)`) were treated as
///    offending. Prism's `CallNode::block()` returns both `BlockNode` (real
///    do/end or {}) and `BlockArgumentNode` (&proc). RuboCop's `hook?` and
///    `example?` only match real block nodes, so `around(&proc)` is NOT
///    offending. Fixed by checking `b.as_block_node().is_some()` instead of
///    just `c.block().is_some()` for hooks and examples.
///
/// 2. `describe [attr]` (call without a block) was treated as offending.
///    RuboCop's `spec_group?` requires a block node, so bare example group
///    calls without blocks are NOT offending. Fixed by adding block check
///    before setting `first_relevant_name` for example group calls.
pub struct LeadingSubject;

impl Cop for LeadingSubject {
    fn name(&self) -> &'static str {
        "RSpec/LeadingSubject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let program = match node.as_program_node() {
            Some(p) => p,
            None => return,
        };

        // Walk top-level statements looking for spec groups.
        // Only spec groups at the file root (not inside module/class) are checked,
        // matching RuboCop's InsideExampleGroup behavior.
        for stmt in program.statements().body().iter() {
            if is_spec_group_call(&stmt) {
                self.check_block_body(source, &stmt, diagnostics);
            }
            // Skip modules, classes, requires, and anything else at the top level.
        }
    }
}

impl LeadingSubject {
    /// Check subject ordering within a block body and recurse into child blocks.
    /// This is the unified handler for example groups, include-family blocks,
    /// and arbitrary blocks — matching RuboCop's `on_block` behavior which fires
    /// on ALL blocks and uses `parent(node)` to check ordering within the
    /// immediate parent block.
    fn check_block_body(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let first_nested_relevant_name = first_direct_offending_name(&stmts);

        // Check subject ordering within this block
        let mut first_relevant_name: Option<&[u8]> = None;

        for stmt in stmts.body().iter() {
            // Handle non-block containers: RuboCop compares nested `subject`
            // calls inside these containers against the enclosing example-group
            // body, not the container body itself.
            if stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_def_node().is_some()
            {
                self.recurse_into_non_block_container(
                    source,
                    &stmt,
                    first_nested_relevant_name,
                    diagnostics,
                );
                continue;
            }

            if let Some(c) = stmt.as_call_node() {
                let name = c.name().as_slice();

                // Handle calls with receiver (e.g. RSpec.describe, items.each)
                if c.receiver().is_some() {
                    let is_rspec_group =
                        constant_predicates::constant_short_name(&c.receiver().unwrap())
                            .is_some_and(|n| n == b"RSpec")
                            && is_rspec_example_group(name);
                    if is_rspec_group {
                        // Recurse into RSpec.describe / RSpec.shared_examples_for / etc.
                        self.check_block_body(source, &stmt, diagnostics);
                        // Also treat as offending (spec_group in RuboCop), but only
                        // when there's a real block — matching spec_group? behavior.
                        if c.block().is_some_and(|b| b.as_block_node().is_some())
                            && first_relevant_name.is_none()
                        {
                            first_relevant_name = Some(name);
                        }
                    } else if c.block().is_some() {
                        // Arbitrary receiver calls with blocks (e.g. items.each do...end)
                        // must be recursed into to find subjects in nested scopes,
                        // matching RuboCop's on_block behavior.
                        self.check_block_body(source, &stmt, diagnostics);
                    }
                    continue;
                }

                if is_rspec_subject(name) && c.block().is_some() {
                    // Subject declaration (with block) -- check if something relevant came before it.
                    // Bare `subject` calls without a block are method references, not declarations.
                    if let Some(prev_name) = first_relevant_name {
                        self.add_subject_offense(source, &stmt, prev_name, diagnostics);
                    }
                } else if is_rspec_example_group(name) {
                    // Recurse into nested context/describe/shared_examples blocks.
                    // RuboCop's spec_group? requires a real block node, so only
                    // treat as offending when the call has a block.
                    self.check_block_body(source, &stmt, diagnostics);
                    if c.block().is_some_and(|b| b.as_block_node().is_some())
                        && first_relevant_name.is_none()
                    {
                        first_relevant_name = Some(name);
                    }
                } else if is_example_include(name) {
                    // Recurse into include-family blocks; also treat as offending
                    self.check_block_body(source, &stmt, diagnostics);
                    if first_relevant_name.is_none() {
                        first_relevant_name = Some(name);
                    }
                } else if is_rspec_let(name) {
                    // RuboCop's let? requires a block or block_pass:
                    //   (block (send nil? #Helpers.all ...) ...)
                    //   (send nil? #Helpers.all _ block_pass)
                    let has_block = c.block().is_some();
                    let has_block_pass = c.arguments().is_some_and(|args| {
                        args.arguments()
                            .iter()
                            .any(|a| a.as_block_argument_node().is_some())
                    });
                    if has_block {
                        self.check_block_body(source, &stmt, diagnostics);
                    }
                    if (has_block || has_block_pass) && first_relevant_name.is_none() {
                        first_relevant_name = Some(name);
                    }
                } else if is_rspec_hook(name) || is_rspec_example(name) {
                    // RuboCop's hook? and example? require a real block (do/end or {}),
                    // not a block_pass (&proc). Prism's block() returns both BlockNode
                    // and BlockArgumentNode, so we must check for BlockNode specifically.
                    let has_real_block = c.block().is_some_and(|b| b.as_block_node().is_some());
                    if has_real_block {
                        self.check_block_body(source, &stmt, diagnostics);
                        if first_relevant_name.is_none() {
                            first_relevant_name = Some(name);
                        }
                    }
                } else if c.block().is_some() {
                    // Arbitrary block-bearing calls (custom DSL methods, etc.)
                    // are NOT offending but we must recurse into their blocks
                    // to check subject ordering within, matching RuboCop's
                    // on_block behavior that fires on ALL blocks.
                    self.check_block_body(source, &stmt, diagnostics);
                }
            }
        }
    }

    fn add_subject_offense(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        prev_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let prev_str = std::str::from_utf8(prev_name).unwrap_or("let");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Declare `subject` above any other `{prev_str}` declarations."),
        ));
    }

    /// Recurse into non-block containers that can hold nested `subject` calls
    /// while still belonging to the enclosing example-group body.
    fn recurse_into_non_block_container(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parent_first_relevant_name: Option<&[u8]>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                self.scan_non_block_container_statements(
                    source,
                    &stmts,
                    parent_first_relevant_name,
                    diagnostics,
                );
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.recurse_into_non_block_container(
                    source,
                    &subsequent,
                    parent_first_relevant_name,
                    diagnostics,
                );
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                self.scan_non_block_container_statements(
                    source,
                    &stmts,
                    parent_first_relevant_name,
                    diagnostics,
                );
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    self.scan_non_block_container_statements(
                        source,
                        &stmts,
                        parent_first_relevant_name,
                        diagnostics,
                    );
                }
            }
        } else if let Some(else_node) = node.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                self.scan_non_block_container_statements(
                    source,
                    &stmts,
                    parent_first_relevant_name,
                    diagnostics,
                );
            }
        } else if let Some(def_node) = node.as_def_node() {
            if let Some(body) = def_node.body().and_then(|b| b.as_statements_node()) {
                self.scan_non_block_container_statements(
                    source,
                    &body,
                    parent_first_relevant_name,
                    diagnostics,
                );
            }
        }
    }

    fn scan_non_block_container_statements(
        &self,
        source: &SourceFile,
        stmts: &ruby_prism::StatementsNode<'_>,
        parent_first_relevant_name: Option<&[u8]>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for stmt in stmts.body().iter() {
            if stmt.as_if_node().is_some()
                || stmt.as_unless_node().is_some()
                || stmt.as_def_node().is_some()
            {
                self.recurse_into_non_block_container(
                    source,
                    &stmt,
                    parent_first_relevant_name,
                    diagnostics,
                );
                continue;
            }

            if let Some(c) = stmt.as_call_node() {
                if is_rspec_subject(c.name().as_slice()) && c.block().is_some() {
                    if let Some(prev_name) = parent_first_relevant_name {
                        self.add_subject_offense(source, &stmt, prev_name, diagnostics);
                    }
                } else if c.block().is_some() {
                    self.check_block_body(source, &stmt, diagnostics);
                }
            }
        }
    }
}

fn is_spec_group_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    if let Some(recv) = call.receiver() {
        // RSpec.describe, RSpec.shared_examples_for, RSpec.shared_context, RSpec.feature, etc.
        constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
            && is_rspec_example_group(name)
    } else {
        is_rspec_example_group(name)
    }
}

fn is_example_include(name: &[u8]) -> bool {
    name == b"include_examples"
        || name == b"it_behaves_like"
        || name == b"it_should_behave_like"
        || name == b"include_context"
}

fn first_direct_offending_name<'pr>(stmts: &ruby_prism::StatementsNode<'pr>) -> Option<&'pr [u8]> {
    stmts
        .body()
        .iter()
        .find_map(|node| direct_offending_name(&node))
}

fn direct_offending_name<'pr>(node: &ruby_prism::Node<'pr>) -> Option<&'pr [u8]> {
    let call = node.as_call_node()?;
    let name = call.name().as_slice();

    if let Some(recv) = call.receiver() {
        let is_rspec_group = constant_predicates::constant_short_name(&recv)
            .is_some_and(|n| n == b"RSpec")
            && is_rspec_example_group(name)
            && call.block().is_some_and(|b| b.as_block_node().is_some());
        return is_rspec_group.then_some(name);
    }

    if is_rspec_example_group(name) {
        return call
            .block()
            .is_some_and(|b| b.as_block_node().is_some())
            .then_some(name);
    }

    if is_example_include(name) {
        return Some(name);
    }

    if is_rspec_let(name) {
        let has_block = call.block().is_some();
        let has_block_pass = call.arguments().is_some_and(|args| {
            args.arguments()
                .iter()
                .any(|arg| arg.as_block_argument_node().is_some())
        });
        return (has_block || has_block_pass).then_some(name);
    }

    if is_rspec_hook(name) || is_rspec_example(name) {
        return call
            .block()
            .is_some_and(|b| b.as_block_node().is_some())
            .then_some(name);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LeadingSubject, "cops/rspec/leading_subject");
}
