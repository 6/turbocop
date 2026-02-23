use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct SafeNavigation;

/// Methods that `nil` responds to in vanilla Ruby.
/// Converting `foo && foo.bar.is_a?(X)` to `foo&.bar&.is_a?(X)` changes behavior
/// because nil already responds to these methods.
const NIL_METHODS: &[&[u8]] = &[
    b"nil?",
    b"is_a?",
    b"kind_of?",
    b"instance_of?",
    b"respond_to?",
    b"eql?",
    b"equal?",
    b"frozen?",
    b"class",
    b"clone",
    b"dup",
    b"freeze",
    b"hash",
    b"inspect",
    b"to_s",
    b"to_a",
    b"to_f",
    b"to_i",
    b"to_r",
    b"to_c",
    b"object_id",
    b"send",
    b"__send__",
    b"__id__",
    b"public_send",
    b"tap",
    b"then",
    b"yield_self",
    b"itself",
    b"display",
    b"method",
    b"public_method",
    b"singleton_method",
    b"define_singleton_method",
    b"extend",
    b"pp",
    b"respond_to_missing?",
    b"instance_variable_get",
    b"instance_variable_set",
    b"instance_variable_defined?",
    b"instance_variables",
    b"remove_instance_variable",
];

impl SafeNavigation {
    /// Check if two nodes represent the same identifier, including:
    /// - local variable reads
    /// - instance/class/global variable reads
    /// - bare method calls (no receiver, no args) which look like variable reads
    fn same_identifier(a: &ruby_prism::Node<'_>, b: &ruby_prism::Node<'_>) -> bool {
        if let (Some(la), Some(lb)) = (
            a.as_local_variable_read_node(),
            b.as_local_variable_read_node(),
        ) {
            return la.name().as_slice() == lb.name().as_slice();
        }
        if let (Some(ia), Some(ib)) = (
            a.as_instance_variable_read_node(),
            b.as_instance_variable_read_node(),
        ) {
            return ia.name().as_slice() == ib.name().as_slice();
        }
        if let (Some(ca), Some(cb)) = (
            a.as_class_variable_read_node(),
            b.as_class_variable_read_node(),
        ) {
            return ca.name().as_slice() == cb.name().as_slice();
        }
        if let (Some(ga), Some(gb)) = (
            a.as_global_variable_read_node(),
            b.as_global_variable_read_node(),
        ) {
            return ga.name().as_slice() == gb.name().as_slice();
        }
        // Both are bare method calls (no receiver, no args) with the same name
        if let (Some(ca), Some(cb)) = (a.as_call_node(), b.as_call_node()) {
            if ca.receiver().is_none()
                && cb.receiver().is_none()
                && ca.arguments().is_none()
                && cb.arguments().is_none()
                && ca.block().is_none()
                && cb.block().is_none()
            {
                return ca.name().as_slice() == cb.name().as_slice();
            }
        }
        // One is a local variable read and the other is a bare method call with same name
        if let Some(lv) = a.as_local_variable_read_node() {
            if let Some(call) = b.as_call_node() {
                if call.receiver().is_none() && call.arguments().is_none() && call.block().is_none()
                {
                    return lv.name().as_slice() == call.name().as_slice();
                }
            }
        }
        if let Some(lv) = b.as_local_variable_read_node() {
            if let Some(call) = a.as_call_node() {
                if call.receiver().is_none() && call.arguments().is_none() && call.block().is_none()
                {
                    return lv.name().as_slice() == call.name().as_slice();
                }
            }
        }
        false
    }

    fn is_simple_identifier(node: &ruby_prism::Node<'_>) -> bool {
        if node.as_local_variable_read_node().is_some()
            || node.as_instance_variable_read_node().is_some()
            || node.as_class_variable_read_node().is_some()
            || node.as_global_variable_read_node().is_some()
        {
            return true;
        }
        // Bare method call (no receiver, no args) acts like a variable read
        if let Some(call) = node.as_call_node() {
            if call.receiver().is_none()
                && call.arguments().is_none()
                && call.block().is_none()
                && call.call_operator_loc().is_none()
            {
                return true;
            }
        }
        false
    }

