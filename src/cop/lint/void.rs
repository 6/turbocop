use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for operators, variables, literals, lambda, proc and nonmutating
/// methods used in void context.
///
/// ## Investigation findings (2026-03-10)
///
/// Root causes of FPs (670) and FNs (683):
///
/// **FP: each block operator exemption missing** — RuboCop exempts void operators
/// (like `==`, `>=`, `<=>`) inside `each` blocks because the receiver may be an
/// Enumerator used as a filter. Our implementation flagged these operators.
///
/// **FN: void context for last expression** — RuboCop checks the last expression
/// in void contexts: `initialize`, setter methods (`def foo=`), `each`/`tap` blocks,
/// `for` loops, and `ensure` bodies. Our implementation always skipped the last
/// expression.
///
/// **FN: lambda/proc not detected** — `-> { }`, `lambda { }`, `proc { }` in void
/// context were not detected as void expressions.
///
/// **FN: `.freeze` on literal** — `'foo'.freeze` was not treated as entirely literal.
///
/// **FN: single-expression void blocks** — Single-expression `each`/`tap`/`for`
/// bodies and `ensure` bodies were not checked because we required `len > 1`.
///
/// **FP: binary operator with dot and no args** — `a.+` (no args, with dot) should
/// not be flagged; only `a.+(b)` should be.
///
/// Fixes applied: void context tracking via parent node inspection, each block
/// operator exemption, lambda/proc detection, `.freeze` on literal detection,
/// single-expression void body checking, dot-operator-no-args exemption.
///
/// ## Investigation findings (2026-03-11)
///
/// Root causes of remaining FPs (569) and FNs (746):
///
/// **FN: conditional/guard clause unwrapping missing** — RuboCop's `check_expression`
/// does `expr = expr.body if expr.if_type?` to unwrap if/unless/ternary nodes before
/// checking for void literals, vars, consts, self, defined?, and lambda/proc. This
/// means `42 unless condition`, `CONST unless cond`, `x unless cond`, and
/// `condition ? CONST : nil` are all flagged when in non-last position. Key detail:
/// rubocop-ast's `IfNode#node_parts` normalizes `unless` by swapping branches, so
/// `body` always returns the truthy/executing branch. In Prism, `IfNode#statements()`
/// and `UnlessNode#statements()` serve the same purpose. Operators are NOT unwrapped
/// through conditionals (only through `check_void_op`).
///
/// **FN: CheckForMethodsWithNoSideEffects not implemented** — When
/// `CheckForMethodsWithNoSideEffects: true`, RuboCop flags nonmutating methods
/// (sort, flatten, map, collect, etc.) in void context. This config key was read
/// but ignored.
///
/// Fixes applied: conditional unwrapping via `check_conditional_body`, nonmutating
/// method detection via `check_nonmutating` with full method list matching RuboCop's
/// `NONMUTATING_METHODS_WITH_BANG_VERSION` and `METHODS_REPLACEABLE_BY_EACH`.
///
/// ## Investigation findings (2026-03-16)
///
/// **FP: `is_void_def` matched operator methods** — `is_void_def` used
/// `name.ends_with(b"=")` which incorrectly matched `==`, `===`, and `!=` as
/// setter methods. RuboCop uses `assignment_method?` which matches
/// `/^[a-z_]\w*=$/` — only proper setter names (e.g., `name=`) where the char
/// before `=` is alphanumeric or underscore. Fixed to check that the char before
/// the trailing `=` is a word character, excluding operator methods.
///
/// ## Investigation findings (2026-03-16, round 2)
///
/// Root causes of remaining FPs (273) and FNs (688):
///
/// **FP/FN: operator offense reported at wrong location** — nitrocop reported
/// void operators at the whole expression start, while RuboCop reports at the
/// operator selector position (e.g., `==` in `a.should == value`). For multiline
/// expressions, this caused line mismatches (FP at expression start, FN at
/// operator position). Fixed by using `call.message_loc()` for operator offset.
///
/// **FP: single-expression void def bodies** — RuboCop has no `on_def` callback;
/// void context for initialize/setter bodies is handled via `on_begin` which only
/// fires for multi-statement (begin node) bodies. Single-expression def bodies
/// were incorrectly flagged. Fixed by skipping `check_statements` when
/// `body_stmts.len() == 1`.
///
/// **FP: singleton method defs matched as void** — `def self.initialize` and
/// `def self.foo=` were treated as void context, but RuboCop's `void_context?`
/// returns false for `defs` (singleton method) nodes. Fixed by checking
/// `node.receiver().is_some()`.
///
/// **FP: single-expression for loop bodies** — RuboCop has no `on_for` callback.
/// Single-expression for bodies were incorrectly flagged. Fixed similarly to defs.
///
/// **FP: single-expression ensure body operators** — RuboCop's `check_ensure`
/// calls `check_expression` (no `check_void_op`) for single-expression ensure
/// bodies. Operators in single-expression ensure bodies were incorrectly flagged.
/// Fixed with `check_void_expression_no_op` for ensure single-expression case.
///
/// **FN: interpolated strings not detected** — RuboCop considers `dstr`
/// (interpolated strings) as literals via `node.literal?`. Added
/// `InterpolatedStringNode`, `InterpolatedSymbolNode`, and
/// `InterpolatedRegularExpressionNode` detection.
///
/// ## Investigation findings (2026-03-17)
///
/// **FP: `**` (exponentiation) incorrectly in void operators list** — RuboCop's
/// `BINARY_OPERATORS` is `%i[* / % + - == === != < > <= >= <=>]` — it does NOT
/// include `**`. Removed `**` from `void_operator_name_offset()` match list.
/// This fixes 19 FPs (17 from ruby__rbs, 2 remaining from rufo may be
/// location mismatches for unary operators).
///
/// Remaining gaps (~0 FP, ~453 FN): FNs are diverse across jruby, eye, natalie
/// repos — mostly location mismatches for complex multiline patterns and missing
/// operator detection in deeply nested contexts. 9 FNs from rufo repo are
/// hash/array with interpolation in void context (fixed in round 2).
///
/// ## Investigation findings (2026-03-17, round 2)
///
/// Root causes of remaining FP (1) and FNs (453):
///
/// **FP: `proc { _1 + _2 }` flagged as void** — RuboCop's `proc?` matcher uses
/// `(block (send nil? :proc) ...)` which only matches `:block` AST type, NOT
/// `:numblock` or `:itblock`. Blocks with numbered params (`_1`, `_2`) or
/// it-keyword params are parsed as `numblock`/`itblock` in Parser 3.4+, so
/// `lambda_or_proc?` returns false. In Prism, these are `BlockNode` with
/// `NumberedParametersNode` or `ItParametersNode` — must exclude from detection.
///
/// **FN: `in_each_block` propagated to nested blocks** — The `in_each_block` flag
/// was set when entering an `each` block and remained true for all descendant
/// nodes, including nested blocks like `it "..." do ... end`. RuboCop checks
/// `node.each_ancestor(:any_block).first` — the nearest ancestor block, not all
/// ancestors. Fixed by resetting `in_each_block = false` in `visit_block_node`.
///
/// **FN: `[]=` not matched as setter** — `is_void_def` required the char before
/// `=` to be alphanumeric/underscore, missing `[]=`. RuboCop's `assignment_method?`
/// is `!comparison_method? && end_with?('=')`, which matches `[]=`. Fixed to use
/// the same comparison-operator exclusion approach.
///
/// **FN: singleton setters excluded** — `is_void_def` skipped all singleton
/// methods (`def self.foo=`). RuboCop's `void_context?` returns true for setter
/// methods via `assignment_method?` regardless of receiver; only `initialize`
/// requires no receiver. Fixed by only excluding receiver for `initialize`.
///
/// ## Investigation findings (2026-03-18)
///
/// **FN: `is_entirely_literal` missing interpolated nodes** — `is_entirely_literal`
/// (used recursively by container checks) did not include `InterpolatedStringNode`,
/// `InterpolatedSymbolNode`, or `InterpolatedRegularExpressionNode`. RuboCop's
/// `entirely_literal?` falls through to `node.literal?` which returns true for
/// `dstr`/`dsym`/`dregx`. This caused `%W(foo #{1})` and `{ "foo #{2}": 1 }` to
/// not be detected as void literals when used as array/hash element types.
///
/// **FN: `AssocSplatNode` rejected in hash literal check** —
/// `is_entirely_literal_container` for hashes only recognized `AssocNode` elements,
/// returning false for `AssocSplatNode` (`{ **x }`). RuboCop's `each_key`/`each_value`
/// skip `kwsplat` nodes (only iterate over `pair` children), so `{ **x }` vacuously
/// passes `entirely_literal?`. Fixed by treating `AssocSplatNode` as vacuously true.
///
/// ## Investigation findings (2026-03-28)
///
/// **FN: recursive literal check missed range nodes** — RuboCop's
/// `entirely_literal?` falls back to `node.literal?`, and `irange`/`erange`
/// return true there. Our `is_entirely_literal` did not include `RangeNode`, so
/// hash literals like `{ foo: ...bar }` and `{ foo: ..bar }` were rejected as
/// non-literal containers and missed in void context. The fix is intentionally
/// scoped to the recursive literal helper only; top-level range expressions stay
/// exempt because `Lint/Void` still skips `range_type?` in the main literal check.
///
/// **Remaining corpus mismatch is outside this cop** — after the `RangeNode`
/// fix, `./target/release/nitrocop --force-default-config --only Lint/Void
/// test/corpus/literal/range.rb` reports the expected offenses at lines 11 and
/// 14 in `mbj__unparser__15c57a1`. The corpus location verifier still shows
/// them as missing because non-default-config runs against that cloned repo fail
/// early with `No lockfile found ...`, so the remaining mismatch is in
/// config/root resolution rather than `Lint/Void` detection.
pub struct Void;

