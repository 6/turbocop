use ruby_prism::Visit;

use crate::cop::shared::method_identifier_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Fixed FP=6 and FN=4 with two behavior corrections:
/// - Top-level `ParenthesesNode` returns are treated as `Opaque`, matching
///   Parser's `:begin` wrappers.
///
/// ## Corpus investigation (2026-03-13)
///
/// Corpus oracle reported FP=137, FN=51.
///
/// Root cause: RuboCop's `extract_conditional_branches` synthesizes `s(:nil)`
/// for conditionals without an else branch (`branches.push(s(:nil)) unless
/// node.else_branch`). Our code was not doing this, causing methods like
/// `def foo; true if bar; end` to appear all-boolean (returns=[true]) when
/// RuboCop sees (returns=[true, nil]) and does NOT flag them.
///
/// Fix: push `NonBooleanLiteral` (representing implicit nil) when IfNode,
/// UnlessNode, CaseNode, or CaseMatchNode has no else branch. This matches
/// RuboCop's behavior exactly.
///
/// Previous doc comment said nil synthesis was removed because it "produced
/// FNs". That analysis was wrong — RuboCop clearly does synthesize nil.
/// The FP=137 was a direct consequence of not doing so.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=1, FN=19. All verified fixed.
///
/// FP=1: `read_node?` method containing `yield` calls. RuboCop treats
/// `:yield` as `call_type?`, so conservative mode skips it. Our code was
/// classifying YieldNode as Opaque instead of Unknown.
/// Fix: classify YieldNode as Unknown in `classify_node`.
///
/// FN=15 (parenthesized expressions): Methods returning `(x == y)` etc.
/// RuboCop's `return_values` unwraps `:begin` (from parens) via `begin_type?`
/// check, exposing the inner boolean expression. Our code treated
/// ParenthesesNode as Opaque at ALL levels. Fix: unwrap ParenthesesNode in
/// `collect_implicit_return` (method body level) while keeping it Opaque in
/// `collect_and_or_leaves` (leaf level in ||/&& chains).
///
/// FN=4 (if/elsif without else): Methods like `if c1; true; elsif c2; false; end`.
/// RuboCop's `IfNode#branches` flattens elsif chains but EXCLUDES nil for the
/// missing else on inner elsifs (`!else?` returns `[if_branch]` only). And
/// `extract_conditional_branches` only pushes nil based on the OUTER if's
/// `else_branch`, which is the elsif node (truthy). So nil is NOT pushed for
/// `if/elsif/end` chains, making all-boolean branches an offense.
/// Fix: iterate through if/elsif chain instead of recursing, push nil only
/// when top-level if has no subsequent.
///
/// ## Corpus investigation (2026-03-14) — batch 2
///
/// Corpus oracle reported FP=5, FN=5.
///
/// FP=5: Multi-statement methods with parenthesized comparisons as last
/// statement (e.g., `def foo; log(); (x > y); end`). Root cause:
/// `collect_implicit_return` unwrapped ParenthesesNode at all levels,
/// but RuboCop's `last_value` only unwraps one level of `:begin`. In
/// multi-statement bodies, the parens-`:begin` is the second level and
/// is NOT unwrapped. Fix: only unwrap ParenthesesNode when it's the sole
/// child of a StatementsNode (matching Parser gem single-statement behavior).
///
/// FN=5: Predicate methods with yield and implicit nil return. Root cause:
/// YieldNode was classified as Unknown (triggering conservative skip), but
/// RuboCop's call_type? = send_type? || csend_type? does NOT include :yield.
/// Fix: classify YieldNode as Opaque instead of Unknown.
///
/// ## Corpus investigation (2026-03-14) — batch 3
///
/// Corpus oracle reported FP=0, FN=7. All 7 verified fixed.
///
/// FN root cause 1 (4 FN): Parenthesized expressions in conditional branch
/// values (e.g., `if c; (x == y); else; false; end`). RuboCop's `last_value`
/// unwraps `:begin` (parens) for branch values via `begin_type?` check. Our
/// IfNode/CaseNode/etc. handlers extracted the last item from StatementsNode
/// directly, bypassing the ParenthesesNode unwrapping logic.
/// Fix: extracted `collect_statements_return` helper that processes
/// StatementsNode with proper single-statement ParenthesesNode unwrapping,
/// and used it in all conditional branch handlers.
///
/// FN root cause 2 (3 FN): Parenthesized expressions in `return` arguments
/// (e.g., `return (i != 0)`). In Parser gem, `return (expr)` does NOT produce
/// a `:begin` wrapper — the parens are treated as argument grouping. Prism
/// preserves the ParenthesesNode. Fix: added `collect_return_arg` helper that
/// unwraps ParenthesesNode before recursing, used in both ReturnFinder and
/// the ReturnNode handler in `collect_implicit_return`.
pub struct PredicateMethod;

