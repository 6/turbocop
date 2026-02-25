use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MethodCallWithArgsParentheses;

fn is_operator(name: &[u8]) -> bool {
    matches!(
        name,
        b"+" | b"-"
            | b"*"
            | b"/"
            | b"%"
            | b"**"
            | b"=="
            | b"!="
            | b"<"
            | b">"
            | b"<="
            | b">="
            | b"<=>"
            | b"<<"
            | b">>"
            | b"&"
            | b"|"
            | b"^"
            | b"~"
            | b"!"
            | b"[]"
            | b"[]="
            | b"=~"
            | b"!~"
            | b"+@"
            | b"-@"
    )
}

/// Check if name is a setter method (ends with `=`)
fn is_setter(name: &[u8]) -> bool {
    name.last() == Some(&b'=') && name.len() > 1 && name != b"==" && name != b"!="
}

/// Check if a method name matches any pattern in the list (regex-style).
fn matches_any_pattern(name_str: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(name_str) {
                return true;
            }
        }
    }
    false
}

/// Check if the method name starts with an uppercase letter (CamelCase).
fn is_camel_case_method(name: &[u8]) -> bool {
    name.first().is_some_and(|b| b.is_ascii_uppercase())
}

/// Context for tracking whether we're in macro scope.
#[derive(Clone, Copy, PartialEq)]
enum Scope {
    /// Top-level (root) scope — macros are allowed
    Root,
    /// Inside class/module/sclass body — macros are allowed
    ClassLike,
    /// Inside a wrapper (begin, block, if branch) that is itself in macro scope
    WrapperInMacro,
    /// Inside a method definition — NOT macro scope
    MethodDef,
    /// Other non-macro context (e.g., wrapper inside a method)
    Other,
}

impl Scope {
    fn is_macro_scope(self) -> bool {
        matches!(self, Scope::Root | Scope::ClassLike | Scope::WrapperInMacro)
    }
}

/// Parent node type for omit_parentheses context checks.
#[derive(Clone, Copy, PartialEq)]
enum ParentKind {
    Array,
    Pair,
    Range,
    Splat,
    KwSplat,
    BlockPass,
    Ternary,
    LogicalOp,
    Call,
    OptArg,
    KwOptArg,
    ClassSingleLine,
    When,
    MatchPattern,
    Assignment,
    Conditional,
    ConstantPath,
}

impl Cop for MethodCallWithArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodCallWithArgsParentheses"
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
        let enforced_style = config.get_str("EnforcedStyle", "require_parentheses");
        let ignore_macros = config.get_bool("IgnoreMacros", true);
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let included_macros = config.get_string_array("IncludedMacros");
        let included_macro_patterns = config.get_string_array("IncludedMacroPatterns");
        let allow_multiline = config.get_bool("AllowParenthesesInMultilineCall", false);
        let allow_chaining = config.get_bool("AllowParenthesesInChaining", false);
        let allow_camel = config.get_bool("AllowParenthesesInCamelCaseMethod", false);
        let allow_interp = config.get_bool("AllowParenthesesInStringInterpolation", false);

        let mut visitor = ParenVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            enforced_style,
            ignore_macros,
            allowed_methods: allowed_methods.as_deref(),
            allowed_patterns: allowed_patterns.as_deref(),
            included_macros: included_macros.as_deref(),
            included_macro_patterns: included_macro_patterns.as_deref(),
            allow_multiline,
            allow_chaining,
            allow_camel,
            allow_interp,
            scope_stack: vec![Scope::Root],
            parent_stack: vec![],
            in_interpolation: false,
            in_endless_def: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ParenVisitor<'a> {
    cop: &'a MethodCallWithArgsParentheses,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    enforced_style: &'a str,
    ignore_macros: bool,
    allowed_methods: Option<&'a [String]>,
    allowed_patterns: Option<&'a [String]>,
    included_macros: Option<&'a [String]>,
    included_macro_patterns: Option<&'a [String]>,
    allow_multiline: bool,
    allow_chaining: bool,
    allow_camel: bool,
    allow_interp: bool,
    scope_stack: Vec<Scope>,
    parent_stack: Vec<ParentKind>,
    in_interpolation: bool,
    in_endless_def: bool,
}