impl Cop for Void {
    fn name(&self) -> &'static str {
        "Lint/Void"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let check_methods = config.get_bool("CheckForMethodsWithNoSideEffects", false);

        let mut visitor = VoidVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_each_block: false,
            check_methods,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Nonmutating methods that have a bang version (e.g., `sort` -> `sort!`).
const NONMUTATING_METHODS_WITH_BANG: &[&[u8]] = &[
    b"capitalize",
    b"chomp",
    b"chop",
    b"compact",
    b"delete_prefix",
    b"delete_suffix",
    b"downcase",
    b"encode",
    b"flatten",
    b"gsub",
    b"lstrip",
    b"merge",
    b"next",
    b"reject",
    b"reverse",
    b"rotate",
    b"rstrip",
    b"scrub",
    b"select",
    b"shuffle",
    b"slice",
    b"sort",
    b"sort_by",
    b"squeeze",
    b"strip",
    b"sub",
    b"succ",
    b"swapcase",
    b"tr",
    b"tr_s",
    b"transform_values",
    b"unicode_normalize",
    b"uniq",
    b"upcase",
];

/// Methods replaceable by `each` (e.g., `collect`, `map`).
const METHODS_REPLACEABLE_BY_EACH: &[&[u8]] = &[b"collect", b"map"];

struct VoidVisitor<'a, 'src> {
    cop: &'a Void,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Whether we are currently inside an `each` block body.
    /// Used to exempt void operators (enumerator filter pattern).
    in_each_block: bool,
    /// Whether to check for nonmutating methods without side effects.
    check_methods: bool,
}

impl VoidVisitor<'_, '_> {
    fn check_void_expression(&mut self, stmt: &ruby_prism::Node<'_>) {
        // Check non-operator void expressions (report at expression start)
        if is_void_non_operator(stmt) {
            let loc = stmt.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Void value expression detected.".to_string(),
            ));
            return;
        }
        // Check void operators separately (report at operator/selector position)
        // RuboCop reports operators at node.loc.selector (the operator name)
        if let Some(op_offset) = void_operator_name_offset(stmt, self.in_each_block) {
            let (line, column) = self.source.offset_to_line_col(op_offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Void value expression detected.".to_string(),
            ));
            return;
        }
        // RuboCop unwraps if/unless/ternary nodes to check their body for void
        // non-operator expressions (literals, vars, consts, self, defined?, lambda/proc).
        // e.g., `42 unless condition` flags `42` as void literal.
        // Operators are NOT checked through this path (only through direct check_void_op).
        self.check_conditional_body(stmt);

        // CheckForMethodsWithNoSideEffects: detect nonmutating methods in void context
        if self.check_methods {
            self.check_nonmutating(stmt);
        }
    }

    /// Check a single expression in void context for non-operator void expressions only.
    /// Used for single-expression ensure bodies (matches RuboCop's check_expression
    /// which does NOT call check_void_op).
    fn check_void_expression_no_op(&mut self, stmt: &ruby_prism::Node<'_>) {
        if is_void_non_operator(stmt) {
            let loc = stmt.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Void value expression detected.".to_string(),
            ));
            return;
        }
        self.check_conditional_body(stmt);
        if self.check_methods {
            self.check_nonmutating(stmt);
        }
    }

    /// Unwrap if/unless/ternary nodes and check the body for void non-operator expressions.
    /// Matches RuboCop's `expr = expr.body if expr.if_type?` in check_expression.
    fn check_conditional_body(&mut self, stmt: &ruby_prism::Node<'_>) {
        let stmts_opt = if let Some(if_node) = stmt.as_if_node() {
            if_node.statements()
        } else if let Some(unless_node) = stmt.as_unless_node() {
            unless_node.statements()
        } else {
            return;
        };
        if let Some(stmts) = stmts_opt {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() == 1 {
                let inner = &body[0];
                if is_void_non_operator(inner) {
                    let loc = inner.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Void value expression detected.".to_string(),
                    ));
                }
            }
        }
    }

    /// Check for nonmutating methods used in void context.
    /// Only called when `CheckForMethodsWithNoSideEffects` is enabled.
    fn check_nonmutating(&mut self, stmt: &ruby_prism::Node<'_>) {
        // Get the method name from a call node (either direct send or block with send)
        let (method_name, loc) = if let Some(call) = stmt.as_call_node() {
            // Direct send: `x.sort`
            if call.block().is_some() {
                // Block call: `[1,2,3].collect do |n| ... end`
                (call.name().as_slice(), stmt.location())
            } else {
                (call.name().as_slice(), stmt.location())
            }
        } else {
            return;
        };

        let is_nonmutating_bang = NONMUTATING_METHODS_WITH_BANG.contains(&method_name);
        let is_replaceable_by_each = METHODS_REPLACEABLE_BY_EACH.contains(&method_name);

        if !is_nonmutating_bang && !is_replaceable_by_each {
            return;
        }

        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Void value expression detected.".to_string(),
        ));
    }

    /// Check statements in a body, optionally including the last expression
    /// (when in void context).
    fn check_statements(&mut self, body: &[ruby_prism::Node<'_>], void_context: bool) {
        if body.is_empty() {
            return;
        }

        let check_up_to = if void_context {
            body.len()
        } else {
            body.len().saturating_sub(1)
        };

        for stmt in &body[..check_up_to] {
            self.check_void_expression(stmt);
        }
    }
}

