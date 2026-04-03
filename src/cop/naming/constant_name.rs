use crate::cop::shared::node_type::{
    CONSTANT_AND_WRITE_NODE, CONSTANT_OPERATOR_WRITE_NODE, CONSTANT_OR_WRITE_NODE,
    CONSTANT_PATH_AND_WRITE_NODE, CONSTANT_PATH_OPERATOR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE,
    CONSTANT_PATH_TARGET_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_TARGET_NODE, CONSTANT_WRITE_NODE,
};
use crate::cop::shared::util::is_screaming_snake_case;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Naming/ConstantName - checks that constants are in SCREAMING_SNAKE_CASE.
///
/// ## Investigation (2026-03-08)
/// FN=365 root cause: `is_valid_rhs_for_assignment` was too permissive, allowing
/// array literals and regex literals that RuboCop does NOT allow. Also missing
/// handling for `ConstantOrWriteNode` (`||=`), `ConstantPathOrWriteNode`,
/// `ConstantTargetNode` / `ConstantPathTargetNode` (multi-assignment), and
/// `is_literal()` was missing range and interpolated string/symbol nodes.
///
/// Fixes applied:
/// - Removed array and regex allowances from `is_valid_rhs_for_assignment`
/// - Added `CONSTANT_OR_WRITE_NODE`, `CONSTANT_PATH_OR_WRITE_NODE` handling
/// - Added `MULTI_WRITE_NODE` to handle `ConstantTargetNode`/`ConstantPathTargetNode`
///   in multi-assignment (always flag, no valid_rhs check since value is shared)
/// - Added range, interpolated string/symbol nodes to `is_literal()`
///
/// Follow-up (2026-03-08): FP=1 regressed at a site using
/// `# rubocop:disable Style/ConstantName`. RuboCop still suppresses
/// `Naming/ConstantName` for that moved legacy name because the short name
/// stayed `ConstantName`. Fixed centrally in `parse/directives.rs`.
///
/// Follow-up (2026-03-08): FN=64 from missing compound assignment node types.
/// Added `ConstantAndWriteNode` (`&&=`), `ConstantOperatorWriteNode` (`+=`),
/// `ConstantPathAndWriteNode` (`Foo::Bar &&=`), `ConstantPathOperatorWriteNode`
/// (`Foo::Bar +=`). Also switched from `MultiWriteNode` parent traversal to
/// direct `ConstantTargetNode`/`ConstantPathTargetNode` dispatch, which also
/// picks up rescue-clause constant targets (`rescue => CapturedError`).
///
/// Follow-up (2026-03-09): FN=45 from `is_literal()` missing array and hash
/// node types. Method calls on array/hash receivers (e.g., `[...].freeze`,
/// `[...].to_set`, `[...] + OTHER`, `{...}.freeze`) were treated as non-literal
/// receiver calls and allowed. RuboCop's `literal?` predicate includes `array`
/// and `hash` types. Added `as_array_node`, `as_hash_node`, and
/// `as_keyword_hash_node` to `is_literal()`.
///
/// Follow-up (2026-03-10): FP=5 from CallNode with block not recognized as
/// allowed assignment. In Parser AST, `[1,2].map { |x| x }` becomes a `:block`
/// node wrapping the `:send`, and RuboCop's `allowed_assignment?` allows `:block`
/// immediately. In Prism, the CallNode itself has a block child. Fixed by checking
/// `call.block().is_some()` before the literal receiver check in
/// `is_valid_rhs_for_assignment()`.
///
/// Follow-up (2026-03-20): FN=4 from two root causes:
/// 1. Range nodes (`irange`/`erange`) were missing from `is_literal()`. RuboCop's
///    `literal?` predicate includes ranges, so `(1..5).freeze` and `('A'..'Z').to_a`
///    should be flagged.
/// 2. `call.block().is_some()` matched both `BlockNode` and `BlockArgumentNode`.
///    In Parser AST, `&:to_sym` is an argument, not a wrapping block — only actual
///    `BlockNode` (do/end, {}) should be allowed. Fixed to use
///    `call.block().and_then(|b| b.as_block_node()).is_some()`.
///
/// Follow-up (2026-03-20): FP=14 from lambda literals (`-> {}`, `-> do end`).
/// In Parser AST, lambdas are `:block` nodes wrapping `:lambda`, so RuboCop's
/// `%i[block const casgn].include?(value.type)` allows them. In Prism, lambdas are
/// `LambdaNode` (not `BlockNode`), so they need a separate check. Previously
/// `LambdaNode` was incorrectly removed thinking RuboCop didn't allow them.
/// Fixed by re-adding `value.as_lambda_node().is_some()` to
/// `is_valid_rhs_for_assignment()`.
///
/// ## Corpus investigation (2026-03-23) — extended corpus
///
/// Extended corpus reported FP=7 (all vendor-path repos), FN=2.
/// FN=2: `Positive = ->{ (_1 in Range) and ... }` — lambda with numbered
/// parameters. In Parser AST, `->{ _1 }` creates `:numblock` (not `:block`),
/// which is NOT in RuboCop's `allowed_assignment?` list. In Prism, all lambdas
/// are `LambdaNode` regardless. Fix: check if lambda parameters are
/// `NumberedParametersNode` or `ItParametersNode` and only allow regular lambdas.
pub struct ConstantName;