    /// Check if the innermost receiver of a call chain matches a variable.
    /// Returns the chain depth if matched.
    fn matches_receiver_chain(
        node: &ruby_prism::Node<'_>,
        lhs: &ruby_prism::Node<'_>,
    ) -> Option<usize> {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                if Self::same_identifier(&recv, lhs) {
                    return Some(1);
                }
                // Recurse into the receiver chain
                if let Some(depth) = Self::matches_receiver_chain(&recv, lhs) {
                    return Some(depth + 1);
                }
            }
        }
        None
    }

    /// Check if a call node is a dotless operator method ([], []=, +, -, etc.)
    fn is_dotless_operator(call: &ruby_prism::CallNode<'_>) -> bool {
        // If there's a dot/call operator, it's not a dotless operator call
        if call.call_operator_loc().is_some() {
            return false;
        }
        let name = call.name().as_slice();
        // [] and []= subscript operators
        if name == b"[]" || name == b"[]=" {
            return true;
        }
        // Binary/unary operator methods (called without dot)
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
                | b"+@"
                | b"-@"
        )
    }

    /// Check if any call in the chain from innermost to outermost is a dotless operator
    fn has_dotless_operator_in_chain(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if Self::is_dotless_operator(&call) {
                return true;
            }
            // Walk up: check receiver chain
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    if Self::is_dotless_operator(&recv_call) {
                        return true;
                    }
                    // Continue recursing into the receiver chain
                    if Self::has_dotless_operator_in_chain(&recv) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a single method name is inherently unsafe for safe navigation.
    fn is_unsafe_single_method(name_bytes: &[u8]) -> bool {
        // empty? — nil&.empty? returns nil, not false, changing behavior
        if name_bytes == b"empty?" {
            return true;
        }
        // Assignment methods
        if name_bytes.ends_with(b"=") && !name_bytes.ends_with(b"==") {
            return true;
        }
        false
    }

    /// Check if any method in the call chain is unsafe for safe navigation conversion.
    /// This checks:
    /// 1. Methods that nil responds to (is_a?, respond_to?, etc.)
    /// 2. Methods in the AllowedMethods config
    /// 3. Inherently unsafe methods (empty?, assignment methods)
    fn has_unsafe_method_in_chain(
        node: &ruby_prism::Node<'_>,
        allowed_methods: &Option<Vec<String>>,
    ) -> bool {
        if let Some(call) = node.as_call_node() {
            let name_bytes = call.name().as_slice();

            // Check if this method is inherently unsafe
            if Self::is_unsafe_single_method(name_bytes) {
                return true;
            }

            // Check if this method is one that nil responds to
            if NIL_METHODS.contains(&name_bytes) {
                return true;
            }

            // Check if this method is in the AllowedMethods config
            if let Some(allowed) = allowed_methods {
                if let Ok(name_str) = std::str::from_utf8(name_bytes) {
                    if allowed.iter().any(|m| m == name_str) {
                        return true;
                    }
                }
            }

            // Recurse into the receiver chain
            if let Some(recv) = call.receiver() {
                if Self::has_unsafe_method_in_chain(&recv, allowed_methods) {
                    return true;
                }
            }
        }
        false
    }

    /// Get the single statement from a StatementsNode, if exactly one.
    fn single_stmt_from_stmts<'a>(
        stmts: &ruby_prism::StatementsNode<'a>,
    ) -> Option<ruby_prism::Node<'a>> {
        let body: Vec<_> = stmts.body().iter().collect();
        if body.len() == 1 {
            Some(body.into_iter().next().unwrap())
        } else {
            None
        }
    }

    /// Check if a node is a nil literal.
    fn is_nil(node: &ruby_prism::Node<'_>) -> bool {
        node.as_nil_node().is_some()
    }
}

impl Cop for SafeNavigation {
    fn name(&self) -> &'static str {
        "Style/SafeNavigation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max_chain_length = config.get_usize("MaxChainLength", 2);
        let _convert_nil = config.get_bool("ConvertCodeThatCanStartToReturnNil", false);
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .or_else(|| Some(vec!["present?".to_string(), "blank?".to_string()]));