/// Check if a node is an `each` or `tap` method call (for void context detection).
fn is_void_context_method(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    matches!(name, b"each" | b"tap")
}

/// Check if a node is specifically an `each` method call.
fn is_each_method(call: &ruby_prism::CallNode<'_>) -> bool {
    call.name().as_slice() == b"each"
}

/// Check if a def node is a void context (initialize or setter method).
/// RuboCop's `void_context?` on DefNode:
///   `(def_type? && method?(:initialize)) || assignment_method?`
/// Where `assignment_method?` is: `!comparison_method? && method_name.end_with?('=')`
/// And comparison operators are: `== === != <= >= > <`
///
/// This means:
/// - `initialize` is void only for instance methods (def_type?, not defs)
/// - Setter methods (`foo=`, `[]=`) are void for BOTH instance and singleton methods
/// - Comparison operators (`==`, `===`, `!=`, `<=`, `>=`) are NOT void
fn is_void_def(node: &ruby_prism::DefNode<'_>) -> bool {
    let name = node.name().as_slice();

    // `initialize` is void only for instance methods (no receiver).
    // RuboCop uses `def_type?` which is false for `defs` (singleton methods).
    if name == b"initialize" && node.receiver().is_none() {
        return true;
    }

    // Assignment method: ends with '=' but is NOT a comparison operator.
    // Matches RuboCop's `!comparison_method? && method_name.end_with?('=')`
    // This works for both instance and singleton methods (def self.foo=).
    if name.last() == Some(&b'=') {
        // Exclude comparison operators: ==, ===, !=, <=, >=
        !matches!(name, b"==" | b"===" | b"!=" | b"<=" | b">=")
    } else {
        false
    }
}

