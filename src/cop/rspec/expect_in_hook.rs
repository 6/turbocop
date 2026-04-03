use crate::cop::shared::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_hook};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ExpectInHook: flags expectation calls inside before/after/around hooks.
///
/// **Root cause of original 157 FNs (0 FP):** The `find_expects_in_node` recursive search only handled
/// `StatementsNode` and `CallNode` receiver chains. It did not recurse into `IfNode`, `BlockNode`,
/// `UnlessNode`, `CaseNode`, `BeginNode`, or any other container nodes. This meant any `expect`
/// call nested inside control flow or iterator blocks within a hook body was missed.
///
/// **Fix (round 1):** Expanded `find_expects_in_node` to recurse into all common container node types
/// (if/unless/case/when/else/begin/ensure/block/for/while/until/lambda/parentheses/case_match/in),
/// plus CallNode arguments and blocks. This matches RuboCop's `def_node_search` deep traversal.
///
/// **Root cause of remaining 13 FNs (0 FP):** Three distinct gaps:
/// 1. `DefNode` — `expect` inside `def method_name` inside a hook was not traversed (4 FN in discourse, 2 in jruby)
/// 2. Write nodes — `@var = expr` wrapping CallNodes with blocks containing `expect` (1 antiwork, 1 neo4jrb, 2 diaspora, 1 discourse)
/// 3. Missing expectation methods — `should_receive`/`should_not_receive` not in EXPECT_METHODS (3 FN in brynary/webrat)
///
/// **Fix (round 2):** Added DefNode body traversal, write node (local/instance/class/global/constant/multi)
/// value traversal, and expanded EXPECT_METHODS to match RuboCop's full `Expectations.all` list:
/// `expect`, `is_expected`, `expect_any_instance_of`, `are_expected`, `should`, `should_not`,
/// `should_receive`, `should_not_receive`.
pub struct ExpectInHook;

/// Expectation methods to flag inside hooks.
/// Matches RuboCop's `Expectations.all` from rspec language config.
const EXPECT_METHODS: &[&[u8]] = &[
    b"expect",
    b"is_expected",
    b"expect_any_instance_of",
    b"are_expected",
    b"should",
    b"should_not",
    b"should_receive",
    b"should_not_receive",
];

impl Cop for ExpectInHook {
    fn name(&self) -> &'static str {
        "RSpec/ExpectInHook"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name().as_slice();
        if !is_rspec_hook(method_name) {
            return;
        }

        let hook_name = std::str::from_utf8(method_name).unwrap_or("hook");

        // Check the block body for expect calls
        let block_raw = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        find_expects_in_node(&body, source, self, hook_name, diagnostics);
    }
}

fn find_expects_in_node(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    cop: &ExpectInHook,
    hook_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            if EXPECT_METHODS.contains(&name) {
                let method_str = std::str::from_utf8(name).unwrap_or("expect");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    format!("Do not use `{method_str}` in `{hook_name}` hook"),
                ));
                // Don't return — still recurse into children (e.g., block args)
                // but for expect calls we've already reported, skip children to avoid dupes
                return;
            }
        }
    }

    // Recurse into all child nodes (deep search like RuboCop's def_node_search).
    // ruby_prism::Node doesn't expose a generic child_nodes() iterator, so we
    // handle each container type that can appear inside a hook body.
    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            find_expects_in_node(&child, source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(call) = node.as_call_node() {
        // Recurse into receiver chain (e.g., expect(x).to eq(...))
        if let Some(recv) = call.receiver() {
            find_expects_in_node(&recv, source, cop, hook_name, diagnostics);
        }
        // Recurse into arguments
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                find_expects_in_node(&arg, source, cop, hook_name, diagnostics);
            }
        }
        // Recurse into block
        if let Some(block) = call.block() {
            find_expects_in_node(&block, source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            find_expects_in_node(&body, source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_expects_in_node(&subsequent, source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        if let Some(else_clause) = unless_node.else_clause() {
            find_expects_in_node(&else_clause.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            find_expects_in_node(&cond, source, cop, hook_name, diagnostics);
        }
        if let Some(else_clause) = case_node.else_clause() {
            find_expects_in_node(&else_clause.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(when_node) = node.as_when_node() {
        if let Some(stmts) = when_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(ensure_node) = node.as_ensure_node() {
        if let Some(stmts) = ensure_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(in_node) = node.as_in_node() {
        if let Some(stmts) = in_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(case_match) = node.as_case_match_node() {
        for cond in case_match.conditions().iter() {
            find_expects_in_node(&cond, source, cop, hook_name, diagnostics);
        }
        if let Some(else_clause) = case_match.else_clause() {
            find_expects_in_node(&else_clause.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(parentheses) = node.as_parentheses_node() {
        if let Some(body) = parentheses.body() {
            find_expects_in_node(&body, source, cop, hook_name, diagnostics);
        }
        return;
    }
    // For/while/until loops
    if let Some(for_node) = node.as_for_node() {
        if let Some(stmts) = for_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(while_node) = node.as_while_node() {
        if let Some(stmts) = while_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        if let Some(stmts) = until_node.statements() {
            find_expects_in_node(&stmts.as_node(), source, cop, hook_name, diagnostics);
        }
        return;
    }
    // Lambda literal
    if let Some(lambda) = node.as_lambda_node() {
        if let Some(body) = lambda.body() {
            find_expects_in_node(&body, source, cop, hook_name, diagnostics);
        }
        return;
    }
    // DefNode — method definitions inside hooks (RuboCop's def_node_search traverses into these)
    if let Some(def_node) = node.as_def_node() {
        if let Some(body) = def_node.body() {
            find_expects_in_node(&body, source, cop, hook_name, diagnostics);
        }
        return;
    }
    // Write nodes — assignments whose RHS may contain expect calls
    if let Some(write) = node.as_local_variable_write_node() {
        find_expects_in_node(&write.value(), source, cop, hook_name, diagnostics);
        return;
    }
    if let Some(write) = node.as_instance_variable_write_node() {
        find_expects_in_node(&write.value(), source, cop, hook_name, diagnostics);
        return;
    }
    if let Some(write) = node.as_class_variable_write_node() {
        find_expects_in_node(&write.value(), source, cop, hook_name, diagnostics);
        return;
    }
    if let Some(write) = node.as_global_variable_write_node() {
        find_expects_in_node(&write.value(), source, cop, hook_name, diagnostics);
        return;
    }
    if let Some(write) = node.as_constant_write_node() {
        find_expects_in_node(&write.value(), source, cop, hook_name, diagnostics);
        return;
    }
    if let Some(write) = node.as_multi_write_node() {
        find_expects_in_node(&write.value(), source, cop, hook_name, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectInHook, "cops/rspec/expect_in_hook");
}
