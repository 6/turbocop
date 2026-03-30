use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MethodCallWithoutArgsParentheses: Do not use parentheses for method
/// calls with no arguments.
///
/// Corpus investigation: 176 FP, 174 FN at 92.4% match rate.
///
/// Root cause of FPs: nitrocop did not implement `same_name_assignment?` or
/// `default_argument?` from RuboCop. When a no-receiver method call like
/// `test()` appears on the RHS of an assignment whose LHS has the same name
/// (`test = test()`, `test ||= test()`, `one, test = 1, test()`), or inside
/// a default parameter (`def foo(test = test())`), the parentheses are needed
/// for disambiguation from the local variable. Additionally, `AllowedPatterns`
/// was read from config but never applied as regex matching.
///
/// Root cause of FNs: nitrocop skipped ALL bare `it()` calls without a
/// receiver, and also skipped every parenthesized zero-arg call with an
/// attached real block literal (`at_exit() do`, `synchronize() do`). RuboCop
/// only skips `it()` inside a block whose parameters are empty and have no
/// delimiters (i.e., `{ it() }` or `do it() end`, but NOT `{ || it() }` or
/// `{ |_n| it() }`). `it()` in def bodies, blocks with explicit params, and
/// ordinary no-arg block calls should still be flagged, but block-pass calls
/// like `foo(&block)` still count as having an argument and must remain
/// allowed.
///
/// Fix: Converted from `check_node` to `check_source` with a visitor that
/// tracks assignment context (local variable writes, or-writes, op-writes,
/// multi-writes, optional parameters) and block context (whether inside a
/// block with no explicit params). Implemented `AllowedPatterns` regex
/// matching. Fixed `it()` exemption to only apply in parameterless blocks, and
/// stopped dropping parenthesized no-arg calls solely because they have a real
/// block literal, while still skipping block-pass forms that Prism stores in
/// `call.block()` as `BlockArgumentNode`.
pub struct MethodCallWithoutArgsParentheses;

