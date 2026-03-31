use crate::cop::{CodeMap, Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

/// Style/BlockDelimiters checks for uses of braces or do/end around single-line
/// or multi-line blocks.
///
/// ## Investigation findings (2026-03-08)
///
/// Root cause of 2,263 FPs: RuboCop suppresses nested block offenses. When a block
/// is flagged (e.g., outer multi-line `{...}`), RuboCop calls `ignore_node(block)` in
/// the `add_offense` handler. This causes `part_of_ignored_node?` to return true for
/// all blocks whose source range is contained within the flagged block. As a result,
/// only the outermost offending block is flagged — inner blocks are suppressed.
///
/// nitrocop was missing this suppression: it flagged every multi-line `{...}` block
/// independently, including those nested inside already-flagged blocks. This produced
/// many duplicate offenses that RuboCop does not emit.
///
/// Additionally, blocks in non-parenthesized argument positions (already handled via
/// `ignored_blocks`) were not propagating their suppression to nested child blocks.
/// A block inside an ignored block's body should also be suppressed, matching
/// RuboCop's `part_of_ignored_node?` range-containment check.
///
/// Fix: track "suppressed ranges" (byte offset ranges). When a block is ignored
/// (non-parenthesized arg) or flagged (offense registered), add its full byte range.
/// Before checking any block, verify it is not contained within a suppressed range.
///
/// ## Investigation findings (2026-03-15)
///
/// Root cause of 188 FPs: chained method calls like `a.select { }.reject { }.each { }`
/// In Parser AST, the outermost block (`.each`) wraps the entire chain, so RuboCop's
/// `ignore_node` + `part_of_ignored_node?` naturally suppresses inner blocks.
/// In Prism, BlockNode ranges only cover `{...}`, not the receiver chain. Fix: use
/// the CallNode's range (which covers the full chain) for suppression instead of the
/// BlockNode's range.
///
/// Root cause of some FNs: operator methods (`+`, `*`, etc.) with a single block-bearing
/// argument were incorrectly having their argument blocks ignored. RuboCop's
/// `single_argument_operator_method?` check skips the ignore logic for these cases.
/// Fix: added `is_operator_method` check to skip `collect_ignored_blocks` for operators.
///
/// ## Investigation findings (2026-03-15, second pass)
///
/// Root cause of 68 FPs: `is_single_arg_operator` was too broad. It checked only
/// `args.len() == 1` without verifying the argument is a block node. RuboCop's
/// `single_argument_operator_method?` additionally checks `first_argument.block_type?`.
/// When the operator's argument is a send/call chain (e.g., `a + items.map { }.join`),
/// the argument is NOT a block type, so RuboCop proceeds to call `get_blocks` on it,
/// finding and ignoring blocks within the expression tree. Fix: added block-type check.
///
/// Additional FP source: lambda nodes (`-> { }`) in non-parenthesized keyword args.
/// In Parser AST, lambdas are block nodes, so RuboCop's `get_blocks` yields them and
/// `ignore_node` suppresses all nested blocks. In Prism, lambdas are `LambdaNode`,
/// which `collect_ignored_blocks` didn't handle. Fix: added LambdaNode handling that
/// recurses into the lambda body to find and ignore nested blocks.
///
/// Root cause of 14 FNs: `super(...)` with blocks uses `SuperNode` / `ForwardingSuperNode`
/// in Prism, not `CallNode`. The visitor only had `visit_call_node`. Fix: added
/// `visit_super_node` and `visit_forwarding_super_node` handlers.
///
/// ## Investigation findings (2026-03-15, third pass)
///
/// Root cause of 6 FPs in two patterns:
///
/// 1. Blocks inside lambda bodies with assignment (4 FPs in pakyow): lambda body
///    recursion via `collect_ignored_blocks_from_body` only handled `CallNode` and
///    `StatementsNode`, missing assignment nodes like `LocalVariableWriteNode`. When
///    a block appeared as the RHS of an assignment inside a lambda body in a keyword
///    arg, it wasn't collected and suppressed. Fix: added handling for all assignment
///    node types in `collect_ignored_blocks_from_body`.
///
/// 2. Hash[] constructor arguments (2 FPs): `Hash[list.map { }]` uses `[]` method,
///    which in Prism has `opening_loc() = Some` (pointing to `[`), making it appear
///    parenthesized. RuboCop treats `[]` calls as non-parenthesized for block-binding.
///    Fix: exclude `[]` method name from the `is_parenthesized` check.
///
///
/// ## Investigation findings (2026-03-15, smoke follow-up)
///
/// Root cause of 18 FNs on `ruby-formatter/rufo`: a previous change incorrectly added a
/// `return_value_used?` carve-out to `EnforcedStyle: line_count_based`. In RuboCop 1.84.2,
/// `line_count_based_block_style?` is just `node.multiline? ^ node.braces?`; the
/// `return_value_used?` / `functional_block?` logic only applies to `semantic` style.
/// That mistake suppressed real offenses like `expect { ... }.to output(...)` and
/// `result = items.map { ... }`. Fix: removed the `return_value_used` tracking/exemption
/// and added regression fixtures for chained/assigned multi-line brace blocks.
///
/// ## Investigation findings (2026-03-31)
///
/// Root cause of 1 FP in `thirdtank/brut`: blocks using `it` (implicit parameter) inside
/// operator method arguments. The corpus config uses `TargetRubyVersion: 4.0`, which causes
/// RuboCop's Parser to parse `reject { it =~ /.../ }` as an `:itblock` node instead of
/// `:block`. RuboCop's `single_argument_operator_method?` checks `first_argument.block_type?`,
/// which returns false for `:itblock` (and `:numblock`) since `block_type?` checks
/// `@type == :block` exactly. When `single_argument_operator_method?` returns false,
/// `on_send` proceeds to call `get_blocks`, which finds and ignores the blocks via
/// `ignore_node`. In Prism, all blocks are `BlockNode` regardless of parameter style.
/// Fix: added `is_explicit_block()` check that returns false when the block's parameters
/// are `ItParametersNode` or `NumberedParametersNode`, matching Parser's type distinction.
pub struct BlockDelimiters;

impl Cop for BlockDelimiters {
    fn name(&self) -> &'static str {
        "Style/BlockDelimiters"
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
        let enforced_style = config.get_str("EnforcedStyle", "line_count_based");
        let _procedural_methods = config.get_string_array("ProceduralMethods");
        let _functional_methods = config.get_string_array("FunctionalMethods");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let _allow_braces_on_procedural =
            config.get_bool("AllowBracesOnProceduralOneLiners", false);
        let braces_required_methods = config.get_string_array("BracesRequiredMethods");

        if enforced_style != "line_count_based" {
            return;
        }

        let allowed = allowed_methods
            .unwrap_or_else(|| vec!["lambda".to_string(), "proc".to_string(), "it".to_string()]);
        let patterns = allowed_patterns.unwrap_or_default();
        let braces_required = braces_required_methods.unwrap_or_default();

        let mut visitor = BlockDelimitersVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            ignored_blocks: HashSet::new(),
            suppressed_ranges: Vec::new(),
            allowed_methods: allowed,
            allowed_patterns: patterns,
            braces_required_methods: braces_required,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlockDelimitersVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a BlockDelimiters,
    diagnostics: Vec<Diagnostic>,
    ignored_blocks: HashSet<usize>,
    /// Byte ranges of blocks that suppress nested block checks.
    /// Includes: (1) blocks in non-parenthesized arg positions (binding change),
    /// (2) blocks that already received an offense (RuboCop `ignore_node` behavior).
    suppressed_ranges: Vec<(usize, usize)>,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<String>,
    braces_required_methods: Vec<String>,
}

impl<'a> BlockDelimitersVisitor<'a> {
    /// Check if a block's byte range is contained within any suppressed range.
    fn is_suppressed(&self, start: usize, end: usize) -> bool {
        self.suppressed_ranges
            .iter()
            .any(|&(s, e)| s <= start && end <= e)
    }

    /// Add a byte range to the suppressed set.
    ///
    /// Callers should pass the **call node's** range (not just the block node's)
    /// so that chained blocks are properly suppressed. In Prism, chained calls
    /// like `a.select { }.reject { }` have the outermost CallNode covering the
    /// entire chain, while BlockNode ranges only cover their own `{...}`.
    fn suppress_range(&mut self, start: usize, end: usize) {
        self.suppressed_ranges.push((start, end));
    }

    fn check_block(&mut self, block_node: &ruby_prism::BlockNode<'_>, method_name: &[u8]) -> bool {
        let method_str = std::str::from_utf8(method_name).unwrap_or("");

        // Skip AllowedMethods (default: lambda, proc, it)
        if self.allowed_methods.iter().any(|m| m == method_str) {
            return false;
        }

        // Skip AllowedPatterns
        for pattern in &self.allowed_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(method_str) {
                    return false;
                }
            }
        }

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();
        let opening = opening_loc.as_slice();

        let (open_line, _) = self.source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = self.source.offset_to_line_col(closing_loc.start_offset());
        let is_single_line = open_line == close_line;

        // BracesRequiredMethods: must use braces
        if self.braces_required_methods.iter().any(|m| m == method_str) {
            if opening == b"do" {
                let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!(
                        "Brace delimiters `{{...}}` required for '{}' method.",
                        method_str
                    ),
                ));
                return true;
            }
            return false;
        }

        // require_do_end: single-line do-end blocks with rescue/ensure clauses
        // cannot be converted to braces (syntax error). Skip these.
        if is_single_line && opening == b"do" && block_has_rescue_or_ensure(block_node) {
            return false;
        }

        // line_count_based style
        if is_single_line && opening == b"do" {
            let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer `{...}` over `do...end` for single-line blocks.".to_string(),
            ));
            return true;
        } else if !is_single_line && opening == b"{" {
            let (line, column) = self.source.offset_to_line_col(opening_loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer `do...end` over `{...}` for multi-line blocks.".to_string(),
            ));
            return true;
        }
        false
    }
}