impl ParenVisitor<'_> {
    fn current_scope(&self) -> Scope {
        *self.scope_stack.last().unwrap_or(&Scope::Other)
    }

    fn immediate_parent(&self) -> Option<ParentKind> {
        self.parent_stack.last().copied()
    }

    fn is_macro_scope(&self) -> bool {
        self.current_scope().is_macro_scope()
    }

    /// Derive child scope for wrapper nodes (begin, block, if branches)
    fn wrapper_child_scope(&self) -> Scope {
        if self.current_scope().is_macro_scope() {
            Scope::WrapperInMacro
        } else {
            Scope::Other
        }
    }

    fn check_require_parentheses(&mut self, call: &ruby_prism::CallNode<'_>) {
        let name = call.name().as_slice();

        // Skip operators and setters
        if is_operator(name) || is_setter(name) {
            return;
        }

        let has_parens = call.opening_loc().is_some();
        if has_parens {
            return;
        }

        // Must have arguments
        if call.arguments().is_none() {
            return;
        }

        let name_str = std::str::from_utf8(name).unwrap_or("");
        let is_receiverless = call.receiver().is_none();

        // AllowedMethods: exempt specific method names
        if let Some(methods) = self.allowed_methods {
            if methods.iter().any(|m| m == name_str) {
                return;
            }
        }

        // AllowedPatterns: exempt methods matching patterns
        if let Some(patterns) = self.allowed_patterns {
            if matches_any_pattern(name_str, patterns) {
                return;
            }
        }

        // IgnoreMacros: skip macro calls (receiverless + in macro scope)
        // unless they are in IncludedMacros or IncludedMacroPatterns
        if is_receiverless && self.ignore_macros && self.is_macro_scope() {
            let in_included = self
                .included_macros
                .is_some_and(|macros| macros.iter().any(|m| m == name_str));
            let in_included_patterns = self
                .included_macro_patterns
                .is_some_and(|patterns| matches_any_pattern(name_str, patterns));

            if !in_included && !in_included_patterns {
                return;
            }
        }

        // RuboCop reports the offense at the start of the full expression (including
        // receiver), not at the method name. Use call.location() to match.
        let (line, column) = self
            .source
            .offset_to_line_col(call.location().start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use parentheses for method calls with arguments.".to_string(),
        ));
    }

    fn check_omit_parentheses(&mut self, call: &ruby_prism::CallNode<'_>) {
        let name = call.name().as_slice();

        let has_parens = call.opening_loc().is_some();
        if !has_parens {
            return;
        }

        // syntax_like_method_call? — implicit call (.()) or operator methods
        if is_operator(name) {
            return;
        }

        // Check for implicit call: foo.() has call_operator_loc but no message_loc
        if call.message_loc().is_none() && call.call_operator_loc().is_some() {
            return;
        }

        // inside_endless_method_def? — parens required in endless methods
        if self.in_endless_def && call.arguments().is_some() {
            return;
        }

        // method_call_before_constant_resolution? — parent is ConstantPathNode
        if self.immediate_parent() == Some(ParentKind::ConstantPath) {
            return;
        }

        // super_call_without_arguments? — not applicable for CallNode

        // allowed_camel_case_method_call?
        if is_camel_case_method(name) && (call.arguments().is_none() || self.allow_camel) {
            return;
        }

        // AllowParenthesesInStringInterpolation
        if self.allow_interp && self.in_interpolation {
            return;
        }

        // legitimate_call_with_parentheses? — many sub-checks
        if self.legitimate_call_with_parentheses(call) {
            return;
        }

        // require_parentheses_for_hash_value_omission?
        if self.require_parentheses_for_hash_value_omission(call) {
            return;
        }

        let open_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = self.source.offset_to_line_col(open_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Omit parentheses for method calls with arguments.".to_string(),
        ));
    }

    /// Check require_parentheses_for_hash_value_omission?
    fn require_parentheses_for_hash_value_omission(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let args = match call.arguments() {
            Some(a) => a,
            None => return false,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        let last_arg = match arg_list.last() {
            Some(a) => a,
            None => return false,
        };

        // Check if last arg is a hash with value omission
        let has_value_omission = if let Some(hash) = last_arg.as_hash_node() {
            has_hash_value_omission(&hash)
        } else if let Some(kw_hash) = last_arg.as_keyword_hash_node() {
            has_keyword_hash_value_omission(&kw_hash)
        } else {
            return false;
        };

        if !has_value_omission {
            return false;
        }

        // parent&.conditional? || parent&.single_line? || !last_expression?
        let parent = self.immediate_parent();
        if parent == Some(ParentKind::Conditional) || parent == Some(ParentKind::When) {
            return true;
        }

        true // Conservative: keep parens when hash value omission is present
    }

    fn legitimate_call_with_parentheses(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        self.call_in_literals()
            || self.immediate_parent() == Some(ParentKind::When)
            || self.call_with_ambiguous_arguments(call)
            || self.call_in_logical_operators()
            || self.call_in_optional_arguments()
            || self.call_in_single_line_inheritance()
            || self.allowed_multiline_call_with_parentheses(call)
            || self.allowed_chained_call_with_parentheses(call)
            || self.assignment_in_condition()
            || self.forwards_anonymous_rest_arguments(call)
    }

    fn call_in_literals(&self) -> bool {
        // Check if the immediate parent is array, pair, range, splat, ternary
        if let Some(p) = self.parent_stack.last() {
            matches!(
                p,
                ParentKind::Array
                    | ParentKind::Pair
                    | ParentKind::Range
                    | ParentKind::Splat
                    | ParentKind::KwSplat
                    | ParentKind::BlockPass
                    | ParentKind::Ternary
            )
        } else {
            false
        }
    }

    fn call_in_logical_operators(&self) -> bool {
        self.immediate_parent() == Some(ParentKind::LogicalOp)
    }

    fn call_in_optional_arguments(&self) -> bool {
        self.immediate_parent() == Some(ParentKind::OptArg)
            || self.immediate_parent() == Some(ParentKind::KwOptArg)
    }

    fn call_in_single_line_inheritance(&self) -> bool {
        self.immediate_parent() == Some(ParentKind::ClassSingleLine)
    }

    fn allowed_multiline_call_with_parentheses(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if !self.allow_multiline {
            return false;
        }
        let call_loc = call.location();
        let (start_line, _) = self.source.offset_to_line_col(call_loc.start_offset());
        let (end_line, _) = self.source.offset_to_line_col(call_loc.end_offset());
        start_line != end_line
    }

    fn allowed_chained_call_with_parentheses(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if !self.allow_chaining {
            return false;
        }
        has_parenthesized_ancestor_call(call)
    }

    fn call_with_ambiguous_arguments(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        self.call_with_braced_block(call)
            || self.call_in_argument_with_block(call)
            || self.call_as_argument_or_chain()
            || self.call_in_match_pattern()
            || self.hash_literal_in_arguments(call)
            || self.ambiguous_range_argument(call)
            || self.has_ambiguous_content_in_descendants(call)
            || self.call_has_block_pass(call)
    }

    fn call_with_braced_block(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(block) = call.block() {
            if let Some(block_node) = block.as_block_node() {
                let open = block_node.opening_loc();
                let src = self.source.as_bytes();
                if open.start_offset() < src.len() && src[open.start_offset()] == b'{' {
                    return true;
                }
            }
        }
        false
    }

    fn call_in_argument_with_block(&self, _call: &ruby_prism::CallNode<'_>) -> bool {
        // Check if call is inside a block whose parent is a call/super/yield
        // We approximate this by checking parent stack: block inside call
        // This is already handled by the block visitor pushing scope, but
        // the parent_stack check for Call covers this case too
        false // covered by call_as_argument_or_chain
    }

    fn call_as_argument_or_chain(&self) -> bool {
        matches!(self.immediate_parent(), Some(ParentKind::Call))
    }

    fn call_has_block_pass(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        // Check if the call has a block argument (&block)
        call.block()
            .is_some_and(|b| b.as_block_argument_node().is_some())
    }

    fn call_in_match_pattern(&self) -> bool {
        self.immediate_parent() == Some(ParentKind::MatchPattern)
    }

    fn hash_literal_in_arguments(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if has_hash_literal(&arg) {
                    return true;
                }
            }
        }
        false
    }

    fn ambiguous_range_argument(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let args = match call.arguments() {
            Some(a) => a,
            None => return false,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();

        // First arg is a beginless range
        if let Some(first) = arg_list.first() {
            if let Some(range) = first.as_range_node() {
                if range.left().is_none() {
                    return true;
                }
            }
        }

        // Last arg is an endless range
        if let Some(last) = arg_list.last() {
            if let Some(range) = last.as_range_node() {
                if range.right().is_none() {
                    return true;
                }
            }
        }

        false
    }

    /// Check for forwarded args, ambiguous literals, logical operators, and blocks in descendants
    fn has_ambiguous_content_in_descendants(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if is_ambiguous_descendant(&arg, self.source) {
                    return true;
                }
            }
        }
        false
    }

    fn forwards_anonymous_rest_arguments(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(last) = arg_list.last() {
                // forwarded_restarg_type? — anonymous *
                if last
                    .as_splat_node()
                    .is_some_and(|s| s.expression().is_none())
                {
                    return true;
                }
                // Check for forwarded_kwrestarg in hash
                if let Some(kw_hash) = last.as_keyword_hash_node() {
                    for elem in kw_hash.elements().iter() {
                        if elem
                            .as_assoc_splat_node()
                            .is_some_and(|s| s.value().is_none())
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn assignment_in_condition(&self) -> bool {
        if self.parent_stack.len() >= 2 {
            let parent = self.parent_stack[self.parent_stack.len() - 1];
            let grandparent = self.parent_stack[self.parent_stack.len() - 2];
            if parent == ParentKind::Assignment
                && (grandparent == ParentKind::Conditional || grandparent == ParentKind::When)
            {
                return true;
            }
        }
        false
    }

    fn visit_call_common(&mut self, call: &ruby_prism::CallNode<'_>) {
        match self.enforced_style {
            "omit_parentheses" => self.check_omit_parentheses(call),
            _ => self.check_require_parentheses(call),
        }
    }
}

/// Check if a hash node has value omission (Ruby 3.1 shorthand `{foo:}`)
fn has_hash_value_omission(hash: &ruby_prism::HashNode<'_>) -> bool {
    for elem in hash.elements().iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if assoc.value().as_implicit_node().is_some() {
                return true;
            }
        }
    }
    false
}

fn has_keyword_hash_value_omission(kw_hash: &ruby_prism::KeywordHashNode<'_>) -> bool {
    for elem in kw_hash.elements().iter() {
        if let Some(assoc) = elem.as_assoc_node() {
            if assoc.value().as_implicit_node().is_some() {
                return true;
            }
        }
    }
    false
}

/// Check if a node contains a hash literal with braces (not keyword hash)
fn has_hash_literal(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(hash) = node.as_hash_node() {
        if hash.opening_loc().as_slice() == b"{" {
            return true;
        }
    }
    // Recurse into call descendants
    if let Some(call) = node.as_call_node() {
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if has_hash_literal(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a CallNode has parenthesized ancestor calls in the chain
fn has_parenthesized_ancestor_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let mut current = call.receiver();
    while let Some(recv) = current {
        if let Some(recv_call) = recv.as_call_node() {
            if recv_call.opening_loc().is_some() {
                return true;
            }
            current = recv_call.receiver();
        } else {
            break;
        }
    }
    false
}

/// Recursively check if a node or its descendants are ambiguous in omit_parentheses style.
/// This covers: splats, ternary, regex, unary, forwarded args, logical operators, blocks.
fn is_ambiguous_descendant(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    // Direct checks on this node
    if node.as_splat_node().is_some()
        || node.as_assoc_splat_node().is_some()
        || node.as_block_argument_node().is_some()
    {
        return true;
    }

    // Ternary if — has then_keyword (the `?`) but no end_keyword
    if let Some(if_node) = node.as_if_node() {
        if if_node.then_keyword_loc().is_some() && if_node.end_keyword_loc().is_none() {
            return true;
        }
    }

    // Regex slash literal
    if let Some(regex) = node.as_regular_expression_node() {
        let bytes = source.as_bytes();
        let open = regex.opening_loc();
        if open.start_offset() < bytes.len() && bytes[open.start_offset()] == b'/' {
            return true;
        }
    }
    if let Some(regex) = node.as_interpolated_regular_expression_node() {
        let bytes = source.as_bytes();
        let open_offset = regex.opening_loc().start_offset();
        if open_offset < bytes.len() && bytes[open_offset] == b'/' {
            return true;
        }
    }

    // Unary literal: negative/positive numbers
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
    {
        let bytes = source.as_bytes();
        let start = node.location().start_offset();
        if start < bytes.len() && (bytes[start] == b'-' || bytes[start] == b'+') {
            return true;
        }
    }

    // Unary operation on non-numeric (e.g., `+""`, `-""`)
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"+@" || name == b"-@")
            && call.receiver().is_some()
            && call.arguments().is_none()
        {
            return true;
        }
    }

    // Forwarded args
    if node.as_forwarding_arguments_node().is_some() {
        return true;
    }

    // Logical operators
    if node.as_and_node().is_some() || node.as_or_node().is_some() {
        return true;
    }

    // Block node
    if node.as_block_node().is_some() {
        return true;
    }

    // Recurse into children of certain compound node types
    if let Some(call) = node.as_call_node() {
        if call.block().is_some() {
            return true;
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if is_ambiguous_descendant(&arg, source) {
                    return true;
                }
            }
        }
        if let Some(recv) = call.receiver() {
            if is_ambiguous_descendant(&recv, source) {
                return true;
            }
        }
    }
    // Recurse into array elements
    if let Some(array) = node.as_array_node() {
        for elem in array.elements().iter() {
            if is_ambiguous_descendant(&elem, source) {
                return true;
            }
        }
    }
    // Recurse into hash pairs
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if is_ambiguous_descendant(&elem, source) {
                return true;
            }
        }
    }
    if let Some(kw_hash) = node.as_keyword_hash_node() {
        for elem in kw_hash.elements().iter() {
            if is_ambiguous_descendant(&elem, source) {
                return true;
            }
        }
    }
    if let Some(assoc) = node.as_assoc_node() {
        if is_ambiguous_descendant(&assoc.value(), source) {
            return true;
        }
    }

    false
}

