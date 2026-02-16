use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PredicateMethod;

const MSG_PREDICATE: &str = "Predicate method names should end with `?`.";
const MSG_NON_PREDICATE: &str = "Non-predicate method names should not end with `?`.";

const DEFAULT_ALLOWED_METHODS: &[&str] = &["call"];
const DEFAULT_WAYWARD_PREDICATES: &[&str] = &["infinite?", "nonzero?"];

/// Known operator method names in Ruby.
const OPERATOR_METHODS: &[&[u8]] = &[
    b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"===",
    b"[]", b"[]=", b"+", b"-", b"*", b"/", b"%", b"**",
    b"<<", b">>", b"&", b"|", b"^", b"~", b"!", b"!~", b"=~",
    b"+@", b"-@",
];

/// Comparison methods whose return value is boolean.
const COMPARISON_METHODS: &[&[u8]] = &[
    b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"===",
    b"match?", b"equal?", b"eql?",
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
    ) -> Vec<Diagnostic> {
        let mode = config.get_str("Mode", "conservative");
        let conservative = mode == "conservative";

        let allowed_methods_cfg = config.get_string_array("AllowedMethods");
        let allowed_methods: Vec<String> = allowed_methods_cfg.unwrap_or_else(|| {
            DEFAULT_ALLOWED_METHODS.iter().map(|s| s.to_string()).collect()
        });

        let allowed_patterns = config.get_string_array("AllowedPatterns").unwrap_or_default();
        let compiled_patterns: Vec<regex::Regex> = allowed_patterns
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect();

        let allow_bang = config.get_bool("AllowBangMethods", false);

        let wayward_cfg = config.get_string_array("WaywardPredicates");
        let wayward: Vec<String> = wayward_cfg.unwrap_or_else(|| {
            DEFAULT_WAYWARD_PREDICATES.iter().map(|s| s.to_string()).collect()
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
        visitor.diagnostics
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
        if is_operator_method(method_name) {
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
        if self.compiled_patterns.iter().any(|re| re.is_match(method_str)) {
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
        // Do NOT recurse into nested defs — each def is checked independently
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

/// Check if a method name is an operator method.
fn is_operator_method(name: &[u8]) -> bool {
    OPERATOR_METHODS.iter().any(|op| *op == name)
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
    if conservative && return_types.iter().any(|rt| *rt == ReturnType::Boolean) {
        return false;
    }
    // Check if any return value is a non-boolean literal
    return_types
        .iter()
        .any(|rt| *rt == ReturnType::NonBooleanLiteral)
}

/// Collect all return types from a method body.
///
/// This collects:
/// 1. All explicit `return` statements (via visitor)
/// 2. The implicit return value (last expression, recursing into conditionals/and/or)
fn collect_all_return_types(body: &ruby_prism::Node<'_>, wayward: &[String]) -> Vec<ReturnType> {
    let mut return_types = Vec::new();

    // 1. Collect explicit return statements
    let mut return_finder = ReturnFinder {
        returns: Vec::new(),
    };
    return_finder.visit(body);

    for ret_node_info in &return_finder.returns {
        match ret_node_info {
            ReturnValue::NoArg => {
                // `return` with no value is implicit nil
                return_types.push(ReturnType::NonBooleanLiteral);
            }
            ReturnValue::MultipleArgs => {
                // Multiple return values => array, not boolean
                return_types.push(ReturnType::NonBooleanLiteral);
            }
            ReturnValue::SingleArg(rt) => {
                return_types.push(*rt);
            }
        }
    }

    // 2. Collect the implicit return (last expression in body)
    collect_implicit_return(body, &mut return_types, wayward);

    return_types
}

/// Information about a return statement's value.
enum ReturnValue {
    NoArg,
    MultipleArgs,
    SingleArg(ReturnType),
}

/// Visitor to find all explicit `return` statements in a method body.
struct ReturnFinder {
    returns: Vec<ReturnValue>,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        match node.arguments() {
            None => {
                self.returns.push(ReturnValue::NoArg);
            }
            Some(args) => {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    self.returns.push(ReturnValue::MultipleArgs);
                } else {
                    // Single argument — we need to classify it.
                    // We'll classify from the node directly.
                    let rt = classify_node(&arg_list[0], &[]);
                    self.returns.push(ReturnValue::SingleArg(rt));
                }
            }
        }
        // Don't recurse into the return node's arguments; we already handled them
    }

    // Don't recurse into nested defs/classes/modules
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

/// Collect the implicit return type(s) from a node.
/// Recurses into conditionals, and/or, begin blocks, etc.
fn collect_implicit_return(
    node: &ruby_prism::Node<'_>,
    returns: &mut Vec<ReturnType>,
    wayward: &[String],
) {
    // StatementsNode (method body) — take last statement
    if let Some(stmts) = node.as_statements_node() {
        let body: Vec<_> = stmts.body().iter().collect();
        if let Some(last) = body.last() {
            collect_implicit_return(last, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral); // empty body => nil
        }
        return;
    }

    // BeginNode — take last statement from its statements
    if let Some(begin) = node.as_begin_node() {
        if let Some(stmts) = begin.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                collect_implicit_return(last, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // IfNode — recurse into branches
    if let Some(if_node) = node.as_if_node() {
        // Then branch
        if let Some(stmts) = if_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                collect_implicit_return(last, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral); // empty then => nil
            }
        } else {
            returns.push(ReturnType::NonBooleanLiteral); // no then => nil
        }

        // Else/elsif branch
        if let Some(subsequent) = if_node.subsequent() {
            if let Some(elsif) = subsequent.as_if_node() {
                // elsif — recurse
                collect_implicit_return(&elsif.as_node(), returns, wayward);
            } else if let Some(else_node) = subsequent.as_else_node() {
                // else branch
                if let Some(stmts) = else_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if let Some(last) = body.last() {
                        collect_implicit_return(last, returns, wayward);
                    } else {
                        returns.push(ReturnType::NonBooleanLiteral);
                    }
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            // No else branch => implicit nil
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // UnlessNode — same structure as IfNode
    if let Some(unless_node) = node.as_unless_node() {
        // Body (the "then" part of unless)
        if let Some(stmts) = unless_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                collect_implicit_return(last, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }

        // Else branch
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if let Some(last) = body.last() {
                    collect_implicit_return(last, returns, wayward);
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            // No else => implicit nil
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // CaseNode — recurse into when branches and else
    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if let Some(last) = body.last() {
                        collect_implicit_return(last, returns, wayward);
                    } else {
                        returns.push(ReturnType::NonBooleanLiteral);
                    }
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
            }
        }
        // Else clause
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if let Some(last) = body.last() {
                    collect_implicit_return(last, returns, wayward);
                } else {
                    returns.push(ReturnType::NonBooleanLiteral);
                }
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            // No else in case => implicit nil
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // AndNode / OrNode — recurse into both sides
    if let Some(and_node) = node.as_and_node() {
        collect_implicit_return(&and_node.left(), returns, wayward);
        collect_implicit_return(&and_node.right(), returns, wayward);
        return;
    }
    if let Some(or_node) = node.as_or_node() {
        collect_implicit_return(&or_node.left(), returns, wayward);
        collect_implicit_return(&or_node.right(), returns, wayward);
        return;
    }

    // WhileNode / UntilNode — recurse into body's last value
    if let Some(while_node) = node.as_while_node() {
        if let Some(stmts) = while_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                collect_implicit_return(last, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        if let Some(stmts) = until_node.statements() {
            let body: Vec<_> = stmts.body().iter().collect();
            if let Some(last) = body.last() {
                collect_implicit_return(last, returns, wayward);
            } else {
                returns.push(ReturnType::NonBooleanLiteral);
            }
        } else {
            returns.push(ReturnType::NonBooleanLiteral);
        }
        return;
    }

    // ReturnNode — extract its value
    if let Some(ret_node) = node.as_return_node() {
        match ret_node.arguments() {
            None => returns.push(ReturnType::NonBooleanLiteral), // return without value => nil
            Some(args) => {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    returns.push(ReturnType::NonBooleanLiteral); // multiple args => array
                } else {
                    // Recurse into the single argument for classification
                    collect_implicit_return(&arg_list[0], returns, wayward);
                }
            }
        }
        return;
    }

    // ParenthesesNode — unwrap
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            collect_implicit_return(&body, returns, wayward);
        } else {
            returns.push(ReturnType::NonBooleanLiteral); // empty parens => nil
        }
        return;
    }

    // Leaf node: classify directly
    returns.push(classify_node(node, wayward));
}

/// Classify a single node as a ReturnType.
fn classify_node(node: &ruby_prism::Node<'_>, wayward: &[String]) -> ReturnType {
    // true/false literals => Boolean
    if node.as_true_node().is_some() || node.as_false_node().is_some() {
        return ReturnType::Boolean;
    }

    // nil => NonBooleanLiteral (nil is a literal but not boolean)
    if node.as_nil_node().is_some() {
        return ReturnType::NonBooleanLiteral;
    }

    // Other literals => NonBooleanLiteral
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
        || node.as_self_node().is_some()
        || node.as_lambda_node().is_some()
    {
        return ReturnType::NonBooleanLiteral;
    }

    // super / forwarding_super => Super
    if node.as_super_node().is_some() || node.as_forwarding_super_node().is_some() {
        return ReturnType::Super;
    }

    // CallNode => check method name
    if let Some(call) = node.as_call_node() {
        let method_name = call.name().as_slice();

        // Negation: `!x` is CallNode with method name `!` and a receiver
        if method_name == b"!" && call.receiver().is_some() && call.arguments().is_none() {
            return ReturnType::Boolean;
        }

        // Comparison methods => Boolean
        if COMPARISON_METHODS.iter().any(|m| *m == method_name) {
            return ReturnType::Boolean;
        }

        // Predicate method calls (ending in ?) that are not wayward => Boolean
        if method_name.ends_with(b"?") {
            let method_str = std::str::from_utf8(method_name).unwrap_or("");
            if !wayward.iter().any(|w| w == method_str) {
                return ReturnType::Boolean;
            }
            // Wayward predicate — treat as unknown
            return ReturnType::Unknown;
        }

        // Any other method call => Unknown
        return ReturnType::Unknown;
    }

    // Everything else (variables, constants, etc.) => Unknown
    ReturnType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(PredicateMethod, "cops/naming/predicate_method");
}