impl<'a> Visit<'_> for BlockDelimitersVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // For non-parenthesized calls with arguments, mark argument blocks
        // as ignored. Changing delimiters on these blocks would change binding
        // semantics (braces bind tighter than do..end).
        // `[]` method calls (e.g., `Hash[x]`) use square brackets, not parens.
        // In Prism, `opening_loc()` returns `Some` for `[`, but RuboCop treats
        // `[]` calls as non-parenthesized for block-binding purposes.
        let method_name = node.name().as_slice();
        let is_parenthesized = node.opening_loc().is_some() && method_name != b"[]";
        let is_assignment = method_name.ends_with(b"=")
            && method_name != b"=="
            && method_name != b"!="
            && method_name != b"<="
            && method_name != b">="
            && method_name != b"===";

        // Skip operator methods with a single block-bearing argument.
        // RuboCop's `single_argument_operator_method?` check: for `a + b { }`,
        // the `+` call should NOT mark `b`'s block as ignored, because the
        // block is genuinely part of `b`'s call, not an ambiguous binding case.
        // IMPORTANT: Only skip when the single argument IS itself a block node
        // (i.e., a call whose block is the argument). When the argument is a
        // send/call chain (e.g., `a + items.map { }.join`), the block is nested
        // inside the expression and should be found via collect_ignored_blocks.
        //
        // RuboCop quirk: `block_type?` checks `@type == :block`, which returns
        // false for `:itblock` (Ruby 4.0 `it` parameter) and `:numblock`
        // (`_1`/`_2` numbered parameters). So `single_argument_operator_method?`
        // returns false for those, causing `get_blocks` to run and ignore them.
        // We replicate this by checking that the block has explicit parameters
        // (BlockParametersNode), not it/numbered parameters.
        let is_single_arg_operator = is_operator_method(method_name)
            && node.arguments().is_some_and(|args| {
                args.arguments().len() == 1
                    && args.arguments().iter().next().is_some_and(|arg| {
                        arg.as_call_node()
                            .and_then(|c| c.block())
                            .and_then(|b| b.as_block_node())
                            .is_some_and(|block| is_explicit_block(block))
                    })
            });

        if !is_parenthesized && !is_assignment && !is_single_arg_operator {
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    collect_ignored_blocks(&arg, &mut self.ignored_blocks);
                }
            }
        }

        // Phase 2: Check this call's block (if any)
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let offset = block_node.opening_loc().start_offset();
                let block_end = block_node.closing_loc().end_offset();

                // Use the call node's full range for suppression. In Prism,
                // chained calls like `a.select { }.reject { }` have the outer
                // CallNode covering the entire chain [0..end], while BlockNode
                // ranges only cover `{...}`. Using the call node's range ensures
                // inner blocks in a chain are contained within the suppressed range.
                let call_start = node.location().start_offset();
                let call_end = node.location().end_offset();

                if self.ignored_blocks.contains(&offset) {
                    // Block is in non-parenthesized arg position — suppress it
                    // and all nested blocks (RuboCop's part_of_ignored_node? behavior)
                    self.suppress_range(call_start, call_end);
                } else if !self.is_suppressed(offset, block_end) {
                    // Block is not inside a suppressed range — check it
                    let flagged = self.check_block(&block_node, method_name);
                    if flagged {
                        // Suppress nested blocks (RuboCop's ignore_node in add_offense)
                        self.suppress_range(call_start, call_end);
                    }
                }
            }
        }

        // Recurse into children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'_>) {
        // SuperNode: `super(args) { ... }` or `super(args) do ... end`
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let offset = block_node.opening_loc().start_offset();
                let block_end = block_node.closing_loc().end_offset();
                let call_start = node.location().start_offset();
                let call_end = node.location().end_offset();

                if !self.is_suppressed(offset, block_end) {
                    let flagged = self.check_block(&block_node, b"super");
                    if flagged {
                        self.suppress_range(call_start, call_end);
                    }
                }
            }
        }
        ruby_prism::visit_super_node(self, node);
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'_>) {
        // ForwardingSuperNode: `super { ... }` or `super do ... end` (no explicit args)
        if let Some(block_node) = node.block() {
            let offset = block_node.opening_loc().start_offset();
            let block_end = block_node.closing_loc().end_offset();
            let call_start = node.location().start_offset();
            let call_end = node.location().end_offset();

            if !self.is_suppressed(offset, block_end) {
                let flagged = self.check_block(&block_node, b"super");
                if flagged {
                    self.suppress_range(call_start, call_end);
                }
            }
        }
        ruby_prism::visit_forwarding_super_node(self, node);
    }
}

