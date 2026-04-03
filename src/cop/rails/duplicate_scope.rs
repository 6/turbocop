use std::collections::HashMap;

use crate::cop::shared::node_type::{CLASS_NODE, SYMBOL_NODE};
use crate::cop::shared::util::{class_body_calls, is_dsl_call};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/DuplicateScope
///
/// ## Reverted fix attempt (2026-03-23, commit bb5f83a2)
///
/// Attempted to fix FP on lambda/arrow equivalence and FN on bodyless scopes.
/// Introduced FP=21 and FN=2 on standard corpus; reverted in 1bf1bea3.
///
/// **FP=21 (block calls treated as bodyless):** `scope :name do ... end` calls
/// have `call.arguments()` returning only `:name` (1 arg) with the `do...end`
/// block in `call.block()`. The `extract_scope_body_source` function saw
/// `arg_list.len() == 1` and returned `__bodyless__`, grouping all block-style
/// scopes as duplicates. In Parser AST, `scope :name do...end` is a `(block
/// (send ...))` node — `each_child_node(:send)` at class body level never finds
/// the send because it's wrapped in block. Fix: skip calls where
/// `call.block().is_some()` in the scope collection loop.
///
/// **FN=2 (lambda normalization removed):** The commit removed lambda/arrow
/// normalization, claiming RuboCop treats `-> {}` and `lambda {}` as different.
/// This is wrong — Parser gem normalizes BOTH to `(lambda ...)` nodes, so
/// RuboCop treats them as duplicates. Fix: restore the normalization.
///
/// ## Fix (2026-03-24): block extensions and bodyless scopes
///
/// **FP fix:** Scopes with block extensions (`scope :name, -> { all } do...end`)
/// were grouped with plain lambda scopes sharing the same body. The block changes
/// behavior, so these are skipped from duplicate grouping when `call.block().is_some()`.
///
/// **FN fix:** Bodyless scopes (`scope :name` with no lambda/proc body) were skipped
/// because `arg_list.len() < 2` returned `None`. Now they use a sentinel key
/// `__bodyless__` so all bodyless scopes in a class are grouped as duplicates,
/// matching RuboCop behavior. The `call.block().is_some()` check in the caller
/// ensures `scope :name do...end` (block-only scopes) are NOT treated as bodyless.
pub struct DuplicateScope;

impl Cop for DuplicateScope {
    fn name(&self) -> &'static str {
        "Rails/DuplicateScope"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, SYMBOL_NODE]
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
        let class = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        let calls = class_body_calls(&class);

        // Group scopes by their body expression (everything after the name).
        // RuboCop flags scopes that share the same expression, not the same name.
        let mut seen: HashMap<Vec<u8>, Vec<&ruby_prism::CallNode<'_>>> = HashMap::new();

        for call in &calls {
            if !is_dsl_call(call, b"scope") {
                continue;
            }

            // Scopes with block extensions (`scope :name, -> { } do ... end`)
            // behave differently even if the lambda body matches another scope.
            // Skip them from duplicate grouping entirely.
            if call.block().is_some() {
                continue;
            }

            let body_key = match extract_scope_body_source(call) {
                Some(k) => k,
                None => continue,
            };

            seen.entry(body_key).or_default().push(call);
        }

        for calls in seen.values() {
            if calls.len() < 2 {
                continue;
            }
            // Flag all scopes in the group (RuboCop flags every duplicate)
            for call in calls {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Multiple scopes share this same expression.".to_string(),
                ));
            }
        }
    }
}

/// Extract a normalised key for the scope body expression so that `-> { all }`
/// and `lambda { all }` are treated as duplicates (matching RuboCop behaviour).
///
/// For lambda expressions (`LambdaNode` from `->` syntax, or `CallNode` named
/// `lambda` from the `lambda` keyword), we extract the block body source and
/// parameter source and combine them into a canonical form.  For everything
/// else we fall back to the raw source of the arguments after the scope name.
fn extract_scope_body_source<'a>(call: &ruby_prism::CallNode<'a>) -> Option<Vec<u8>> {
    let args = match call.arguments() {
        Some(a) => a,
        None => {
            // Bodyless scope with no arguments at all (e.g. `scope :name` via
            // send without parens — but name is still in arguments for DSL calls).
            // Treat as bodyless.
            return Some(b"__bodyless__".to_vec());
        }
    };
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.len() < 2 {
        // Only the scope name, no body expression. Since we already skipped
        // block scopes (`scope :name do...end`) in the caller, this is a
        // truly bodyless scope like `scope :filter_all`.
        return Some(b"__bodyless__".to_vec());
    }

    // If there is exactly one body argument (the lambda/proc expression),
    // try to normalise lambda syntax.
    if arg_list.len() == 2 {
        if let Some(key) = normalise_lambda_body(&arg_list[1]) {
            return Some(key);
        }
    }

    // Fallback: raw source of everything after the scope name.
    let start = arg_list[1].location().start_offset();
    let end = arg_list.last().unwrap().location().end_offset();
    Some(
        call.location().as_slice()
            [start - call.location().start_offset()..end - call.location().start_offset()]
            .to_vec(),
    )
}

/// Try to extract a canonical `(params, body)` key from a lambda expression,
/// regardless of whether it was written as `-> { }` or `lambda { }`.
fn normalise_lambda_body(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    // `-> { body }` parses as a LambdaNode
    if let Some(lambda) = node.as_lambda_node() {
        let params = lambda
            .parameters()
            .map(|p| p.location().as_slice())
            .unwrap_or(b"");
        let body = lambda
            .body()
            .map(|b| b.location().as_slice())
            .unwrap_or(b"");
        let mut key = Vec::with_capacity(b"lambda:".len() + params.len() + 1 + body.len());
        key.extend_from_slice(b"lambda:");
        key.extend_from_slice(params);
        key.push(b':');
        key.extend_from_slice(body);
        return Some(key);
    }

    // `lambda { body }` parses as a CallNode with name `lambda` and an
    // attached block.
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"lambda" {
            if let Some(block) = call.block().and_then(|b| b.as_block_node()) {
                let params = block
                    .parameters()
                    .map(|p| p.location().as_slice())
                    .unwrap_or(b"");
                let body = block.body().map(|b| b.location().as_slice()).unwrap_or(b"");
                let mut key = Vec::with_capacity(b"lambda:".len() + params.len() + 1 + body.len());
                key.extend_from_slice(b"lambda:");
                key.extend_from_slice(params);
                key.push(b':');
                key.extend_from_slice(body);
                return Some(key);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateScope, "cops/rails/duplicate_scope");
}