impl Cop for MethodCallWithoutArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodCallWithoutArgsParentheses"
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
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_default();
        let allowed_patterns = config
            .get_string_array("AllowedPatterns")
            .unwrap_or_default();

        let mut visitor = MethodCallVisitor {
            cop: self,
            source,
            allowed_methods,
            allowed_patterns,
            diagnostics: Vec::new(),
            // Stack of assignment variable names (for same_name_assignment detection)
            assignment_names: Vec::new(),
            // Whether inside an OptionalParameterNode
            in_optarg: false,
            // Stack of block info: true = block with no explicit params (it() exempt)
            block_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MethodCallVisitor<'a, 'src> {
    cop: &'a MethodCallWithoutArgsParentheses,
    source: &'src SourceFile,
    allowed_methods: Vec<String>,
    allowed_patterns: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    /// Stack of sets of variable names from enclosing assignments.
    /// Each entry is a vec of names assigned at that level.
    assignment_names: Vec<Vec<String>>,
    /// Whether we're inside an optional parameter default value.
    in_optarg: bool,
    /// Stack tracking block context. true = block has no explicit params (it() is exempt).
    block_stack: Vec<bool>,
}

impl<'a, 'src> MethodCallVisitor<'a, 'src> {
    fn is_same_name_assignment(&self, method_name: &str) -> bool {
        for names in &self.assignment_names {
            if names.iter().any(|n| n == method_name) {
                return true;
            }
        }
        false
    }

    fn in_block_without_explicit_params(&self) -> bool {
        self.block_stack.iter().any(|&exempt| exempt)
    }

    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must have parentheses (opening_loc present)
        if call.opening_loc().is_none() {
            return;
        }

        // Must have no arguments
        if call.arguments().is_some() {
            return;
        }

        // Must have a message (method name)
        let msg_loc = match call.message_loc() {
            Some(l) => l,
            None => return,
        };
        let has_block_literal = call.block().and_then(|block| block.as_block_node()).is_some();

        // Prism stores block-pass (`foo(&block)`, `map(&:name)`) on `call.block()`
        // rather than in `arguments()`. RuboCop treats these as argument-bearing
        // calls, so only real block literals belong to this FN fix.
        if call.block().is_some() && !has_block_literal {
            return;
        }

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Skip methods starting with uppercase (like Test()) - conversion methods
        if method_bytes.first().is_some_and(|b| b.is_ascii_uppercase()) {
            return;
        }

        // Skip operator methods ([], []=)
        if method_bytes == b"[]" || method_bytes == b"[]=" {
            return;
        }

        // Skip `not()` — keyword
        if method_bytes == b"not" || msg_loc.as_slice() == b"not" {
            return;
        }

        // Skip lambda call syntax: thing.()
        if msg_loc.as_slice() == b"call" && call.call_operator_loc().is_some() {
            let src = self.source.as_bytes();
            let op_loc = call.call_operator_loc().unwrap();
            let after_op = op_loc.end_offset();
            if after_op < src.len() && src[after_op] == b'(' {
                return;
            }
        }

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("");

        // Check `it()` exemption — only exempt inside a block with no explicit params
        // AND with no receiver. `it() { ... }` is still an offense.
        if method_bytes == b"it"
            && call.receiver().is_none()
            && !has_block_literal
            && self.in_block_without_explicit_params()
        {
            return;
        }

        // Check default_argument? — inside an OptionalParameterNode
        if self.in_optarg {
            return;
        }

        // Check same_name_assignment? — no receiver, and an enclosing assignment
        // uses the same variable name
        if call.receiver().is_none() && self.is_same_name_assignment(method_str) {
            return;
        }

        // Check AllowedMethods
        if self.allowed_methods.iter().any(|m| m == method_str) {
            return;
        }

        // Check AllowedPatterns (regex)
        if self
            .allowed_patterns
            .iter()
            .any(|p| regex::Regex::new(p).is_ok_and(|re| re.is_match(method_str)))
        {
            return;
        }

        let open_loc = call.opening_loc().unwrap();
        let (line, column) = self.source.offset_to_line_col(open_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Do not use parentheses for method calls with no arguments.".to_string(),
        ));
    }

    /// Check if a block node has no explicit parameters (empty and without delimiters).
    /// Returns true if it's safe for it() exemption.
    fn block_has_no_explicit_params(block: &ruby_prism::BlockNode<'_>) -> bool {
        match block.parameters() {
            None => true, // No parameters at all: `{ ... }` or `do ... end`
            Some(params) => {
                // If it's a BlockParametersNode, check if it has no params AND no
                // opening delimiter (i.e., no `||`).
                if let Some(bp) = params.as_block_parameters_node() {
                    // If there's an opening_loc, it means `||` was written explicitly
                    if bp.opening_loc().is_some() {
                        return false;
                    }
                    // Check that there are actually no parameters defined
                    if let Some(inner) = bp.parameters() {
                        return inner.requireds().is_empty()
                            && inner.optionals().is_empty()
                            && inner.rest().is_none()
                            && inner.posts().is_empty()
                            && inner.keywords().is_empty()
                            && inner.keyword_rest().is_none()
                            && inner.block().is_none();
                    }
                    true
                } else if params.as_numbered_parameters_node().is_some() {
                    // Numbered parameters like _1 — not empty
                    false
                } else if params.as_it_parameters_node().is_some() {
                    // ItParametersNode — the block uses `it` as a parameter reference
                    // This means `it` IS the block param, so it() is exempt
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl<'a, 'src, 'pr> Visit<'pr> for MethodCallVisitor<'a, 'src> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }

    // Track local variable write: `test = test()`
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.assignment_names.push(vec![name]);
        ruby_prism::visit_local_variable_write_node(self, node);
        self.assignment_names.pop();
    }

    // Track local variable or-write: `test ||= test()`
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.assignment_names.push(vec![name]);
        ruby_prism::visit_local_variable_or_write_node(self, node);
        self.assignment_names.pop();
    }

    // Track local variable and-write: `test &&= test()`
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.assignment_names.push(vec![name]);
        ruby_prism::visit_local_variable_and_write_node(self, node);
        self.assignment_names.pop();
    }

    // Track local variable operator-write: `test += test()`
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.assignment_names.push(vec![name]);
        ruby_prism::visit_local_variable_operator_write_node(self, node);
        self.assignment_names.pop();
    }

    // Track multi-write (parallel assignment): `one, test = 1, test()`
    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        // Collect all local variable target names from the LHS
        let mut names = Vec::new();
        for target in node.lefts().iter() {
            if let Some(lv) = target.as_local_variable_target_node() {
                if let Ok(name) = std::str::from_utf8(lv.name().as_slice()) {
                    names.push(name.to_string());
                }
            }
            // Also check rest if it's a local variable target
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(lv) = expr.as_local_variable_target_node() {
                        if let Ok(name) = std::str::from_utf8(lv.name().as_slice()) {
                            names.push(name.to_string());
                        }
                    }
                }
            }
        }
        self.assignment_names.push(names);
        ruby_prism::visit_multi_write_node(self, node);
        self.assignment_names.pop();
    }

    // Track optional parameter: `def foo(test = test())`
    fn visit_optional_parameter_node(&mut self, node: &ruby_prism::OptionalParameterNode<'pr>) {
        let old = self.in_optarg;
        self.in_optarg = true;
        ruby_prism::visit_optional_parameter_node(self, node);
        self.in_optarg = old;
    }

    // Track block context for it() exemption
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let exempt = Self::block_has_no_explicit_params(node);
        self.block_stack.push(exempt);
        ruby_prism::visit_block_node(self, node);
        self.block_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MethodCallWithoutArgsParentheses,
        "cops/style/method_call_without_args_parentheses"
    );
}