/// Check if a block corresponds to Parser's `:block` type (not `:itblock` or `:numblock`).
/// Returns false for blocks with `it` parameters (`:itblock` in Parser) or numbered
/// parameters like `_1` (`:numblock` in Parser). Blocks with explicit parameters
/// (`|x, y|`) or no parameters at all are both `:block` type.
fn is_explicit_block(block: ruby_prism::BlockNode<'_>) -> bool {
    match block.parameters() {
        Some(p) => {
            // ItParametersNode → :itblock, NumberedParametersNode → :numblock
            p.as_it_parameters_node().is_none() && p.as_numbered_parameters_node().is_none()
        }
        // No parameters → :block type
        None => true,
    }
}

/// Check if a method name is a Ruby operator method.
/// Matches RuboCop's `OPERATOR_METHODS` from `MethodIdentifierPredicates`.
fn is_operator_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"|" | b"^"
            | b"&"
            | b"<=>"
            | b"=="
            | b"==="
            | b"=~"
            | b">"
            | b">="
            | b"<"
            | b"<="
            | b"<<"
            | b">>"
            | b"+"
            | b"-"
            | b"*"
            | b"/"
            | b"%"
            | b"**"
            | b"~"
            | b"+@"
            | b"-@"
            | b"!@"
            | b"~@"
            | b"[]"
            | b"[]="
            | b"!"
            | b"!="
            | b"!~"
            | b"`"
    )
}