impl Cop for ConstantName {
    fn name(&self) -> &'static str {
        "Naming/ConstantName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CONSTANT_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            CONSTANT_AND_WRITE_NODE,
            CONSTANT_PATH_AND_WRITE_NODE,
            CONSTANT_OPERATOR_WRITE_NODE,
            CONSTANT_PATH_OPERATOR_WRITE_NODE,
            CONSTANT_TARGET_NODE,
            CONSTANT_PATH_TARGET_NODE,
        ]
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
        if let Some(cw) = node.as_constant_write_node() {
            let const_name = cw.name().as_slice();
            let value = cw.value();
            diagnostics.extend(self.check_constant(source, const_name, &cw.name_loc(), &value));
        }

        if let Some(cpw) = node.as_constant_path_write_node() {
            let target = cpw.target();
            let name_loc = target.name_loc();
            let const_name = target.name().map(|n| n.as_slice()).unwrap_or(b"");
            let value = cpw.value();
            diagnostics.extend(self.check_constant(source, const_name, &name_loc, &value));
        }

        // Foo ||= value
        if let Some(cow) = node.as_constant_or_write_node() {
            let const_name = cow.name().as_slice();
            let value = cow.value();
            diagnostics.extend(self.check_constant(source, const_name, &cow.name_loc(), &value));
        }

        // Mod::Setting ||= value
        if let Some(cpow) = node.as_constant_path_or_write_node() {
            let target = cpow.target();
            let name_loc = target.name_loc();
            let const_name = target.name().map(|n| n.as_slice()).unwrap_or(b"");
            let value = cpow.value();
            diagnostics.extend(self.check_constant(source, const_name, &name_loc, &value));
        }

        // Foo &&= value
        if let Some(caw) = node.as_constant_and_write_node() {
            let const_name = caw.name().as_slice();
            let value = caw.value();
            diagnostics.extend(self.check_constant(source, const_name, &caw.name_loc(), &value));
        }

        // Mod::Setting &&= value
        if let Some(cpaw) = node.as_constant_path_and_write_node() {
            let target = cpaw.target();
            let name_loc = target.name_loc();
            let const_name = target.name().map(|n| n.as_slice()).unwrap_or(b"");
            let value = cpaw.value();
            diagnostics.extend(self.check_constant(source, const_name, &name_loc, &value));
        }

        // Foo += value
        if let Some(cow) = node.as_constant_operator_write_node() {
            let const_name = cow.name().as_slice();
            let value = cow.value();
            diagnostics.extend(self.check_constant(source, const_name, &cow.name_loc(), &value));
        }

        // Mod::Setting += value
        if let Some(cpow) = node.as_constant_path_operator_write_node() {
            let target = cpow.target();
            let name_loc = target.name_loc();
            let const_name = target.name().map(|n| n.as_slice()).unwrap_or(b"");
            let value = cpow.value();
            diagnostics.extend(self.check_constant(source, const_name, &name_loc, &value));
        }

        // ConstantTargetNode — appears in multi-assignment (A, B = 1, 2) and
        // rescue clauses (rescue => CapturedError). No valid_rhs check.
        if let Some(ct) = node.as_constant_target_node() {
            let const_name = ct.name().as_slice();
            if !is_screaming_snake_case(const_name) {
                let (line, column) = source.offset_to_line_col(ct.location().start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use SCREAMING_SNAKE_CASE for constants.".to_string(),
                ));
            }
        }

        // ConstantPathTargetNode — appears in multi-assignment (Mod::A, Mod::B = 1, 2)
        if let Some(cpt) = node.as_constant_path_target_node() {
            let name_loc = cpt.name_loc();
            let const_name = cpt.name().map(|n| n.as_slice()).unwrap_or(b"");
            if !is_screaming_snake_case(const_name) {
                let (line, column) = source.offset_to_line_col(name_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use SCREAMING_SNAKE_CASE for constants.".to_string(),
                ));
            }
        }
    }
}

impl ConstantName {
    fn check_constant(
        &self,
        source: &SourceFile,
        const_name: &[u8],
        loc: &ruby_prism::Location<'_>,
        value: &ruby_prism::Node<'_>,
    ) -> Vec<Diagnostic> {
        // Allow SCREAMING_SNAKE_CASE (standard constant style)
        if is_screaming_snake_case(const_name) {
            return Vec::new();
        }

        // Allow non-SCREAMING_SNAKE_CASE only if the RHS is a class/module/struct creation
        // pattern. This matches RuboCop's `valid_for_assignment?` check.
        if is_valid_rhs_for_assignment(value) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            "Use SCREAMING_SNAKE_CASE for constants.".to_string(),
        )]
    }
}

