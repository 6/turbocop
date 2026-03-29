use crate::cop::node_type::{
    BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, FALSE_NODE, FLOAT_NODE,
    IMAGINARY_NODE, INTEGER_NODE, NIL_NODE, RATIONAL_NODE, STATEMENTS_NODE, STRING_NODE,
    SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03):
/// - RuboCop only flags `fetch(:key) { 'string' }` when `frozen_string_literal: true`
///   is enabled (`check_for_string?` calls `frozen_string_literals_enabled?`).
///   An earlier attempt removed this check, causing +56 FP on multi_json smoke test.
///   The check was restored.
/// - FP: `::Rails.cache.fetch` was not skipped because `ConstantPathNode.location()`
///   returns `::Rails` (full source), not just `Rails`. Fixed by using `.name()` which
///   returns just the constant name segment.
/// - FN: frozen_string_literal detection only checked line 1, missing files with
///   encoding comments on line 1 (e.g. `# -*- coding: utf-8 -*-`). Fixed by checking
///   the first 3 lines.
/// - FN: RuboCop scans all leading comment lines before the first non-comment token
///   for `frozen_string_literal`; limiting nitrocop to the first 3 lines missed files
///   with longer license headers before the magic comment. Fixed by scanning the full
///   leading comment block and honoring explicit `true`/`false`.
/// - FN: Unary minus with space (`- 1`) parsed as `CallNode` with method `:-@` wrapping
///   an integer/float, not a bare literal. Fixed by treating such CallNodes as simple
///   literals in `is_simple_literal`.
pub struct RedundantFetchBlock;

impl RedundantFetchBlock {
    fn is_simple_literal(node: &ruby_prism::Node<'_>) -> bool {
        node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_symbol_node().is_some()
            || node.as_string_node().is_some()
            || node.as_true_node().is_some()
            || node.as_false_node().is_some()
            || node.as_nil_node().is_some()
            || node.as_rational_node().is_some()
            || node.as_imaginary_node().is_some()
            || Self::is_unary_minus_literal(node)
    }

    /// Detect unary minus applied to a numeric literal (e.g. `- 1` or `-1.0`).
    /// Prism parses `- 1` (with space) as a CallNode with method `:-@` and an
    /// integer/float receiver, rather than folding it into the literal.
    fn is_unary_minus_literal(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"-@" && call.arguments().is_none() {
                if let Some(receiver) = call.receiver() {
                    return receiver.as_integer_node().is_some()
                        || receiver.as_float_node().is_some();
                }
            }
        }
        false
    }

    /// Approximate RuboCop's leading comment scan for `frozen_string_literal`.
    /// We keep scanning until the first non-comment, non-blank line.
    fn frozen_string_literals_enabled(source: &SourceFile) -> bool {
        for line in source.lines() {
            let text = std::str::from_utf8(line).unwrap_or("");
            let trimmed = text.trim_start();

            if trimmed.is_empty() {
                continue;
            }

            if !trimmed.starts_with('#') {
                break;
            }

            if trimmed.contains("frozen_string_literal: true") {
                return true;
            }

            if trimmed.contains("frozen_string_literal: false") {
                return false;
            }
        }

        false
    }
}

impl Cop for RedundantFetchBlock {
    fn name(&self) -> &'static str {
        "Style/RedundantFetchBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            NIL_NODE,
            RATIONAL_NODE,
            STATEMENTS_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let safe_for_constants = config.get_bool("SafeForConstants", false);
        // RuboCop only flags string defaults when frozen_string_literal: true.
        let frozen_string_literal = Self::frozen_string_literals_enabled(source);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"fetch" {
            return;
        }

        // Must have exactly one argument (the key)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Block must have no parameters
        if block_node.parameters().is_some() {
            return;
        }

        // Check block body
        let body = block_node.body();

        // Skip Rails.cache.fetch - those blocks do computation
        if let Some(receiver) = call.receiver() {
            if let Some(recv_call) = receiver.as_call_node() {
                if recv_call.name().as_slice() == b"cache" {
                    if let Some(recv_recv) = recv_call.receiver() {
                        if let Some(const_node) = recv_recv.as_constant_read_node() {
                            if const_node.name().as_slice() == b"Rails" {
                                return;
                            }
                        }
                        if let Some(const_path) = recv_recv.as_constant_path_node() {
                            if let Some(name) = const_path.name() {
                                if name.as_slice() == b"Rails" {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }

        let is_redundant = if let Some(ref body) = body {
            if let Some(stmts) = body.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                if body_stmts.len() == 1 {
                    let expr = &body_stmts[0];
                    // String literals require frozen_string_literal: true
                    if expr.as_string_node().is_some() && !frozen_string_literal {
                        false
                    } else if Self::is_simple_literal(expr) {
                        true
                    } else if safe_for_constants {
                        expr.as_constant_read_node().is_some()
                            || expr.as_constant_path_node().is_some()
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // Empty block: fetch(:key) {} => fetch(:key, nil)
            true
        };

        if !is_redundant {
            return;
        }

        let key_src = std::str::from_utf8(arg_list[0].location().as_slice()).unwrap_or("");
        let value_src = if let Some(body) = body {
            if let Some(stmts) = body.as_statements_node() {
                let body_stmts: Vec<_> = stmts.body().iter().collect();
                if body_stmts.len() == 1 {
                    std::str::from_utf8(body_stmts[0].location().as_slice())
                        .unwrap_or("nil")
                        .to_string()
                } else {
                    "nil".to_string()
                }
            } else {
                "nil".to_string()
            }
        } else {
            "nil".to_string()
        };

        let fetch_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(fetch_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `fetch({key_src}, {value_src})` instead of `fetch({key_src}) {{ {value_src} }}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        RedundantFetchBlock,
        "cops/style/redundant_fetch_block",
        basic = "basic.rb",
        frozen_string_literal_line2 = "frozen_string_literal_line2.rb",
        frozen_string_literal_after_header = "frozen_string_literal_after_header.rb",
    );
}