/// Check if a node is a void expression EXCLUDING operators.
/// Used for conditional unwrapping (RuboCop's check_expression checks literals,
/// vars, consts, self, defined?, lambda/proc, but NOT operators).
fn is_void_non_operator(node: &ruby_prism::Node<'_>) -> bool {
    // Simple literals
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_self_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        // Variable reads
        || node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        // Constants
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
        // Containers
        || is_entirely_literal_container(node)
        || node.as_regular_expression_node().is_some()
        // Interpolated strings/symbols/regexps are literals in RuboCop (dstr/dsym in LITERALS)
        || node.as_interpolated_string_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        // Keywords
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_encoding_node().is_some()
        // defined?
        || node.as_defined_node().is_some()
        // Lambda/proc
        || is_void_lambda_or_proc(node)
        // Literal.freeze
        || is_literal_freeze(node)
}

/// Check if a node is a lambda literal `-> { }` that is NOT called.
/// A lambda literal is a `LambdaNode` in Prism. If it's called (e.g., `-> { }.call`),
/// it won't appear as a standalone LambdaNode — it will be wrapped in a CallNode.
fn is_void_lambda_or_proc(node: &ruby_prism::Node<'_>) -> bool {
    // -> { bar } — lambda literal
    if node.as_lambda_node().is_some() {
        return true;
    }

    // lambda { bar } or proc { bar } — these are CallNode with a block
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"lambda" || name == b"proc") && call.receiver().is_none() {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    // RuboCop's `proc?` uses `(block ...)` which only matches :block,
                    // NOT :numblock or :itblock. `lambda?` uses `(any_block ...)` which
                    // matches all. In Prism, numbered/it params are BlockNode with
                    // NumberedParametersNode or ItParametersNode.
                    // For `proc`, skip if using numbered/it params (matches RuboCop).
                    // For `lambda`, always match (matches RuboCop's `any_block`).
                    if name == b"proc" {
                        if let Some(params) = block_node.parameters() {
                            if params.as_numbered_parameters_node().is_some()
                                || params.as_it_parameters_node().is_some()
                            {
                                return false;
                            }
                        }
                    }
                    return true;
                }
            }
        }
        // Proc.new { bar }
        if name == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(c) = recv.as_constant_read_node() {
                    if c.name().as_slice() == b"Proc" {
                        if let Some(block) = call.block() {
                            if let Some(block_node) = block.as_block_node() {
                                // Same numblock/itblock exclusion as proc
                                if let Some(params) = block_node.parameters() {
                                    if params.as_numbered_parameters_node().is_some()
                                        || params.as_it_parameters_node().is_some()
                                    {
                                        return false;
                                    }
                                }
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if a node is `literal.freeze` or `literal&.freeze` (entirely literal when frozen).
fn is_literal_freeze(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"freeze" {
            if let Some(recv) = call.receiver() {
                return is_entirely_literal(&recv);
            }
        }
    }
    false
}

/// Check if a node is an entirely-literal container (array or hash where all
/// elements are literals). Matches RuboCop's `entirely_literal?` method.
fn is_entirely_literal_container(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(arr) = node.as_array_node() {
        arr.elements().iter().all(|e| is_entirely_literal(&e))
    } else if let Some(hash) = node.as_hash_node() {
        hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_entirely_literal(&assoc.key()) && is_entirely_literal(&assoc.value())
            } else if e.as_assoc_splat_node().is_some() {
                // RuboCop's each_key/each_value skip kwsplat nodes — they only
                // iterate over pair children. So { **x } vacuously passes.
                true
            } else {
                false
            }
        })
    } else if let Some(hash) = node.as_keyword_hash_node() {
        hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_entirely_literal(&assoc.key()) && is_entirely_literal(&assoc.value())
            } else {
                false
            }
        })
    } else {
        false
    }
}