impl<'pr> Visit<'pr> for ParenVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.visit_call_common(node);

        // Visit children — push Call as parent for receiver, args, and block arg
        // because in RuboCop, all these children have the call as parent node
        if let Some(recv) = node.receiver() {
            self.parent_stack.push(ParentKind::Call);
            self.visit(&recv);
            self.parent_stack.pop();
        }
        if let Some(args) = node.arguments() {
            self.parent_stack.push(ParentKind::Call);
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
            self.parent_stack.pop();
        }
        if let Some(block) = node.block() {
            self.parent_stack.push(ParentKind::Call);
            self.visit(&block);
            self.parent_stack.pop();
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Check if single-line
        let (start_line, _) = self
            .source
            .offset_to_line_col(node.location().start_offset());
        let (end_line, _) = self.source.offset_to_line_col(node.location().end_offset());
        let is_single_line = start_line == end_line;

        if let Some(superclass) = node.superclass() {
            if is_single_line {
                self.parent_stack.push(ParentKind::ClassSingleLine);
            }
            self.visit(&superclass);
            if is_single_line {
                self.parent_stack.pop();
            }
        }

        self.scope_stack.push(Scope::ClassLike);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.scope_stack.push(Scope::ClassLike);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.scope_stack.push(Scope::ClassLike);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let is_endless = node.end_keyword_loc().is_none() && node.equal_loc().is_some();
        let prev_endless = self.in_endless_def;
        if is_endless {
            self.in_endless_def = true;
        }

        self.scope_stack.push(Scope::MethodDef);
        // Visit parameters
        if let Some(params) = node.parameters() {
            self.visit_parameters_node(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
        self.in_endless_def = prev_endless;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let child_scope = self.wrapper_child_scope();
        self.scope_stack.push(child_scope);
        if let Some(params) = node.parameters() {
            self.visit(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.scope_stack.push(Scope::Other);
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.scope_stack.pop();
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let child_scope = self.wrapper_child_scope();
        self.scope_stack.push(child_scope);
        // Delegate to default visitor for all children
        ruby_prism::visit_begin_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Check if this is a ternary: has then_keyword (the `?`) but no end_keyword
        let is_ternary = node.then_keyword_loc().is_some() && node.end_keyword_loc().is_none();

        // Visit condition — if ternary, push Ternary parent
        if is_ternary {
            self.parent_stack.push(ParentKind::Ternary);
        }
        self.visit(&node.predicate());
        if is_ternary {
            self.parent_stack.pop();
        }

        // Visit then/else branches as wrapper in macro scope
        let child_scope = self.wrapper_child_scope();

        if let Some(stmts) = node.statements() {
            self.scope_stack.push(child_scope);
            if is_ternary {
                self.parent_stack.push(ParentKind::Ternary);
            }
            self.visit_statements_node(&stmts);
            if is_ternary {
                self.parent_stack.pop();
            }
            self.scope_stack.pop();
        }
        if let Some(subsequent) = node.subsequent() {
            self.scope_stack.push(child_scope);
            if is_ternary {
                self.parent_stack.push(ParentKind::Ternary);
            }
            self.visit(&subsequent);
            if is_ternary {
                self.parent_stack.pop();
            }
            self.scope_stack.pop();
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.visit(&node.predicate());

        let child_scope = self.wrapper_child_scope();

        if let Some(stmts) = node.statements() {
            self.scope_stack.push(child_scope);
            self.visit_statements_node(&stmts);
            self.scope_stack.pop();
        }
        if let Some(consequent) = node.else_clause() {
            self.scope_stack.push(child_scope);
            self.visit_else_node(&consequent);
            self.scope_stack.pop();
        }
    }

    // Track parent context for omit_parentheses checks
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        self.parent_stack.push(ParentKind::Array);
        for elem in node.elements().iter() {
            self.visit(&elem);
        }
        self.parent_stack.pop();
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        self.parent_stack.push(ParentKind::Pair);
        self.visit(&node.key());
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_range_node(&mut self, node: &ruby_prism::RangeNode<'pr>) {
        self.parent_stack.push(ParentKind::Range);
        if let Some(left) = node.left() {
            self.visit(&left);
        }
        if let Some(right) = node.right() {
            self.visit(&right);
        }
        self.parent_stack.pop();
    }

    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        self.parent_stack.push(ParentKind::Splat);
        if let Some(expr) = node.expression() {
            self.visit(&expr);
        }
        self.parent_stack.pop();
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        self.parent_stack.push(ParentKind::KwSplat);
        if let Some(value) = node.value() {
            self.visit(&value);
        }
        self.parent_stack.pop();
    }

    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        self.parent_stack.push(ParentKind::BlockPass);
        if let Some(expr) = node.expression() {
            self.visit(&expr);
        }
        self.parent_stack.pop();
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        self.parent_stack.push(ParentKind::LogicalOp);
        self.visit(&node.left());
        self.visit(&node.right());
        self.parent_stack.pop();
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        self.parent_stack.push(ParentKind::LogicalOp);
        self.visit(&node.left());
        self.visit(&node.right());
        self.parent_stack.pop();
    }

    fn visit_optional_parameter_node(&mut self, node: &ruby_prism::OptionalParameterNode<'pr>) {
        self.parent_stack.push(ParentKind::OptArg);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_optional_keyword_parameter_node(
        &mut self,
        node: &ruby_prism::OptionalKeywordParameterNode<'pr>,
    ) {
        self.parent_stack.push(ParentKind::KwOptArg);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_match_required_node(&mut self, node: &ruby_prism::MatchRequiredNode<'pr>) {
        self.parent_stack.push(ParentKind::MatchPattern);
        self.visit(&node.value());
        self.parent_stack.pop();
        self.visit(&node.pattern());
    }

    fn visit_match_predicate_node(&mut self, node: &ruby_prism::MatchPredicateNode<'pr>) {
        self.parent_stack.push(ParentKind::MatchPattern);
        self.visit(&node.value());
        self.parent_stack.pop();
        self.visit(&node.pattern());
    }

    fn visit_when_node(&mut self, node: &ruby_prism::WhenNode<'pr>) {
        self.parent_stack.push(ParentKind::When);
        for cond in node.conditions().iter() {
            self.visit(&cond);
        }
        self.parent_stack.pop();

        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        // The child (left side of ::) gets ConstantPath as parent context
        if let Some(parent_node) = node.parent() {
            self.parent_stack.push(ParentKind::ConstantPath);
            self.visit(&parent_node);
            self.parent_stack.pop();
        }
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        let prev = self.in_interpolation;
        self.in_interpolation = true;
        for part in node.parts().iter() {
            self.visit(&part);
        }
        self.in_interpolation = prev;
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        let prev = self.in_interpolation;
        self.in_interpolation = true;
        for part in node.parts().iter() {
            self.visit(&part);
        }
        self.in_interpolation = prev;
    }

    // Track assignment context
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.parent_stack.push(ParentKind::Assignment);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.parent_stack.push(ParentKind::Assignment);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.parent_stack.push(ParentKind::Assignment);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.parent_stack.push(ParentKind::Assignment);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.parent_stack.push(ParentKind::Assignment);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        self.visit_constant_path_node(&node.target());
        self.parent_stack.push(ParentKind::Assignment);
        self.visit(&node.value());
        self.parent_stack.pop();
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.parent_stack.push(ParentKind::Conditional);
        self.visit(&node.predicate());
        self.parent_stack.pop();
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.parent_stack.push(ParentKind::Conditional);
        self.visit(&node.predicate());
        self.parent_stack.pop();
        if let Some(stmts) = node.statements() {
            self.visit_statements_node(&stmts);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(
        MethodCallWithArgsParentheses,
        "cops/style/method_call_with_args_parentheses"
    );

    #[test]
    fn operators_are_ignored() {
        let source = b"x = 1 + 2\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn method_without_args_is_ok() {
        let source = b"foo.bar\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn receiverless_in_class_body_is_macro() {
        let source = b"class Foo\n  bar :baz\nend\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty(), "Macro in class body should be ignored");
    }

    #[test]
    fn receiverless_in_method_body_is_not_macro() {
        let source = b"def foo\n  bar 1, 2\nend\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert_eq!(
            diags.len(),
            1,
            "Receiverless call inside method should be flagged"
        );
    }

    #[test]
    fn receiverless_in_module_body_is_macro() {
        let source = b"module Foo\n  bar :baz\nend\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(diags.is_empty(), "Macro in module body should be ignored");
    }

    #[test]
    fn receiverless_at_top_level_is_macro() {
        let source = b"puts 'hello'\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(
            diags.is_empty(),
            "Receiverless call at top level should be treated as macro"
        );
    }

    #[test]
    fn macro_in_block_inside_class() {
        let source = b"class Foo\n  concern do\n    bar :baz\n  end\nend\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(
            diags.is_empty(),
            "Macro in block inside class should be ignored"
        );
    }

    #[test]
    fn omit_parentheses_flags_parens() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.bar(1)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(diags.len(), 1, "Should flag parens with omit_parentheses");
        assert!(diags[0].message.contains("Omit parentheses"));
    }

    #[test]
    fn omit_parentheses_allows_no_parens() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.bar 1\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag calls without parens in omit_parentheses"
        );
    }

    #[test]
    fn omit_accepts_parens_in_array() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"[foo.bar(1)]\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens inside array literal");
    }

    #[test]
    fn omit_accepts_parens_in_logical_ops() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(a) && bar(b)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in logical operator context"
        );
    }

    #[test]
    fn omit_accepts_parens_in_chained_calls() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo().bar(3).wait(4).it\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in chained calls (not last)"
        );
    }

    #[test]
    fn omit_accepts_parens_in_default_arg() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def foo(arg = default(42))\n  nil\nend\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in default argument value"
        );
    }

    #[test]
    fn omit_accepts_parens_with_splat() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(*args)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens with splat args");
    }

    #[test]
    fn omit_accepts_parens_with_block_pass() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(&block)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens with block pass");
    }

    #[test]
    fn omit_accepts_parens_with_braced_block() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(1) { 2 }\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens with braced block");
    }

    #[test]
    fn omit_accepts_parens_with_hash_literal() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"top.test({foo: :bar})\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens with hash literal arg"
        );
    }

    #[test]
    fn omit_accepts_parens_with_unary_arg() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(-1)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens with unary minus arg");
    }

    #[test]
    fn omit_accepts_parens_with_regex() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(/regexp/)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens with regex arg");
    }

    #[test]
    fn omit_accepts_parens_with_range() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"1..limit(n)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens inside range literal");
    }

    #[test]
    fn omit_accepts_parens_in_ternary() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.include?(bar) ? bar : quux\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens in ternary condition");
    }

    #[test]
    fn omit_accepts_parens_in_when_clause() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"case condition\nwhen do_something(arg)\nend\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens in when clause");
    }

    #[test]
    fn omit_accepts_parens_in_endless_def() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def x() = foo(y)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in endless method def"
        );
    }

    #[test]
    fn omit_accepts_parens_before_constant_resolution() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"do_something(arg)::CONST\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens before constant resolution"
        );
    }

    #[test]
    fn omit_accepts_parens_as_method_arg() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"top.test 1, 2, foo: bar(3)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens for calls used as method args"
        );
    }

    #[test]
    fn omit_accepts_parens_in_match_pattern() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"execute(query) in {elapsed:, sql_count:}\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens in match pattern");
    }

    #[test]
    fn omit_accepts_operator_methods() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"data.[](value)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(diags.is_empty(), "Should allow parens on operator method");
    }

    #[test]
    fn omit_flags_last_in_chain() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo().bar(3).wait(4)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag only the last parenthesized call in chain"
        );
    }

    #[test]
    fn omit_flags_do_end_block() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo(:arg) do\n  bar\nend\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(diags.len(), 1, "Should flag parens in do-end block call");
    }

    #[test]
    fn omit_accepts_parens_in_single_line_inheritance() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"class Point < Struct.new(:x, :y); end\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in single-line inheritance"
        );
    }

    #[test]
    fn omit_accepts_forwarded_args() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("omit_parentheses".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"def delegated_call(...)\n  @proxy.call(...)\nend\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens for forwarded arguments"
        );
    }

    #[test]
    fn allowed_methods_exempts() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "AllowedMethods".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("custom_log".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.custom_log 'msg'\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag method in AllowedMethods list"
        );
    }

    #[test]
    fn allowed_patterns_exempts() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "AllowedPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^assert".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"foo.assert_equal 'x', y\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag method matching AllowedPatterns"
        );
    }

    #[test]
    fn ignore_macros_false_flags_receiverless() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("IgnoreMacros".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"custom_macro :arg\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag receiverless macro with IgnoreMacros:false"
        );
    }

    #[test]
    fn ignore_macros_skips_receiverless() {
        let source = b"custom_macro :arg\n";
        let diags = run_cop_full(&MethodCallWithArgsParentheses, source);
        assert!(
            diags.is_empty(),
            "Should skip receiverless macro with IgnoreMacros:true"
        );
    }

    #[test]
    fn included_macros_forces_check() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "IncludedMacros".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("custom_macro".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"custom_macro :arg\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag macro in IncludedMacros despite IgnoreMacros:true"
        );
    }

    #[test]
    fn included_macro_patterns_forces_check() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "IncludedMacroPatterns".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("^validate".into())]),
            )]),
            ..CopConfig::default()
        };
        let source = b"validates_presence :name\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag macro matching IncludedMacroPatterns"
        );
    }

    #[test]
    fn omit_allow_multiline_call() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInMultilineCall".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"foo.bar(\n  1\n)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens in multiline call with AllowParenthesesInMultilineCall"
        );
    }

    #[test]
    fn omit_allow_chaining() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInChaining".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"foo().bar(3).quux.wait(4)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens when chaining with previous parens"
        );
    }

    #[test]
    fn omit_allow_camel_case() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInCamelCaseMethod".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"Array(1)\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens on CamelCase method with AllowParenthesesInCamelCaseMethod"
        );
    }

    #[test]
    fn omit_allow_string_interpolation() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                (
                    "EnforcedStyle".into(),
                    serde_yml::Value::String("omit_parentheses".into()),
                ),
                (
                    "AllowParenthesesInStringInterpolation".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let source = b"x = \"#{foo.bar(1)}\"\n";
        let diags = run_cop_full_with_config(&MethodCallWithArgsParentheses, source, config);
        assert!(
            diags.is_empty(),
            "Should allow parens inside string interpolation"
        );
    }
}