        let mut visitor = SafeNavVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            max_chain_length,
            allowed_methods,
            in_unsafe_parent: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Visitor that tracks whether we're inside a call node whose method is
/// an assignment method (e.g. `[]=`, `foo=`) or dotless operator (e.g. `+`, `>`).
/// RuboCop walks ancestors from the method call to detect this context;
/// we simulate it by tracking depth while visiting.
struct SafeNavVisitor<'a> {
    cop: &'a SafeNavigation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    max_chain_length: usize,
    allowed_methods: Option<Vec<String>>,
    in_unsafe_parent: usize,
}

impl<'a> SafeNavVisitor<'a> {
    /// Check if a call node is an unsafe parent context for safe navigation.
    /// This means the call is an assignment method (name ends with `=`) or
    /// a dotless operator call.
    fn is_unsafe_parent_call(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        // Assignment methods: []=, foo=, etc. (but not == or !=)
        if name.ends_with(b"=") && name != b"==" && name != b"!=" {
            return true;
        }
        // Dotless calls (no dot/safe-nav operator)
        if call.call_operator_loc().is_none() {
            // Binary/unary operator methods
            if matches!(
                name,
                b"+" | b"-"
                    | b"*"
                    | b"/"
                    | b"%"
                    | b"**"
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
            ) {
                return true;
            }
            // Dotless method calls with arguments or a block (e.g., `scope :bar, lambda`,
            // `puts(foo && foo.bar)`). RuboCop considers these unsafe because converting
            // `&&` to safe navigation inside their arguments changes behavior.
            // Excludes: `!` (negation) and bare names with no args (variable-like reads).
            if name != b"!" && (call.arguments().is_some() || call.block().is_some()) {
                return true;
            }
        }
        false
    }
}