/// Recursively check if a node is entirely literal (no variables, method calls, etc.)
/// Matches RuboCop's `entirely_literal?` which uses `node.literal?` in the else branch.
/// `literal?` returns true for interpolated strings/symbols/regexps (dstr/dsym/dregx).
fn is_entirely_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_range_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || is_entirely_literal_container(node)
        || is_literal_freeze(node)
}

/// Return the byte offset of the operator name if the node is a void operator.
/// RuboCop reports void operators at `node.loc.selector` (the operator name position).
/// Returns `None` if the node is not a void operator.
fn void_operator_name_offset(node: &ruby_prism::Node<'_>, in_each_block: bool) -> Option<usize> {
    // Unwrap parentheses nodes to find the inner operator
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let stmts_vec: Vec<_> = stmts.body().iter().collect();
                if stmts_vec.len() == 1 {
                    return void_operator_name_offset(&stmts_vec[0], in_each_block);
                }
            }
        }
        return None;
    }

    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        let is_operator = matches!(
            name,
            b"+" | b"-"
                | b"*"
                | b"/"
                | b"%"
                | b"=="
                | b"==="
                | b"!="
                | b"<"
                | b">"
                | b"<="
                | b">="
                | b"<=>"
                | b"!"
                | b"~"
                | b"-@"
                | b"+@"
        );

        if !is_operator {
            return None;
        }

        // Exempt operators inside `each` blocks (enumerator filter pattern)
        if in_each_block {
            return None;
        }

        // Binary operators called with dot notation and no arguments are NOT void
        // e.g., `a.+` is not flagged, but `a.+(b)` is
        let is_unary = matches!(name, b"!" | b"~" | b"-@" | b"+@");
        if !is_unary && call.call_operator_loc().is_some() && call.arguments().is_none() {
            return None;
        }

        // Return the offset of the message/selector (operator name)
        Some(
            call.message_loc()
                .map_or_else(|| call.location().start_offset(), |loc| loc.start_offset()),
        )
    } else {
        None
    }
}