/// Check if a block's body contains rescue or ensure clauses.
/// In Prism, this manifests as a BeginNode body with rescue_clause or ensure_clause.
fn block_has_rescue_or_ensure(block_node: &ruby_prism::BlockNode<'_>) -> bool {
    if let Some(body) = block_node.body() {
        if let Some(begin_node) = body.as_begin_node() {
            return begin_node.rescue_clause().is_some() || begin_node.ensure_clause().is_some();
        }
    }
    false
}

/// Recursively collect blocks inside argument expressions of non-parenthesized
/// method calls. These blocks must be ignored because changing `{...}` to
/// `do...end` (or vice versa) would change block binding.
fn collect_ignored_blocks(node: &ruby_prism::Node<'_>, ignored: &mut HashSet<usize>) {
    // CallNode: mark its block as ignored, recurse into receiver + arguments
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                ignored.insert(block_node.opening_loc().start_offset());
            }
        }
        if let Some(receiver) = call.receiver() {
            collect_ignored_blocks(&receiver, ignored);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_ignored_blocks(&arg, ignored);
            }
        }
        return;
    }

    // KeywordHashNode (unbraced hash in argument position)
    if let Some(kwh) = node.as_keyword_hash_node() {
        for element in kwh.elements().iter() {
            collect_ignored_blocks(&element, ignored);
        }
        return;
    }

    // HashNode (braced hash) — skip per vendor logic (braces prevent rebinding)
    if node.as_hash_node().is_some() {
        return;
    }

    // AssocNode (key: value pair)
    if let Some(assoc) = node.as_assoc_node() {
        collect_ignored_blocks(&assoc.value(), ignored);
        return;
    }

    // AssocSplatNode (**hash)
    if let Some(splat) = node.as_assoc_splat_node() {
        if let Some(value) = splat.value() {
            collect_ignored_blocks(&value, ignored);
        }
        return;
    }

    // LambdaNode (`-> { ... }`) — in Parser AST, lambdas are block nodes.
    // RuboCop's `get_blocks` yields them, so `ignore_node` is called on the
    // lambda block. Any blocks nested inside the lambda body are then
    // suppressed by `part_of_ignored_node?`. We must recurse into the lambda's
    // body to find and ignore nested blocks.
    if let Some(lambda) = node.as_lambda_node() {
        if let Some(body) = lambda.body() {
            collect_ignored_blocks_from_body(&body, ignored);
        }
    }
}