impl<'a, 'pr> Visit<'pr> for SafeNavVisitor<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let is_unsafe = Self::is_unsafe_parent_call(node);
        if is_unsafe {
            self.in_unsafe_parent += 1;
        }
        ruby_prism::visit_call_node(self, node);
        if is_unsafe {
            self.in_unsafe_parent -= 1;
        }
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        // Skip if inside an assignment method, operator call, or dotless method call.
        // RuboCop skips `&&` patterns when any ancestor send node is "unsafe" (dotless,
        // assignment, or operator method). For example, `scope :bar, ->(user) { user && user.name }`
        // is not flagged because `scope` is a dotless method call.
        if self.in_unsafe_parent > 0 {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        let lhs = node.left();
        let rhs = node.right();

        // LHS must be a simple variable or bare method
        if !SafeNavigation::is_simple_identifier(&lhs) {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        // RHS must be a method call chain
        let rhs_call = match rhs.as_call_node() {
            Some(c) => c,
            None => {
                ruby_prism::visit_and_node(self, node);
                return;
            }
        };

        // The outermost call must use a dot operator
        if rhs_call.call_operator_loc().is_none() {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        // Check if the innermost receiver matches the LHS variable
        let chain_len = match SafeNavigation::matches_receiver_chain(&rhs, &lhs) {
            Some(d) => d,
            None => {
                ruby_prism::visit_and_node(self, node);
                return;
            }
        };

        if chain_len > self.max_chain_length {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        // Skip if any call in the chain uses a dotless operator
        if SafeNavigation::has_dotless_operator_in_chain(&rhs) {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        // Skip if any method in the chain is unsafe for safe navigation
        if SafeNavigation::has_unsafe_method_in_chain(&rhs, &self.allowed_methods) {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
        ));

        // Don't visit children — we already processed this and_node
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // Skip if inside an assignment method or operator call
        if self.in_unsafe_parent > 0 {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        let if_node = node;
        let node_loc = if_node.location();

        // Check if it's a ternary (no `if` keyword location in Prism)
        if if_node.if_keyword_loc().is_none() {
            let diags = self.cop.check_ternary(
                self.source,
                &node_loc,
                if_node,
                self.max_chain_length,
                &self.allowed_methods,
            );
            self.diagnostics.extend(diags);
            ruby_prism::visit_if_node(self, node);
            return;
        }

        // Check modifier if patterns: `foo.bar if foo`
        let kw = if_node.if_keyword_loc().unwrap();
        let is_unless = kw.as_slice() == b"unless";

        // Skip elsif
        if kw.as_slice() == b"elsif" {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        // Must be modifier form (no end keyword)
        if if_node.end_keyword_loc().is_some() {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        // Must not have else/elsif
        if if_node.subsequent().is_some() {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        let diags = self.cop.check_modifier_if(
            self.source,
            &node_loc,
            if_node,
            is_unless,
            self.max_chain_length,
            &self.allowed_methods,
        );
        self.diagnostics.extend(diags);

        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        if self.in_unsafe_parent > 0 {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        // Must be modifier form (no end keyword)
        if node.end_keyword_loc().is_some() {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        // Must not have else
        if node.else_clause().is_some() {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        let node_loc = node.location();
        let condition = node.predicate();
        let body_stmts = match node.statements() {
            Some(s) => s,
            None => {
                ruby_prism::visit_unless_node(self, node);
                return;
            }
        };

        // Must have exactly one body statement
        let body = match SafeNavigation::single_stmt_from_stmts(&body_stmts) {
            Some(n) => n,
            None => {
                ruby_prism::visit_unless_node(self, node);
                return;
            }
        };

        let bytes = self.source.as_bytes();

        // Extract checked_src: `unless foo.nil?` → check foo
        let checked_src: Option<&[u8]> = if let Some(call) = condition.as_call_node() {
            let name = call.name().as_slice();
            if name == b"nil?" {
                call.receiver()
                    .map(|r| &bytes[r.location().start_offset()..r.location().end_offset()])
            } else {
                None
            }
        } else {
            None
        };

        let checked_src = match checked_src {
            Some(s) => s,
            None => {
                ruby_prism::visit_unless_node(self, node);
                return;
            }
        };

        // Body must be a method call chain
        let body_call = match body.as_call_node() {
            Some(c) => c,
            None => {
                ruby_prism::visit_unless_node(self, node);
                return;
            }
        };

        if body_call.call_operator_loc().is_none() {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if !self.cop.find_receiver_by_bytes(&body, checked_src, bytes) {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        let chain_len = self.cop.chain_length_by_bytes(&body, checked_src, bytes);
        if chain_len > self.max_chain_length {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if SafeNavigation::has_dotless_operator_in_chain(&body) {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if SafeNavigation::has_unsafe_method_in_chain(&body, &self.allowed_methods) {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        let (line, column) = self.source.offset_to_line_col(node_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
        ));

        ruby_prism::visit_unless_node(self, node);
    }
}

impl SafeNavigation {
    fn check_ternary(
        &self,
        source: &SourceFile,
        node_loc: &ruby_prism::Location<'_>,
        if_node: &ruby_prism::IfNode<'_>,
        max_chain_length: usize,
        allowed_methods: &Option<Vec<String>>,
    ) -> Vec<Diagnostic> {
        let condition = if_node.predicate();
        let bytes = source.as_bytes();

        // Extract checked_variable range and determine which branch is the body
        // Patterns:
        // 1. foo.nil? ? nil : foo.bar  => checked_var = foo, body = else_branch
        // 2. !foo.nil? ? foo.bar : nil => checked_var = foo, body = if_branch
        // 3. foo ? foo.bar : nil       => checked_var = foo, body = if_branch

        // Determine condition type
        let (checked_var_range, body_is_else) = if let Some(call) = condition.as_call_node() {
            let name = call.name().as_slice();
            if name == b"nil?" {
                // foo.nil? ? nil : foo.bar
                if let Some(recv) = call.receiver() {
                    let range = (recv.location().start_offset(), recv.location().end_offset());
                    // if_branch must be nil
                    let if_is_nil = if_node
                        .statements()
                        .and_then(|s| Self::single_stmt_from_stmts(&s))
                        .is_none_or(|n| Self::is_nil(&n));
                    if !if_is_nil {
                        return Vec::new();
                    }
                    (range, true) // body is else branch
                } else {
                    return Vec::new();
                }
            } else if name == b"!" {
                // !foo or !foo.nil?
                if let Some(recv) = call.receiver() {
                    if let Some(inner_call) = recv.as_call_node() {
                        if inner_call.name().as_slice() == b"nil?" {
                            // !foo.nil? ? foo.bar : nil
                            if let Some(inner_recv) = inner_call.receiver() {
                                let range = (
                                    inner_recv.location().start_offset(),
                                    inner_recv.location().end_offset(),
                                );
                                // else_branch must be nil
                                let else_is_nil = self.else_branch_is_nil(if_node);
                                if !else_is_nil {
                                    return Vec::new();
                                }
                                (range, false) // body is if branch
                            } else {
                                return Vec::new();
                            }
                        } else {
                            // !foo ? nil : foo.bar
                            let range =
                                (recv.location().start_offset(), recv.location().end_offset());
                            let if_is_nil = if_node
                                .statements()
                                .and_then(|s| Self::single_stmt_from_stmts(&s))
                                .is_none_or(|n| Self::is_nil(&n));
                            if !if_is_nil {
                                return Vec::new();
                            }
                            (range, true) // body is else branch
                        }
                    } else {
                        // !foo ? nil : foo.bar
                        let range = (recv.location().start_offset(), recv.location().end_offset());
                        let if_is_nil = if_node
                            .statements()
                            .and_then(|s| Self::single_stmt_from_stmts(&s))
                            .is_none_or(|n| Self::is_nil(&n));
                        if !if_is_nil {
                            return Vec::new();
                        }
                        (range, true)
                    }
                } else {
                    return Vec::new();
                }
            } else {
                // foo ? foo.bar : nil => plain variable/expression check
                let range = (
                    condition.location().start_offset(),
                    condition.location().end_offset(),
                );
                // else_branch must be nil
                let else_is_nil = self.else_branch_is_nil(if_node);
                if !else_is_nil {
                    return Vec::new();
                }
                (range, false) // body is if branch
            }
        } else {
            // Non-call condition: foo ? foo.bar : nil
            let range = (
                condition.location().start_offset(),
                condition.location().end_offset(),
            );
            let else_is_nil = self.else_branch_is_nil(if_node);
            if !else_is_nil {
                return Vec::new();
            }
            (range, false)
        };

        // Get the body node (the non-nil branch)
        let body = if body_is_else {
            // Body is in else branch
            let subsequent = match if_node.subsequent() {
                Some(s) => s,
                None => return Vec::new(),
            };
            let else_node = match subsequent.as_else_node() {
                Some(e) => e,
                None => return Vec::new(),
            };
            match else_node
                .statements()
                .and_then(|s| Self::single_stmt_from_stmts(&s))
            {
                Some(n) => n,
                None => return Vec::new(),
            }
        } else {
            // Body is in if branch
            match if_node
                .statements()
                .and_then(|s| Self::single_stmt_from_stmts(&s))
            {
                Some(n) => n,
                None => return Vec::new(),
            }
        };

        // Body must be a method call chain with a dot operator
        let body_call = match body.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if body_call.call_operator_loc().is_none() {
            return Vec::new();
        }

        // Find matching receiver using source byte comparison
        let checked_src = &bytes[checked_var_range.0..checked_var_range.1];
        if !self.find_receiver_by_bytes(&body, checked_src, bytes) {
            return Vec::new();
        }

        let chain_len = self.chain_length_by_bytes(&body, checked_src, bytes);
        if chain_len > max_chain_length {
            return Vec::new();
        }

        // Check if the call directly on the matched receiver is a dotless operator
        if self.call_on_receiver_is_dotless_by_bytes(&body, checked_src, bytes) {
            return Vec::new();
        }

        // Skip if any method in the chain is unsafe for safe navigation
        if Self::has_unsafe_method_in_chain(&body, allowed_methods) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(node_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
        )]
    }

    fn else_branch_is_nil(&self, if_node: &ruby_prism::IfNode<'_>) -> bool {
        match if_node.subsequent() {
            Some(subsequent) => {
                match subsequent.as_else_node() {
                    Some(else_node) => {
                        match else_node.statements() {
                            Some(stmts) => {
                                match Self::single_stmt_from_stmts(&stmts) {
                                    Some(n) => Self::is_nil(&n),
                                    None => true, // empty else => nil
                                }
                            }
                            None => true, // no statements => nil
                        }
                    }
                    None => false,
                }
            }
            None => false, // no else branch at all — not the pattern we want
        }
    }

    fn find_receiver_by_bytes(
        &self,
        node: &ruby_prism::Node<'_>,
        checked_src: &[u8],
        bytes: &[u8],
    ) -> bool {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                let recv_src = &bytes[recv.location().start_offset()..recv.location().end_offset()];
                if recv_src == checked_src {
                    return true;
                }
                return self.find_receiver_by_bytes(&recv, checked_src, bytes);
            }
        }
        false
    }

    fn chain_length_by_bytes(
        &self,
        node: &ruby_prism::Node<'_>,
        checked_src: &[u8],
        bytes: &[u8],
    ) -> usize {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                let recv_src = &bytes[recv.location().start_offset()..recv.location().end_offset()];
                if recv_src == checked_src {
                    return 1;
                }
                return 1 + self.chain_length_by_bytes(&recv, checked_src, bytes);
            }
        }
        0
    }

    fn call_on_receiver_is_dotless_by_bytes(
        &self,
        node: &ruby_prism::Node<'_>,
        checked_src: &[u8],
        bytes: &[u8],
    ) -> bool {
        if let Some(call) = node.as_call_node() {
            if let Some(recv) = call.receiver() {
                let recv_src = &bytes[recv.location().start_offset()..recv.location().end_offset()];
                if recv_src == checked_src {
                    return Self::is_dotless_operator(&call);
                }
                return self.call_on_receiver_is_dotless_by_bytes(&recv, checked_src, bytes);
            }
        }
        false
    }

    fn check_modifier_if(
        &self,
        source: &SourceFile,
        node_loc: &ruby_prism::Location<'_>,
        if_node: &ruby_prism::IfNode<'_>,
        is_unless: bool,
        max_chain_length: usize,
        allowed_methods: &Option<Vec<String>>,
    ) -> Vec<Diagnostic> {
        let condition = if_node.predicate();
        let body_stmts = match if_node.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Must have exactly one body statement
        let body = match Self::single_stmt_from_stmts(&body_stmts) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let bytes = source.as_bytes();

        // Extract the checked variable source range from the condition
        let checked_src: Option<&[u8]> = if let Some(call) = condition.as_call_node() {
            let name = call.name().as_slice();
            if name == b"nil?" {
                // unless foo.nil? => check foo
                if is_unless {
                    call.receiver()
                        .map(|r| &bytes[r.location().start_offset()..r.location().end_offset()])
                } else {
                    return Vec::new();
                }
            } else if name == b"!" {
                // if !foo or if !foo.nil?
                call.receiver().and_then(|r| {
                    if let Some(inner) = r.as_call_node() {
                        if inner.name().as_slice() == b"nil?" {
                            inner.receiver().map(|ir| {
                                &bytes[ir.location().start_offset()..ir.location().end_offset()]
                            })
                        } else {
                            Some(&bytes[r.location().start_offset()..r.location().end_offset()])
                        }
                    } else {
                        Some(&bytes[r.location().start_offset()..r.location().end_offset()])
                    }
                })
            } else {
                // foo.bar if foo
                if !is_unless {
                    Some(
                        &bytes[condition.location().start_offset()
                            ..condition.location().end_offset()],
                    )
                } else {
                    return Vec::new();
                }
            }
        } else {
            // Plain variable: `foo.bar if foo`
            if !is_unless {
                Some(&bytes[condition.location().start_offset()..condition.location().end_offset()])
            } else {
                return Vec::new();
            }
        };

        let checked_src = match checked_src {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Body must be a method call chain
        let body_call = match body.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if body_call.call_operator_loc().is_none() {
            return Vec::new();
        }

        if !self.find_receiver_by_bytes(&body, checked_src, bytes) {
            return Vec::new();
        }

        let chain_len = self.chain_length_by_bytes(&body, checked_src, bytes);
        if chain_len > max_chain_length {
            return Vec::new();
        }

        if Self::has_dotless_operator_in_chain(&body) {
            return Vec::new();
        }

        // Skip if any method in the chain is unsafe for safe navigation
        if Self::has_unsafe_method_in_chain(&body, allowed_methods) {
            return Vec::new();
        }

        // RuboCop: use_var_only_in_unless_modifier? — for `unless foo`, skip
        // if the checked variable is used only in the condition (not a method call)
        if is_unless && !Self::is_method_called(&condition) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(node_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
        )]
    }

    /// Check if the condition node is a method call (has a parent send)
    fn is_method_called(node: &ruby_prism::Node<'_>) -> bool {
        // In RuboCop, this checks `send_node&.parent&.send_type?`
        // We approximate: if the condition itself is a call node with a receiver
        if let Some(call) = node.as_call_node() {
            return call.receiver().is_some();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SafeNavigation, "cops/style/safe_navigation");
}