impl<'pr> Visit<'pr> for VoidVisitor<'_, '_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let body: Vec<_> = node.body().iter().collect();
        // For regular statements nodes (not in special void contexts),
        // check all non-last expressions. Void context handling for
        // for/each/tap/ensure/initialize/setter is done in their respective
        // visit methods.
        self.check_statements(&body, false);
        ruby_prism::visit_statements_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if is_void_def(node) {
            // In void context methods (initialize, setters), ALL expressions
            // including the last are void — but ONLY for multi-statement bodies.
            // RuboCop has no `on_def` callback for void checking; it relies on
            // `on_begin` which only fires for multi-statement (begin node) bodies.
            // Single-expression def bodies are NOT checked.
            if let Some(body) = node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_stmts: Vec<_> = stmts.body().iter().collect();
                    if body_stmts.len() > 1 {
                        // Multi-statement: check all including last (void context)
                        self.check_statements(&body_stmts, true);
                    }
                    // Visit children but don't re-check via visit_statements_node
                    // We need to visit into child nodes for nested blocks, etc.
                    for stmt in &body_stmts {
                        self.visit(stmt);
                    }
                    return;
                }
                // Single expression body (non-StatementsNode) — skip
                self.visit(&body);
                return;
            }
        }
        // Non-void def: let the default visitor handle it
        ruby_prism::visit_def_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check for each/tap blocks with void context
        if is_void_context_method(node) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let is_each = is_each_method(node);
                    let old_in_each = self.in_each_block;
                    if is_each {
                        self.in_each_block = true;
                    }

                    if let Some(body) = block_node.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let body_stmts: Vec<_> = stmts.body().iter().collect();
                            // Void context: check all including last
                            self.check_statements(&body_stmts, true);
                            for stmt in &body_stmts {
                                self.visit(stmt);
                            }
                        } else {
                            // Single expression block body — check it (void context)
                            self.check_void_expression(&body);
                            self.visit(&body);
                        }
                    }

                    self.in_each_block = old_in_each;

                    // Visit receiver and arguments but NOT the block body again
                    if let Some(recv) = node.receiver() {
                        self.visit(&recv);
                    }
                    if let Some(args) = node.arguments() {
                        for arg in args.arguments().iter() {
                            self.visit(&arg);
                        }
                    }
                    return;
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Reset in_each_block when entering nested blocks.
        // RuboCop's check_void_op uses `node.each_ancestor(:any_block).first`
        // to find the nearest ancestor block — only operators directly inside
        // an `each` block body are exempted, not operators inside nested blocks
        // (like `it "..." do ... end` inside `each`).
        // Our visit_call_node handles the direct each/tap block body manually,
        // so this override only fires for blocks reached through the default
        // visitor (i.e., nested blocks).
        let old_in_each = self.in_each_block;
        self.in_each_block = false;
        ruby_prism::visit_block_node(self, node);
        self.in_each_block = old_in_each;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        // For loops are void context — check all expressions including last,
        // but ONLY for multi-statement bodies. RuboCop has no `on_for` callback;
        // it relies on `on_begin` which only fires for multi-statement bodies.
        // Single-expression for loop bodies are NOT checked.
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() > 1 {
                self.check_statements(&body, true);
            }
            for stmt in &body {
                self.visit(stmt);
            }
        }
        // Visit collection
        self.visit(&node.collection());
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        // Ensure bodies are void context.
        // RuboCop's on_ensure/check_ensure handles single-expression ensure bodies
        // with check_expression (no operator check). Multi-expression bodies
        // go through on_begin → check_begin (operators + expressions).
        if let Some(stmts) = node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() > 1 {
                // Multi-expression: check all including operators (void context)
                self.check_statements(&body, true);
            } else if body.len() == 1 {
                // Single expression: check only non-operators (matches RuboCop)
                self.check_void_expression_no_op(&body[0]);
            }
            for stmt in &body {
                self.visit(stmt);
            }
        }
        // Don't use default visitor since we handled statements manually
        // But we still need to visit rescue/else clauses if any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Void, "cops/lint/void");

    fn config_with_check_methods() -> crate::cop::CopConfig {
        let mut config = crate::cop::CopConfig::default();
        config.options.insert(
            "CheckForMethodsWithNoSideEffects".to_string(),
            serde_yml::Value::Bool(true),
        );
        config
    }

    #[test]
    fn test_check_methods_with_no_side_effects() {
        let source = b"x.sort\n^^^^^^ Lint/Void: Void value expression detected.\ntop(x)\n";
        crate::testutil::assert_cop_offenses_full_with_config(
            &Void,
            source,
            config_with_check_methods(),
        );
    }

    #[test]
    fn test_check_methods_no_side_effects_disabled() {
        let source = b"x.sort\ntop(x)\n";
        crate::testutil::assert_cop_no_offenses_full(&Void, source);
    }

    #[test]
    fn test_check_methods_collect_with_block() {
        let source = b"[1,2,3].collect do |n|\n^^^^^^^^^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.\n  n.to_s\nend\n\"done\"\n";
        crate::testutil::assert_cop_offenses_full_with_config(
            &Void,
            source,
            config_with_check_methods(),
        );
    }

    #[test]
    fn test_check_methods_chained() {
        let source =
            b"x.sort.flatten\n^^^^^^^^^^^^^^ Lint/Void: Void value expression detected.\ntop(x)\n";
        crate::testutil::assert_cop_offenses_full_with_config(
            &Void,
            source,
            config_with_check_methods(),
        );
    }

    #[test]
    fn test_void_operator_reported_at_operator_position() {
        // Operators reported at operator selector position (matching RuboCop)
        let source = b"c = foo\nc.bar.should == true\n             ^^ Lint/Void: Void value expression detected.\nc.bar.should == false\n             ^^ Lint/Void: Void value expression detected.\nc\n";
        crate::testutil::assert_cop_offenses_full(&Void, source);
    }

    #[test]
    fn test_multiline_operator_reported_at_operator_position() {
        // Multiline expression: a.foo(args).\n  should == value
        // RuboCop reports at the == selector position on the continuation line
        let source = b"c = foo\na.foo(\n  bar\n).should == true\n         ^^ Lint/Void: Void value expression detected.\na.other\nc\n";
        crate::testutil::assert_cop_offenses_full(&Void, source);
    }

    #[test]
    fn test_method_definition_not_flagged() {
        // def merge; end should NOT be flagged as nonmutating method
        let source = b"def merge\nend\n\ndo_something\n";
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Void,
            source,
            config_with_check_methods(),
        );
    }
}