const MSG_PREDICATE: &str = "Predicate method names should end with `?`.";
const MSG_NON_PREDICATE: &str = "Non-predicate method names should not end with `?`.";

const DEFAULT_ALLOWED_METHODS: &[&str] = &["call"];
const DEFAULT_WAYWARD_PREDICATES: &[&str] = &["infinite?", "nonzero?"];

/// Comparison methods whose return value is boolean.
/// Note: `<=>` is intentionally excluded — it returns Integer (-1, 0, 1), not boolean.
const COMPARISON_METHODS: &[&[u8]] = &[
    b"==", b"!=", b"<", b">", b"<=", b">=", b"===", b"match?", b"equal?", b"eql?",
];

/// Classification of a return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnType {
    /// true, false, comparison, predicate call, negation
    Boolean,
    /// string, integer, float, symbol, nil, array, hash, regex, etc.
    NonBooleanLiteral,
    /// super or forwarding_super
    Super,
    /// method call, variable, or anything we can't classify
    Unknown,
    /// Opaque construct (rescue/ensure) whose return type can't be decomposed.
    /// Prevents all_return_values_boolean and potential_non_predicate from triggering,
    /// but does NOT make the method "acceptable" in conservative mode.
    Opaque,
}

impl Cop for PredicateMethod {
    fn name(&self) -> &'static str {
        "Naming/PredicateMethod"
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
        let mode = config.get_str("Mode", "conservative");
        let conservative = mode == "conservative";