/// Recursively find all blocks inside a node body and mark them as ignored.
/// Used for lambda bodies where we need to suppress all nested blocks.
fn collect_ignored_blocks_from_body(node: &ruby_prism::Node<'_>, ignored: &mut HashSet<usize>) {
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                ignored.insert(block_node.opening_loc().start_offset());
            }
        }
        if let Some(receiver) = call.receiver() {
            collect_ignored_blocks_from_body(&receiver, ignored);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_ignored_blocks_from_body(&arg, ignored);
            }
        }
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                if let Some(body) = block_node.body() {
                    collect_ignored_blocks_from_body(&body, ignored);
                }
            }
        }
        return;
    }

    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_ignored_blocks_from_body(&stmt, ignored);
        }
        return;
    }

    // Assignment nodes — recurse into the value expression
    // e.g., `result = items.find { |item| ... }` inside a lambda body
    if let Some(write) = node.as_local_variable_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    if let Some(write) = node.as_instance_variable_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    if let Some(write) = node.as_class_variable_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    if let Some(write) = node.as_global_variable_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    if let Some(write) = node.as_constant_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    if let Some(write) = node.as_local_variable_operator_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    if let Some(write) = node.as_instance_variable_operator_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }
    // Multi-write: a, b = expr
    if let Some(write) = node.as_multi_write_node() {
        collect_ignored_blocks_from_body(&write.value(), ignored);
        return;
    }

    // IfNode, UnlessNode, etc. — recurse into their bodies for completeness
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                collect_ignored_blocks_from_body(&stmt, ignored);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_ignored_blocks_from_body(&subsequent, ignored);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockDelimiters, "cops/style/block_delimiters");

    #[test]
    fn no_offense_proc_in_keyword_arg() {
        // Proc block in keyword arg without parens — changing braces would change semantics
        let source = b"my_method :arg1, arg2: proc {\n  something\n}, arg3: :another_value\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag proc block in keyword argument position, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_safe_navigation_non_parenthesized() {
        // Safe-navigation call with non-parenthesized block arg
        let source = b"foo&.bar baz {\n  y\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block in safe-navigation non-parenthesized call, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_chained_method_block_in_arg() {
        // Block result chained and used as argument
        let source = b"foo bar + baz {\n}.qux.quux\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag chained block in non-parenthesized arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_lambda_in_keyword_arg_without_parens() {
        // lambda block in keyword arg of non-parenthesized call
        let source = b"foo :bar, :baz, qux: lambda { |a|\n  bar a\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag lambda block in keyword arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_nested_in_non_parens_arg() {
        // text html { body { ... } } — html's block is in non-parenthesized arg of text,
        // body's block is inside html's ignored block => both suppressed
        let source = b"text html {\n  body {\n    input(type: 'text')\n  }\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag blocks nested in non-parenthesized arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_deeply_nested_in_non_parens_arg() {
        // foo browser { text html { body { ... } } } — browser's block is in foo's
        // non-parens arg, all inner blocks are suppressed
        let source =
            b"foo browser {\n  text html {\n    body {\n      input(type: 'text')\n    }\n  }\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag deeply nested blocks in non-parens arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn offense_only_outermost_nested_braces() {
        // When multiple multi-line brace blocks are nested, only the outermost
        // should be flagged (RuboCop's ignore_node behavior)
        let source = b"items.map {\n  items.select {\n    true\n  }\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag only outermost multi-line brace block, got: {:?}",
            diags
        );
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn offense_only_outermost_in_chain() {
        // Chained blocks: a.select { ... }.reject { ... }.each { ... }
        // RuboCop flags only the outermost (last in chain) in Parser AST.
        // In Prism, the outermost CallNode covers the entire chain, so
        // suppressing via the call node's range suppresses inner blocks.
        let source = b"items.select {\n  x.valid?\n}.reject {\n  x.empty?\n}.each {\n  puts x\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag only the outermost chained block, got: {:?}",
            diags
        );
        // The outermost block in Prism is the top-level CallNode (.each)
        assert_eq!(diags[0].location.line, 5, "Should flag .each at line 5");
    }

    #[test]
    fn offense_two_block_chain() {
        // a.select { ... }.reject { ... } — only outermost flagged
        let source = b"items.select {\n  x.valid?\n}.reject {\n  x.empty?\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag only outermost in two-block chain, got: {:?}",
            diags
        );
        assert_eq!(diags[0].location.line, 3, "Should flag .reject at line 3");
    }

    #[test]
    fn offense_block_in_operator_arg() {
        // `a + b { ... }` — operator method with single block-bearing arg.
        // RuboCop does NOT ignore the block (single_argument_operator_method? skips
        // the ignore logic), so the multi-line brace block should be flagged.
        let source = b"a + b {\n  c\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multi-line brace block in operator arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_do_end_single_line_rescue_array() {
        // Single-line do-end with rescue that has array exception type
        // This needs do-end because {} + rescue + array creates ambiguity
        let source = b"foo do next unless bar; rescue StandardError; end\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag single-line do-end with rescue+semicolon, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_block_in_string_concat_operator() {
        // `a + b.collect { ... }.join` — the `+` operator's argument is a send node
        // (not a block), so RuboCop does NOT skip ignore_node logic. The block is
        // found via get_blocks recursion and ignored.
        let source = b"result = prefix + items.collect { |i|\n  i.to_s\n}.join(\", \")\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block inside operator argument chain, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_block_in_string_concat_multi_plus() {
        // Multiple `+` concatenation: `a + b.map { }.join + c`
        let source = b"x = \"prefix\" + items.map { |i|\n  i.to_s\n}.join(\", \") + \"suffix\"\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block in multi-plus concat, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_block_as_rhs_of_or_assign_with_plus() {
        // `@x ||= a + b.collect { ... }.flatten` — the `+` operator's argument
        // is a send node (.flatten), so RuboCop ignores the inner block.
        let source = b"@x ||= prefix + items.collect { |m|\n  m.ancestors\n}.flatten\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block in operator arg of ||= expression, got: {:?}",
            diags
        );
    }

    #[test]
    fn offense_super_multi_line_braces() {
        // `super(args) { ... }` — multi-line brace block on super should be flagged
        let source = b"super(num_waits) {\n  yield if block_given?\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multi-line brace block on super, got: {:?}",
            diags
        );
    }

    #[test]
    fn offense_super_single_line_do_end() {
        // `super(*args) do |item| yielder << item end` — single-line do-end on super
        let source = b"super(*args) do |item| yielder << item end\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag single-line do-end block on super, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_super_multi_line_do_end() {
        // `super(args) do ... end` — correct style for multi-line
        let source = b"super(num_waits) do\n  yield if block_given?\nend\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag multi-line do-end block on super, got: {:?}",
            diags
        );
    }

    #[test]
    fn offense_forwarding_super_multi_line_braces() {
        // `super { ... }` with ForwardingSuperNode — multi-line braces should be flagged
        let source = b"super {\n  yield\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multi-line brace block on bare super, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_block_in_parenthesized_arg() {
        // Block inside a parenthesized method call argument — parenthesized calls
        // don't trigger ignore_node, so block is checked normally.
        // In line_count_based, multi-line braces = offense.
        let source = b"foo(bar.map { |x|\n  x.to_s\n})\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multi-line brace block in parenthesized arg, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_hash_bracket_with_block() {
        // Hash[list.map { ... }] — `[]` is a non-parenthesized method call
        let source = b"Hash[list.map { |k, v|\n  [k, v.to_s]\n}.sort_by(&:first)]\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block inside Hash[] argument, got: {:?}",
            diags
        );
    }

    #[test]
    fn offense_multi_line_braces_when_chained() {
        let source = b"items.map { |x|\n  x.to_s\n}.join(\", \")\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multi-line brace block even when chained, got: {:?}",
            diags
        );
    }

    #[test]
    fn offense_multi_line_braces_when_assigned() {
        let source = b"result = items.map { |x|\n  x.to_s\n}\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multi-line brace block even when assigned, got: {:?}",
            diags
        );
    }

    #[test]
    fn no_offense_lambda_body_assignment_with_block() {
        // Block inside assignment inside lambda body in keyword arg
        let source = b"render node: -> {\n  result = items.find { |item|\n    item.name == \"test\"\n  }\n} do\n  puts \"rendered\"\nend\n";
        let diags = crate::testutil::run_cop_full(&BlockDelimiters, source);
        assert!(
            diags.is_empty(),
            "Should not flag block inside assignment in lambda body in keyword arg, got: {:?}",
            diags
        );
    }
}