/// Check if the RHS of a constant assignment is an acceptable pattern for
/// non-SCREAMING_SNAKE_CASE names. Matches RuboCop's `allowed_assignment?`:
///
/// 1. Block, constant reference, or chained constant assignment
/// 2. Method call where receiver is nil or not a literal
/// 3. `Class.new(...)` or `Struct.new(...)`
/// 4. Conditional expression containing a constant in branches
fn is_valid_rhs_for_assignment(value: &ruby_prism::Node<'_>) -> bool {
    // Block node: `proc { }`, `lambda { }`, `Foo.new { }`
    if value.as_block_node().is_some() {
        return true;
    }

    // Lambda literal: `-> {}`, `-> do ... end`
    // In Parser AST, lambda literals are `:block` nodes wrapping `:lambda`, so
    // RuboCop's `%i[block const casgn].include?(value.type)` allows them.
    // However, lambdas with numbered params (`_1`) are `:numblock` in Parser,
    // and with `it` are `:itblock` — these are NOT in RuboCop's allowed list.
    // In Prism, all lambdas are `LambdaNode` regardless, so we must check the
    // parameters type to distinguish.
    if let Some(lambda) = value.as_lambda_node() {
        let uses_numblock = lambda.parameters().is_some_and(|p| {
            p.as_numbered_parameters_node().is_some() || p.as_it_parameters_node().is_some()
        });
        if !uses_numblock {
            return true;
        }
    }

    // Constant reference: `Server = BaseServer` or `Stream = Foo::Bar`
    if value.as_constant_read_node().is_some() || value.as_constant_path_node().is_some() {
        return true;
    }

    // Chained constant assignment: `A = B = 1`
    if value.as_constant_write_node().is_some() || value.as_constant_path_write_node().is_some() {
        return true;
    }

    // Method call: allowed if receiver is nil or receiver is not a literal.
    // This covers patterns like `NewClass = some_factory_method` and
    // `Uchar1max = (1<<7) - 1` (receiver is a call expression, not a literal).
    // Only method calls ON bare literals like `"foo".freeze` or `1 + 2` are disallowed.
    if let Some(call) = value.as_call_node() {
        // CallNode with a block — equivalent to Parser's :block type node.
        // In Parser AST, `[1,2].map { |x| x }` wraps the send in a block node.
        // In Prism, the CallNode itself has a block child. RuboCop allows these.
        // Important: only allow actual BlockNode (do/end, {}), NOT BlockArgumentNode
        // (&:sym, &block). In Parser AST, `&:sym` is an argument, not a wrapping block.
        if call.block().and_then(|b| b.as_block_node()).is_some() {
            return true;
        }
        match call.receiver() {
            None => return true, // receiverless method call
            Some(receiver) => {
                if !is_literal_receiver(&receiver) {
                    return true;
                }
            }
        }
    }

    // Conditional expression containing a constant in branches
    if let Some(if_node) = value.as_if_node() {
        if branch_contains_constant(&if_node) {
            return true;
        }
    }

    false
}

/// Check if a receiver is a literal (int, float, string, symbol, etc.)
/// or a parenthesized literal `(literal)`. Matches RuboCop's `literal_receiver?`.
fn is_literal_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if is_literal(node) {
        return true;
    }
    // `(literal)` — parenthesized literal
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let children: Vec<_> = stmts.body().iter().collect();
                if children.len() == 1 && is_literal(&children[0]) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a node is a literal value. Matches RuboCop's `literal?` predicate:
/// int, float, str, dstr, sym, dsym, complex, rational, regexp, true, false, nil,
/// array, hash (including keyword hash), range (irange/erange).
/// Used by `is_literal_receiver` to determine if a method call on a literal
/// (e.g., `"foo".freeze`, `[1,2].freeze`, `(1..5).freeze`) should be disallowed.
fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_range_node().is_some()
}

/// Check if an if-expression has a constant in any of its branches.
fn branch_contains_constant(if_node: &ruby_prism::IfNode<'_>) -> bool {
    // Check the "then" branch
    if let Some(stmts) = if_node.statements() {
        for child in stmts.body().iter() {
            if child.as_constant_read_node().is_some() || child.as_constant_path_node().is_some() {
                return true;
            }
        }
    }
    // Check the else branch
    if let Some(else_clause) = if_node.subsequent() {
        if let Some(else_if) = else_clause.as_if_node() {
            if branch_contains_constant(&else_if) {
                return true;
            }
        }
        if let Some(else_node) = else_clause.as_else_node() {
            if let Some(stmts) = else_node.statements() {
                for child in stmts.body().iter() {
                    if child.as_constant_read_node().is_some()
                        || child.as_constant_path_node().is_some()
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantName, "cops/naming/constant_name");
}