        let allowed_methods_cfg = config.get_string_array("AllowedMethods");
        let allowed_methods: Vec<String> = allowed_methods_cfg.unwrap_or_else(|| {
            DEFAULT_ALLOWED_METHODS
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();
        let compiled_patterns: Vec<regex::Regex> = allowed_patterns
            .iter()
            .filter_map(|p| {
                let normalized = normalize_ruby_regex(p);
                regex::Regex::new(&normalized).ok()
            })
            .collect();

        let allow_bang = config.get_bool("AllowBangMethods", false);

        let wayward_cfg = config.get_string_array("WaywardPredicates");
        let wayward: Vec<String> = wayward_cfg.unwrap_or_else(|| {
            DEFAULT_WAYWARD_PREDICATES
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

        let mut visitor = PredicateMethodVisitor {
            cop: self,
            source,
            conservative,
            allowed_methods,
            compiled_patterns,
            allow_bang,
            wayward,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct PredicateMethodVisitor<'a> {
    cop: &'a PredicateMethod,
    source: &'a SourceFile,
    conservative: bool,
    allowed_methods: Vec<String>,
    compiled_patterns: Vec<regex::Regex>,
    allow_bang: bool,
    wayward: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl PredicateMethodVisitor<'_> {
    fn check_method(&mut self, node: &ruby_prism::DefNode<'_>) {
        let method_name = node.name().as_slice();
        let method_str = match std::str::from_utf8(method_name) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Skip initialize
        if method_str == "initialize" {
            return;
        }

        // Skip operator methods
        if method_identifier_predicates::is_operator_method(method_name) {
            return;
        }

        // Skip empty body
        if node.body().is_none() {
            return;
        }

        // Skip allowed methods
        if self.allowed_methods.iter().any(|a| a == method_str) {
            return;
        }

        // Skip allowed patterns
        if self
            .compiled_patterns
            .iter()
            .any(|re| re.is_match(method_str))
        {
            return;
        }

        // Skip bang methods if configured
        if self.allow_bang && method_str.ends_with('!') {
            return;
        }

        let body = node.body().unwrap();

        // Collect all return types from the method body
        let return_types = collect_all_return_types(&body, &self.wayward);

        // In conservative mode: if any return type is Super or Unknown, the method is acceptable
        if self.conservative
            && return_types
                .iter()
                .any(|rt| *rt == ReturnType::Super || *rt == ReturnType::Unknown)
        {
            return;
        }

        let is_predicate_name = method_str.ends_with('?');

        if is_predicate_name {
            // Method ends with ? but returns non-boolean literals
            if potential_non_predicate(&return_types, self.conservative) {
                let name_loc = node.name_loc();
                let (line, column) = self.source.offset_to_line_col(name_loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    MSG_NON_PREDICATE.to_string(),
                ));
            }
        } else {
            // Method does NOT end with ? but all return values are boolean
            if all_return_values_boolean(&return_types) {
                let name_loc = node.name_loc();
                let (line, column) = self.source.offset_to_line_col(name_loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    MSG_PREDICATE.to_string(),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for PredicateMethodVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.check_method(node);
        // Recurse into def body to find nested defs (inside class << self,
        // singleton classes, blocks, etc.) — each nested def is checked independently
        ruby_prism::visit_def_node(self, node);
    }

    // Stop at class/module boundaries
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Do recurse into classes to find defs
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        // Do recurse into modules to find defs
        ruby_prism::visit_module_node(self, node);
    }
}

/// Normalize a Ruby regex pattern to Rust regex syntax.
/// Strips surrounding `/` delimiters (and optional flags), and converts
/// Ruby-specific anchors to Rust equivalents.
fn normalize_ruby_regex(pattern: &str) -> String {
    let mut s = pattern.trim().to_string();

    // Strip surrounding / delimiters (and optional flags like /i)
    if s.starts_with('/') {
        s.remove(0);
        if let Some(last_slash) = s.rfind('/') {
            s.truncate(last_slash);
        }
    }

    // Convert Ruby anchors to Rust equivalents
    s = s
        .replace("\\A", "^")
        .replace("\\z", "$")
        .replace("\\Z", "$");
    s
}

/// Check if all return values are boolean (excluding Super).
/// Returns true only if there's at least one boolean and all non-Super values are boolean.
fn all_return_values_boolean(return_types: &[ReturnType]) -> bool {
    let non_super: Vec<_> = return_types
        .iter()
        .filter(|rt| **rt != ReturnType::Super)
        .collect();
    if non_super.is_empty() {
        return false;
    }
    non_super.iter().all(|rt| **rt == ReturnType::Boolean)
}

/// Check if a predicate method (ending with ?) has non-boolean return values.
fn potential_non_predicate(return_types: &[ReturnType], conservative: bool) -> bool {
    // In conservative mode: if any return value is boolean, the method name is acceptable
    if conservative && return_types.contains(&ReturnType::Boolean) {
        return false;
    }
    // Check if any return value is a non-boolean literal
    return_types.contains(&ReturnType::NonBooleanLiteral)
}

/// Collect all return types from a method body.
fn collect_all_return_types(body: &ruby_prism::Node<'_>, wayward: &[String]) -> Vec<ReturnType> {
    let mut return_types = Vec::new();

    // 1. Collect explicit return statements
    let mut return_finder = ReturnFinder {
        returns: Vec::new(),
        wayward: wayward.to_vec(),
    };
    return_finder.visit(body);
    return_types.extend(return_finder.returns);

    // 2. Collect the implicit return (last expression in body)
    collect_implicit_return(body, &mut return_types, wayward);

    return_types
}

/// Visitor to find all explicit `return` statements in a method body.
/// Collects ReturnType values directly by using `collect_implicit_return`
/// on single return arguments, so compound expressions (and/or/if/case)
/// are properly decomposed instead of falling through to Unknown.
struct ReturnFinder {
    returns: Vec<ReturnType>,
    wayward: Vec<String>,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        match node.arguments() {
            None => {
                self.returns.push(ReturnType::NonBooleanLiteral);
            }
            Some(args) => {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    self.returns.push(ReturnType::NonBooleanLiteral);
                } else {
                    // Use collect_return_arg to unwrap ParenthesesNode (Parser
                    // strips :begin for return args) and recurse into compound
                    // expressions (AndNode, OrNode, IfNode, etc.)
                    collect_return_arg(&arg_list[0], &mut self.returns, &self.wayward);
                }
            }
        }
    }

    // NOTE: We intentionally do NOT stop at nested defs, classes, or modules.
    // RuboCop uses `node.each_descendant(:return)` which traverses the entire
    // subtree without stopping at scope boundaries. This means `return` statements
    // inside nested defs/classes/modules "leak" into the outer method's return
    // value analysis. While semantically incorrect (a `return` inside a nested def
    // returns from that def, not the outer method), we match RuboCop's behavior
    // for corpus conformance.
}

/// Process a StatementsNode to collect branch return types.
/// Handles ParenthesesNode unwrapping for single-statement bodies, matching
/// RuboCop's `last_value` which unwraps one level of `:begin`.
fn collect_statements_return(
    stmts: &ruby_prism::StatementsNode<'_>,
    returns: &mut Vec<ReturnType>,
    wayward: &[String],
) {
    let body: Vec<_> = stmts.body().iter().collect();
    if let Some(last) = body.last() {
        // When the body has exactly one statement and it's a ParenthesesNode,
        // unwrap it. This matches Parser gem where single-statement bodies
        // are NOT wrapped in an outer :begin — the parens-:begin IS the body,
        // and RuboCop's last_value unwraps it.
        if body.len() == 1 {
            if let Some(paren) = last.as_parentheses_node() {
                if let Some(inner) = paren.body() {
                    collect_implicit_return(&inner, returns, wayward);
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
                return;
            }
        }
        collect_implicit_return(last, returns, wayward);
    } else {
        returns.push(ReturnType::NonBooleanLiteral);
    }
}

/// Collect return type from a return argument, unwrapping ParenthesesNode.
/// In Parser gem, `return (expr)` does NOT produce a `:begin` wrapper — the
/// parens are treated as argument grouping. Prism preserves ParenthesesNode,
/// so we unwrap it here to match Parser's behavior.
fn collect_return_arg(
    node: &ruby_prism::Node<'_>,
    returns: &mut Vec<ReturnType>,
    wayward: &[String],
) {
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(inner) = paren.body() {
            collect_implicit_return(&inner, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }
    collect_implicit_return(node, returns, wayward);
}

/// Collect the implicit return type(s) from a node.
fn collect_implicit_return(
    node: &ruby_prism::Node<'_>,
    returns: &mut Vec<ReturnType>,
    wayward: &[String],
) {
    // StatementsNode (method body) -- take last statement
    if let Some(stmts) = node.as_statements_node() {
        let body: Vec<_> = stmts.body().iter().collect();
        if let Some(last) = body.last() {
            // When the body has exactly one statement and it's a ParenthesesNode,
            // unwrap it. This matches Parser gem where single-statement method
            // bodies are NOT wrapped in an outer :begin — the parens-:begin IS
            // the body, and RuboCop's last_value unwraps it.
            // For multi-statement bodies, ParenthesesNode as the last statement
            // falls through to classify_node → Opaque, matching RuboCop's
            // behavior where last_value only unwraps the outer :begin.
            if body.len() == 1 {
                if let Some(paren) = last.as_parentheses_node() {
                    if let Some(inner) = paren.body() {
                        collect_implicit_return(&inner, returns, wayward);
                    } else {
                        returns.push(ReturnType::NonBooleanLiteral);
                    }
                    return;
                }
            }
            collect_implicit_return(last, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // BeginNode -- always treat as Opaque. Bare begin blocks wrap procedural
    // logic whose return type shouldn't make the method a predicate candidate.
    if node.as_begin_node().is_some() {
        returns.push(ReturnType::Opaque);
        return;
    }

    // RescueModifierNode (inline rescue) -- treat as Opaque
    if node.as_rescue_modifier_node().is_some() {
        returns.push(ReturnType::Opaque);
        return;
    }

    // RescueNode -- direct rescue clause on a def body. Treat as Opaque.
    if node.as_rescue_node().is_some() {
        returns.push(ReturnType::Opaque);
        return;
    }

    // IfNode -- iterate through if/elsif chain, collecting branch values.
    // Matches RuboCop's extract_conditional_branches which uses node.branches
    // (a flat list excluding nil for missing else on inner elsifs) plus
    // `branches.push(s(:nil)) unless node.else_branch`. Since node.else_branch
    // returns the elsif node (truthy) when there's an elsif, nil is NOT pushed
    // for `if/elsif/end` chains — only for plain `if/end` with no subsequent.
    if let Some(if_node) = node.as_if_node() {
        let top_has_subsequent = if_node.subsequent().is_some();
        let mut current: Option<ruby_prism::IfNode<'_>> = Some(if_node);
        let mut has_final_else = false;

        while let Some(current_if) = current {
            // Collect from the if/elsif's then-branch
            if let Some(stmts) = current_if.statements() {
                collect_statements_return(&stmts, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }

            // Advance to next in chain
            match current_if.subsequent() {
                Some(sub) => {
                    if let Some(next_if) = sub.as_if_node() {
                        current = Some(next_if);
                    } else if let Some(else_node) = sub.as_else_node() {
                        has_final_else = true;
                        if let Some(stmts) = else_node.statements() {
                            collect_statements_return(&stmts, returns, wayward);
                        } else {
                            returns.push(ReturnType::NonBooleanLiteral);
                        }
                        current = None;
                    } else {
                        current = None;
                    }
                }
                None => {
                    current = None;
                }
            }
        }

        // Push nil for missing else ONLY if the top-level if has no subsequent.
        // When there's an elsif, RuboCop's node.else_branch returns the elsif
        // node (truthy), so nil is NOT pushed — matching RuboCop's behavior.
        if !has_final_else && !top_has_subsequent {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // UnlessNode
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            collect_statements_return(&stmts, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }

        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                collect_statements_return(&stmts, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            // Missing else branch: implicit nil return
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // CaseNode
    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    collect_statements_return(&stmts, returns, wayward);
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                collect_statements_return(&stmts, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            // Missing else branch: implicit nil return
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // CaseMatchNode (case...in...end pattern matching)
    if let Some(case_match) = node.as_case_match_node() {
        for condition in case_match.conditions().iter() {
            if let Some(in_node) = condition.as_in_node() {
                if let Some(stmts) = in_node.statements() {
                    collect_statements_return(&stmts, returns, wayward);
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
            }
        }
        if let Some(else_clause) = case_match.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                collect_statements_return(&stmts, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            // Missing else branch: implicit nil return
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // AndNode / OrNode -- decompose via collect_and_or_leaves, which treats
    // ParenthesesNode leaves as Opaque (matching RuboCop's Parser gem behavior
    // where :begin wrappers from parens are not unwrapped by extract_and_or_clauses).
    if node.as_and_node().is_some() || node.as_or_node().is_some() {
        collect_and_or_leaves(node, returns, wayward);
        return;
    }

    // WhileNode / UntilNode
    if let Some(while_node) = node.as_while_node() {
        if let Some(stmts) = while_node.statements() {
            collect_statements_return(&stmts, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        if let Some(stmts) = until_node.statements() {
            collect_statements_return(&stmts, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // ReturnNode -- extract its value, unwrapping ParenthesesNode for return args
    if let Some(ret_node) = node.as_return_node() {
        match ret_node.arguments() {
            None => returns.push(ReturnType::NonBooleanLiteral),
            Some(args) => {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    returns.push(ReturnType::NonBooleanLiteral);
                } else {
                    collect_return_arg(&arg_list[0], returns, wayward);
                }
            }
        }
        return;
    }

    // Leaf node: classify directly.
    returns.push(classify_node(node, wayward));
}

/// Decompose nested and/or chains into leaf return types.
/// ParenthesesNode leaves are treated as Opaque (matching RuboCop's Parser gem
/// where :begin wrappers from parentheses are not unwrapped by
/// extract_and_or_clauses, and are not recognized by boolean_return? or call_type?).
fn collect_and_or_leaves(
    node: &ruby_prism::Node<'_>,
    returns: &mut Vec<ReturnType>,
    wayward: &[String],
) {
    if let Some(or_node) = node.as_or_node() {
        collect_and_or_leaves(&or_node.left(), returns, wayward);
        collect_and_or_leaves(&or_node.right(), returns, wayward);
    } else if let Some(and_node) = node.as_and_node() {
        collect_and_or_leaves(&and_node.left(), returns, wayward);
        collect_and_or_leaves(&and_node.right(), returns, wayward);
    } else if node.as_parentheses_node().is_some() {
        returns.push(ReturnType::Opaque);
    } else {
        returns.push(classify_node(node, wayward));
    }
}

/// Classify a single node as a ReturnType.
fn classify_node(node: &ruby_prism::Node<'_>, wayward: &[String]) -> ReturnType {
    // ParenthesesNode -- treat as Opaque. Parser gem keeps top-level parens as
    // :begin wrappers, and RuboCop doesn't unwrap those wrappers here.
    if let Some(paren) = node.as_parentheses_node() {
        let _ = paren;
        return ReturnType::Opaque;
    }

    // true/false literals
    if node.as_true_node().is_some() || node.as_false_node().is_some() {
        return ReturnType::Boolean;
    }

    // nil
    if node.as_nil_node().is_some() {
        return ReturnType::NonBooleanLiteral;
    }

    // Other literals
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_range_node().is_some()
        || node.as_x_string_node().is_some()
        || node.as_interpolated_x_string_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_encoding_node().is_some()
    {
        return ReturnType::NonBooleanLiteral;
    }

    // self and lambda — not call_type? in RuboCop, so they should NOT trigger
    // conservative mode skip. Use Opaque.
    if node.as_self_node().is_some() || node.as_lambda_node().is_some() {
        return ReturnType::Opaque;
    }

    // super / forwarding_super
    if node.as_super_node().is_some() || node.as_forwarding_super_node().is_some() {
        return ReturnType::Super;
    }

    // CallNode
    if let Some(call) = node.as_call_node() {
        // In RuboCop (Parser gem), call+block (`:block` node) is NOT call_type?,
        // so it doesn't trigger conservative skip. But call+block_argument
        // (e.g., `foo(&:bar)`) IS still a `:send` node (call_type?).
        // In Prism, both set call.block().is_some(), so distinguish them:
        if let Some(block) = call.block() {
            if block.as_block_node().is_some() {
                // Block body (do..end / {}) — not call_type? in RuboCop
                return ReturnType::Opaque;
            }
            // BlockArgumentNode (e.g., &:foo, &block) — still a call, fall through
        }

        let method_name = call.name().as_slice();

        // Negation: !x
        if method_name == b"!" && call.receiver().is_some() && call.arguments().is_none() {
            return ReturnType::Boolean;
        }

        // Comparison methods
        if COMPARISON_METHODS.contains(&method_name) {
            return ReturnType::Boolean;
        }

        // Predicate method calls (ending in ?) that are not wayward
        if method_name.ends_with(b"?") {
            let method_str = std::str::from_utf8(method_name).unwrap_or("");
            if !wayward.iter().any(|w| w == method_str) {
                return ReturnType::Boolean;
            }
            return ReturnType::Unknown;
        }

        // Any other method call
        return ReturnType::Unknown;
    }

    // Assignment nodes (x = ..., @x = ..., @x ||= ..., etc.)
    // These are NOT call_type? in RuboCop, so they should NOT make a method
    // "acceptable" in conservative mode. Classify as Opaque to prevent
    // conservative-mode skip while not counting as boolean or non-boolean literal.
    if node.as_local_variable_write_node().is_some()
        || node.as_instance_variable_write_node().is_some()
        || node.as_class_variable_write_node().is_some()
        || node.as_global_variable_write_node().is_some()
        || node.as_constant_write_node().is_some()
        || node.as_constant_path_write_node().is_some()
        || node.as_local_variable_or_write_node().is_some()
        || node.as_instance_variable_or_write_node().is_some()
        || node.as_class_variable_or_write_node().is_some()
        || node.as_global_variable_or_write_node().is_some()
        || node.as_constant_or_write_node().is_some()
        || node.as_constant_path_or_write_node().is_some()
        || node.as_local_variable_and_write_node().is_some()
        || node.as_instance_variable_and_write_node().is_some()
        || node.as_class_variable_and_write_node().is_some()
        || node.as_global_variable_and_write_node().is_some()
        || node.as_constant_and_write_node().is_some()
        || node.as_constant_path_and_write_node().is_some()
        || node.as_local_variable_operator_write_node().is_some()
        || node.as_instance_variable_operator_write_node().is_some()
        || node.as_class_variable_operator_write_node().is_some()
        || node.as_global_variable_operator_write_node().is_some()
        || node.as_constant_operator_write_node().is_some()
        || node.as_constant_path_operator_write_node().is_some()
        || node.as_multi_write_node().is_some()
    {
        return ReturnType::Opaque;
    }

    // YieldNode — yield is NOT call_type? in RuboCop (call_type? = send_type? ||
    // csend_type?, does NOT include :yield). So yield does NOT trigger the
    // conservative-mode acceptable? skip. Classify as Opaque (not Unknown) so
    // it doesn't trigger conservative skip, but also doesn't count as boolean
    // or non-boolean literal.
    if node.as_yield_node().is_some() {
        return ReturnType::Opaque;
    }

    // Everything else (variables, constants, etc.)
    // Not call_type? in RuboCop, so should NOT trigger conservative skip.
    ReturnType::Opaque
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(PredicateMethod, "cops/naming/predicate_method");

    #[test]
    fn if_elsif_no_else_all_boolean_is_offense() {
        // RuboCop's IfNode#branches excludes nil for missing else on inner elsifs.
        // So if/elsif/end with all-boolean branches IS an offense.
        let code =
            b"def to_boolean(value)\n  if cond1\n    true\n  elsif cond2\n    false\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&PredicateMethod, code);
        assert_eq!(diags.len(), 1, "if/elsif/end all-boolean should be offense");
    }

    #[test]
    fn if_no_else_boolean_is_no_offense() {
        // Simple if/end with boolean and implicit nil → NOT all boolean
        let code = b"def has_feature\n  true if condition\nend\n";
        let diags = crate::testutil::run_cop_full(&PredicateMethod, code);
        assert_eq!(
            diags.len(),
            0,
            "if/end with implicit nil should NOT be offense"
        );
    }

    #[test]
    fn yield_in_conservative_mode_is_acceptable() {
        // yield is NOT call_type? in RuboCop, so conservative mode doesn't skip.
        // But the method has no nil return (if/elsif chain doesn't push nil),
        // so neither all-boolean nor potential-non-predicate triggers.
        let code = b"def read_node?(node, block_pass)\n  if block_pass.any?\n    yield(node)\n  elsif file_open_read?(node.parent)\n    yield(node.parent)\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&PredicateMethod, code);
        assert_eq!(
            diags.len(),
            0,
            "yield with no nil return should not be flagged"
        );
    }
}
