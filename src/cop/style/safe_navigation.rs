use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Matches RuboCop's guarded receiver handling for modifier `if`/`unless`, adjacent
/// clauses inside chained `&&` guards like `proof && dom_body && dom_body.include?(proof)`,
/// and ternaries such as `uri.port = port ? port.to_i : nil`.
/// The current fix narrows two over-broad ancestor skips that caused corpus false
/// negatives: `&&` and modifier guards inside ordinary `do`/`{}` blocks attached to
/// dotless calls like `loop`/`scope`, and candidates nested inside receiver trees such
/// as array or block receivers that later receive another call (`[].pack`, `end.flatten`).
/// Direct receiver cases like `(foo && foo.bar).to_s` remain skipped, as do unsafe
/// argument, dynamic-send, double-colon-argument, and negated-wrapper contexts.
pub struct SafeNavigation;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AndOperatorKind {
    KeywordAnd,
    DoubleAmpersand,
}

struct TernaryCheckContext<'a> {
    max_chain_length: usize,
    allowed_methods: &'a Option<Vec<String>>,
    offense_start_offset: Option<usize>,
    skip_nested_block_call_args: bool,
    skip_direct_receiver_block_body_block_calls: bool,
}

struct ModifierIfCheckContext<'a> {
    max_chain_length: usize,
    allowed_methods: &'a Option<Vec<String>>,
    skip_direct_receiver_block_body_block_calls: bool,
}

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
    b"to_json",
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
                | b"==="
                | b"=="
                | b"!="
                | b"=~"
                | b"!~"
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

    fn call_chain_from_checked_receiver<'a>(
        node: &ruby_prism::Node<'a>,
        checked_src: &[u8],
        bytes: &[u8],
    ) -> Option<Vec<ruby_prism::CallNode<'a>>> {
        if let Some(parentheses) = node.as_parentheses_node() {
            if let Some(body) = parentheses.body() {
                if let Some(stmts) = body.as_statements_node() {
                    if let Some(inner) = Self::single_stmt_from_stmts(&stmts) {
                        return Self::call_chain_from_checked_receiver(&inner, checked_src, bytes);
                    }
                }
            }
        }

        let call = node.as_call_node()?;
        let receiver = call.receiver()?;
        let receiver_loc = receiver.location();

        if &bytes[receiver_loc.start_offset()..receiver_loc.end_offset()] == checked_src {
            return Some(vec![call]);
        }

        if receiver.as_parentheses_node().is_some() {
            return None;
        }

        let mut chain = Self::call_chain_from_checked_receiver(&receiver, checked_src, bytes)?;
        chain.push(call);
        Some(chain)
    }

    fn has_unsafe_method_after_checked_receiver(
        chain: &[ruby_prism::CallNode<'_>],
        allowed_methods: &Option<Vec<String>>,
    ) -> bool {
        for (index, call) in chain.iter().enumerate() {
            let name_bytes = call.name().as_slice();

            if Self::is_unsafe_single_method(name_bytes) {
                return true;
            }

            if index == 0 {
                continue;
            }

            if NIL_METHODS.contains(&name_bytes) {
                return true;
            }

            if let Some(allowed) = allowed_methods {
                if let Ok(name_str) = std::str::from_utf8(name_bytes) {
                    if allowed.iter().any(|method| method == name_str) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn chain_has_dotless_operator(chain: &[ruby_prism::CallNode<'_>]) -> bool {
        chain.iter().any(Self::is_dotless_operator)
    }

    fn is_double_colon_call(call: &ruby_prism::CallNode<'_>) -> bool {
        call.call_operator_loc()
            .is_some_and(|operator| operator.as_slice() == b"::")
    }

    fn collect_and_clauses<'a>(
        node: ruby_prism::Node<'a>,
        bytes: &[u8],
        expected_operator: Option<AndOperatorKind>,
        clauses: &mut Vec<ruby_prism::Node<'a>>,
    ) {
        if let Some(parentheses) = node.as_parentheses_node() {
            if let Some(body) = parentheses.body() {
                if let Some(stmts) = body.as_statements_node() {
                    if let Some(inner) = Self::single_stmt_from_stmts(&stmts) {
                        if let Some(and) = inner.as_and_node() {
                            let operator = Self::and_operator_kind(&and, bytes);
                            if expected_operator.is_none_or(|expected| expected == operator) {
                                Self::collect_and_clauses(
                                    and.left(),
                                    bytes,
                                    Some(operator),
                                    clauses,
                                );
                                Self::collect_and_clauses(
                                    and.right(),
                                    bytes,
                                    Some(operator),
                                    clauses,
                                );
                                return;
                            }
                        }
                    }
                }
            }
        }

        if let Some(and) = node.as_and_node() {
            let operator = Self::and_operator_kind(&and, bytes);
            if expected_operator.is_none_or(|expected| expected == operator) {
                Self::collect_and_clauses(and.left(), bytes, Some(operator), clauses);
                Self::collect_and_clauses(and.right(), bytes, Some(operator), clauses);
                return;
            }
        }

        clauses.push(node);
    }

    fn and_operator_kind(node: &ruby_prism::AndNode<'_>, bytes: &[u8]) -> AndOperatorKind {
        let left_loc = node.left().location();
        let right_loc = node.right().location();
        let between = &bytes[left_loc.end_offset()..right_loc.start_offset()];

        if between.windows(2).any(|window| window == b"&&") {
            AndOperatorKind::DoubleAmpersand
        } else {
            AndOperatorKind::KeywordAnd
        }
    }

    fn top_level_and_clauses<'a>(
        node: &ruby_prism::AndNode<'a>,
        bytes: &[u8],
    ) -> Vec<ruby_prism::Node<'a>> {
        let mut clauses = Vec::new();
        let operator = Self::and_operator_kind(node, bytes);
        Self::collect_and_clauses(node.as_node(), bytes, Some(operator), &mut clauses);
        clauses
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
            in_nil_safe_call_ancestor: 0,
            in_ternary_operator_parent: 0,
            in_assignment_or_operator_parent: 0,
            dotted_assignment_parent_starts: Vec::new(),
            in_call_arguments: 0,
            in_block_argument: 0,
            in_block: 0,
            direct_call_receiver_roots: Vec::new(),
            direct_receiver_block_bodies: Vec::new(),
            in_dynamic_send_args: 0,
            in_double_colon_call_arguments: 0,
            in_and_clause_visit: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Visitor that tracks ancestor call context that makes a safe-navigation rewrite unsafe
/// or non-idiomatic: assignment/operator parents, dynamic send arguments, and cases where
/// a candidate expression is used as another call's receiver or argument subtree.
struct SafeNavVisitor<'a> {
    cop: &'a SafeNavigation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    max_chain_length: usize,
    allowed_methods: Option<Vec<String>>,
    in_unsafe_parent: usize,
    in_nil_safe_call_ancestor: usize,
    in_ternary_operator_parent: usize,
    in_assignment_or_operator_parent: usize,
    dotted_assignment_parent_starts: Vec<usize>,
    in_call_arguments: usize,
    in_block_argument: usize,
    in_block: usize,
    direct_call_receiver_roots: Vec<(usize, usize)>,
    direct_receiver_block_bodies: Vec<(usize, usize)>,
    in_dynamic_send_args: usize,
    in_double_colon_call_arguments: usize,
    in_and_clause_visit: usize,
}

impl<'a> SafeNavVisitor<'a> {
    fn visit_flattened_and_clauses<'pr>(&mut self, node: &ruby_prism::AndNode<'pr>) {
        let bytes = self.source.as_bytes();
        for clause in SafeNavigation::top_level_and_clauses(node, bytes) {
            self.in_and_clause_visit += 1;
            self.visit(&clause);
            self.in_and_clause_visit -= 1;
        }
    }

    fn visit_direct_and_node<'pr>(&mut self, node: &ruby_prism::AndNode<'pr>) {
        let lhs = node.left();
        let rhs = node.right();

        if lhs.as_parentheses_node().is_some() {
            ruby_prism::visit_and_node(self, node);
            return;
        }
        let bytes = self.source.as_bytes();
        let checked_src = {
            let loc = lhs.location();
            &bytes[loc.start_offset()..loc.end_offset()]
        };

        let chain = match SafeNavigation::call_chain_from_checked_receiver(&rhs, checked_src, bytes)
        {
            Some(chain) => chain,
            None => {
                ruby_prism::visit_and_node(self, node);
                return;
            }
        };

        if chain.len() > self.max_chain_length {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        if SafeNavigation::chain_has_dotless_operator(&chain) {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        if SafeNavigation::has_unsafe_method_after_checked_receiver(&chain, &self.allowed_methods) {
            ruby_prism::visit_and_node(self, node);
            return;
        }

        if self.is_direct_receiver_block_body(&node.as_node())
            && chain.iter().any(|call| call.block().is_some())
        {
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
    }

    fn is_assignment_or_operator_parent_call(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();

        if name.ends_with(b"=") && name != b"==" && name != b"!=" {
            return true;
        }

        call.call_operator_loc().is_none()
            && matches!(
                name,
                b"[]"
                    | b"+"
                    | b"-"
                    | b"*"
                    | b"/"
                    | b"%"
                    | b"**"
                    | b"==="
                    | b"=="
                    | b"!="
                    | b"=~"
                    | b"!~"
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

    /// Check if a call node is an unsafe parent context for safe navigation.
    /// This means the call is an assignment method (name ends with `=`) or
    /// a dotless operator call.
    fn is_unsafe_parent_call(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        // Negation wrappers like `!(foo && foo.bar)` or `!!(...)` are unsafe
        // because converting the inner guard to `&.` changes the boolean result.
        if name == b"!" {
            return true;
        }
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
                    | b"==="
                    | b"=~"
                    | b"!~"
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

    fn is_dynamic_send_call(call: &ruby_prism::CallNode<'_>) -> bool {
        matches!(
            call.name().as_slice(),
            b"send" | b"__send__" | b"public_send"
        )
    }

    fn is_ternary_operator_parent_call(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();

        if name == b"!" {
            return true;
        }

        call.call_operator_loc().is_none()
            && matches!(
                name,
                b"+" | b"-"
                    | b"*"
                    | b"/"
                    | b"%"
                    | b"**"
                    | b"==="
                    | b"=="
                    | b"!="
                    | b"=~"
                    | b"!~"
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
                    | b"+@"
                    | b"-@"
            )
    }

    fn is_dotted_assignment_parent_call(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        call.call_operator_loc().is_some() && name.ends_with(b"=") && name != b"==" && name != b"!="
    }

    fn is_nil_safe_call_ancestor(call: &ruby_prism::CallNode<'_>) -> bool {
        NIL_METHODS.contains(&call.name().as_slice())
    }

    fn collect_direct_receiver_ranges(
        node: &ruby_prism::Node<'_>,
        ranges: &mut Vec<(usize, usize)>,
    ) {
        let loc = node.location();
        ranges.push((loc.start_offset(), loc.end_offset()));

        if let Some(parentheses) = node.as_parentheses_node() {
            if let Some(body) = parentheses.body() {
                if let Some(stmts) = body.as_statements_node() {
                    if let Some(inner) = SafeNavigation::single_stmt_from_stmts(&stmts) {
                        Self::collect_direct_receiver_ranges(&inner, ranges);
                    }
                }
            }
        }
    }

    fn is_direct_call_receiver(&self, node: &ruby_prism::Node<'_>) -> bool {
        let loc = node.location();
        let range = (loc.start_offset(), loc.end_offset());
        self.direct_call_receiver_roots.contains(&range)
    }

    fn collect_direct_receiver_block_bodies(
        node: &ruby_prism::Node<'_>,
        bodies: &mut Vec<(usize, usize)>,
    ) {
        if let Some(parentheses) = node.as_parentheses_node() {
            if let Some(body) = parentheses.body() {
                if let Some(stmts) = body.as_statements_node() {
                    if let Some(inner) = SafeNavigation::single_stmt_from_stmts(&stmts) {
                        Self::collect_direct_receiver_block_bodies(&inner, bodies);
                    }
                }
            }
            return;
        }

        let block = if let Some(call) = node.as_call_node() {
            call.block().and_then(|block| block.as_block_node())
        } else {
            node.as_block_node()
        };
        let Some(block) = block else {
            return;
        };
        let Some(body) = block.body() else {
            return;
        };
        let Some(stmts) = body.as_statements_node() else {
            return;
        };
        let loc = stmts.location();
        bodies.push((loc.start_offset(), loc.end_offset()));
    }

    fn is_direct_receiver_block_body(&self, node: &ruby_prism::Node<'_>) -> bool {
        let loc = node.location();
        let range = (loc.start_offset(), loc.end_offset());
        self.direct_receiver_block_bodies.contains(&range)
            || self
                .direct_receiver_block_bodies
                .iter()
                .any(|(start, end)| range.0 >= *start && range.1 <= *end)
    }
}

impl<'a, 'pr> Visit<'pr> for SafeNavVisitor<'a> {
    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        self.in_block_argument += 1;
        ruby_prism::visit_block_argument_node(self, node);
        self.in_block_argument -= 1;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.in_block += 1;
        ruby_prism::visit_block_node(self, node);
        self.in_block -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let is_unsafe = Self::is_unsafe_parent_call(node);
        if is_unsafe {
            self.in_unsafe_parent += 1;
        }
        let is_nil_safe_call_ancestor = Self::is_nil_safe_call_ancestor(node);
        if is_nil_safe_call_ancestor {
            self.in_nil_safe_call_ancestor += 1;
        }
        let is_assignment_or_operator_parent = Self::is_assignment_or_operator_parent_call(node);
        if is_assignment_or_operator_parent {
            self.in_assignment_or_operator_parent += 1;
        }
        let is_ternary_operator_parent = Self::is_ternary_operator_parent_call(node);
        if is_ternary_operator_parent {
            self.in_ternary_operator_parent += 1;
        }
        let is_dotted_assignment_parent = Self::is_dotted_assignment_parent_call(node);
        if is_dotted_assignment_parent {
            self.dotted_assignment_parent_starts
                .push(node.location().start_offset());
        }
        if let Some(receiver) = node.receiver() {
            let receiver_roots_len = self.direct_call_receiver_roots.len();
            let receiver_block_bodies_len = self.direct_receiver_block_bodies.len();
            Self::collect_direct_receiver_ranges(&receiver, &mut self.direct_call_receiver_roots);
            Self::collect_direct_receiver_block_bodies(
                &receiver,
                &mut self.direct_receiver_block_bodies,
            );
            self.visit(&receiver);
            self.direct_call_receiver_roots.truncate(receiver_roots_len);
            self.direct_receiver_block_bodies
                .truncate(receiver_block_bodies_len);
        }

        if let Some(arguments) = node.arguments() {
            self.in_call_arguments += 1;
            let is_dynamic_send = Self::is_dynamic_send_call(node);
            let is_double_colon = SafeNavigation::is_double_colon_call(node);
            if is_dynamic_send {
                self.in_dynamic_send_args += 1;
            }
            if is_double_colon {
                self.in_double_colon_call_arguments += 1;
            }
            self.visit_arguments_node(&arguments);
            if is_dynamic_send {
                self.in_dynamic_send_args -= 1;
            }
            if is_double_colon {
                self.in_double_colon_call_arguments -= 1;
            }
            self.in_call_arguments -= 1;
        }

        if let Some(block) = node.block() {
            if is_unsafe && self.in_and_clause_visit == 0 {
                self.in_unsafe_parent -= 1;
            }
            let nil_safe_block_scope = is_nil_safe_call_ancestor;
            if nil_safe_block_scope {
                self.in_nil_safe_call_ancestor -= 1;
            }
            self.visit(&block);
            if nil_safe_block_scope {
                self.in_nil_safe_call_ancestor += 1;
            }
        }

        if is_assignment_or_operator_parent {
            self.in_assignment_or_operator_parent -= 1;
        }
        if is_ternary_operator_parent {
            self.in_ternary_operator_parent -= 1;
        }
        if is_dotted_assignment_parent {
            self.dotted_assignment_parent_starts.pop();
        }
        if is_nil_safe_call_ancestor {
            self.in_nil_safe_call_ancestor -= 1;
        }
        if is_unsafe && node.block().is_none_or(|_| self.in_and_clause_visit > 0) {
            self.in_unsafe_parent -= 1;
        }
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        if self.in_block_argument > 0 && self.in_block == 0 {
            self.visit_flattened_and_clauses(node);
            return;
        }

        // Skip if inside an assignment method, operator call, or dotless method call.
        // RuboCop skips `&&` patterns when any ancestor send node is "unsafe" (dotless,
        // assignment, or operator method). For example, `scope :bar, ->(user) { user && user.name }`
        // is not flagged because `scope` is a dotless method call.
        if self.is_direct_call_receiver(&node.as_node())
            || self.in_nil_safe_call_ancestor > 0
            || self.in_unsafe_parent > 0
            || self.in_dynamic_send_args > 0
            || self.in_double_colon_call_arguments > 0
        {
            self.visit_flattened_and_clauses(node);
            return;
        }

        if self.in_block > 0 {
            self.visit_direct_and_node(node);
            return;
        }

        let bytes = self.source.as_bytes();
        let clauses = SafeNavigation::top_level_and_clauses(node, bytes);
        let mut found_offense = false;

        for pair in clauses.windows(2) {
            let lhs = &pair[0];
            let rhs = &pair[1];

            if lhs.as_parentheses_node().is_some() {
                continue;
            }

            let checked_src = {
                let loc = lhs.location();
                &bytes[loc.start_offset()..loc.end_offset()]
            };

            let chain =
                match SafeNavigation::call_chain_from_checked_receiver(rhs, checked_src, bytes) {
                    Some(chain) => chain,
                    None => continue,
                };

            if chain.len() > self.max_chain_length {
                continue;
            }

            if SafeNavigation::chain_has_dotless_operator(&chain) {
                continue;
            }

            if SafeNavigation::has_unsafe_method_after_checked_receiver(
                &chain,
                &self.allowed_methods,
            ) {
                continue;
            }

            if self.is_direct_receiver_block_body(&node.as_node())
                && chain.iter().any(|call| call.block().is_some())
            {
                continue;
            }

            let loc = lhs.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use safe navigation (`&.`) instead of checking if an object exists before calling the method.".to_string(),
            ));
            found_offense = true;
        }

        if !found_offense {
            self.visit_flattened_and_clauses(node);
        }
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let if_node = node;

        // Check if it's a ternary (no `if` keyword location in Prism)
        if if_node.if_keyword_loc().is_none() {
            if (self.in_block_argument > 0 && self.in_block == 0)
                || self.is_direct_call_receiver(&node.as_node())
                || self.in_nil_safe_call_ancestor > 0
                || self.in_ternary_operator_parent > 0
            {
                ruby_prism::visit_if_node(self, node);
                return;
            }

            let diags = self.cop.check_ternary(
                self.source,
                if_node,
                TernaryCheckContext {
                    max_chain_length: self.max_chain_length,
                    allowed_methods: &self.allowed_methods,
                    offense_start_offset: self.dotted_assignment_parent_starts.last().copied(),
                    skip_nested_block_call_args: self.in_call_arguments > 1,
                    skip_direct_receiver_block_body_block_calls: self
                        .is_direct_receiver_block_body(&node.as_node()),
                },
            );
            self.diagnostics.extend(diags);
            ruby_prism::visit_if_node(self, node);
            return;
        }

        let node_loc = if_node.location();

        // Check modifier if patterns: `foo.bar if foo`
        let kw = if_node.if_keyword_loc().unwrap();
        let is_unless = kw.as_slice() == b"unless";

        // Skip elsif
        if kw.as_slice() == b"elsif" {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        // Must not have else/elsif
        if if_node.subsequent().is_some() {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        if self.in_assignment_or_operator_parent > 0 {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        if (self.in_block_argument > 0 && self.in_block == 0)
            || self.is_direct_call_receiver(&node.as_node())
            || self.in_nil_safe_call_ancestor > 0
            || self.in_call_arguments > 0
            || self.in_dynamic_send_args > 0
        {
            ruby_prism::visit_if_node(self, node);
            return;
        }

        let diags = self.cop.check_modifier_if(
            self.source,
            &node_loc,
            if_node,
            is_unless,
            ModifierIfCheckContext {
                max_chain_length: self.max_chain_length,
                allowed_methods: &self.allowed_methods,
                skip_direct_receiver_block_body_block_calls: self
                    .is_direct_receiver_block_body(&node.as_node()),
            },
        );
        self.diagnostics.extend(diags);

        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        // Must not have else
        if node.else_clause().is_some() {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if self.in_assignment_or_operator_parent > 0 {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if (self.in_block_argument > 0 && self.in_block == 0)
            || self.is_direct_call_receiver(&node.as_node())
            || self.in_nil_safe_call_ancestor > 0
            || self.in_call_arguments > 0
            || self.in_dynamic_send_args > 0
        {
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
                if method_dispatch_predicates::is_safe_navigation(&call) {
                    ruby_prism::visit_unless_node(self, node);
                    return;
                }
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

        let chain =
            match SafeNavigation::call_chain_from_checked_receiver(&body, checked_src, bytes) {
                Some(chain) => chain,
                None => {
                    ruby_prism::visit_unless_node(self, node);
                    return;
                }
            };

        if chain.len() > self.max_chain_length {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if SafeNavigation::chain_has_dotless_operator(&chain) {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if SafeNavigation::has_unsafe_method_after_checked_receiver(&chain, &self.allowed_methods) {
            ruby_prism::visit_unless_node(self, node);
            return;
        }

        if self.is_direct_receiver_block_body(&node.as_node())
            && chain.iter().any(|call| call.block().is_some())
        {
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
        if_node: &ruby_prism::IfNode<'_>,
        context: TernaryCheckContext<'_>,
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
                if method_dispatch_predicates::is_safe_navigation(&call) {
                    return Vec::new();
                }
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
                            if method_dispatch_predicates::is_safe_navigation(&inner_call) {
                                return Vec::new();
                            }
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

        if context.skip_nested_block_call_args && body_call.block().is_some() {
            return Vec::new();
        }

        if body_call.call_operator_loc().is_none() {
            return Vec::new();
        }

        // Find matching receiver using source byte comparison
        let checked_src = &bytes[checked_var_range.0..checked_var_range.1];
        let chain = match Self::call_chain_from_checked_receiver(&body, checked_src, bytes) {
            Some(chain) => chain,
            None => return Vec::new(),
        };

        if context.skip_direct_receiver_block_body_block_calls
            && chain.iter().any(|call| call.block().is_some())
        {
            return Vec::new();
        }

        if chain.len() > context.max_chain_length {
            return Vec::new();
        }

        if Self::chain_has_dotless_operator(&chain) {
            return Vec::new();
        }

        if Self::has_unsafe_method_after_checked_receiver(&chain, context.allowed_methods) {
            return Vec::new();
        }

        let node_loc = if_node.location();
        let (line, column) = source.offset_to_line_col(
            context
                .offense_start_offset
                .unwrap_or(node_loc.start_offset()),
        );
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

    fn check_modifier_if(
        &self,
        source: &SourceFile,
        node_loc: &ruby_prism::Location<'_>,
        if_node: &ruby_prism::IfNode<'_>,
        is_unless: bool,
        context: ModifierIfCheckContext<'_>,
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
                if method_dispatch_predicates::is_safe_navigation(&call) {
                    return Vec::new();
                }
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
                            if method_dispatch_predicates::is_safe_navigation(&inner) {
                                return None;
                            }
                            if is_unless {
                                None
                            } else {
                                inner.receiver().map(|ir| {
                                    &bytes[ir.location().start_offset()..ir.location().end_offset()]
                                })
                            }
                        } else if is_unless {
                            Some(&bytes[r.location().start_offset()..r.location().end_offset()])
                        } else {
                            None
                        }
                    } else if is_unless {
                        Some(&bytes[r.location().start_offset()..r.location().end_offset()])
                    } else {
                        None
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

        let chain = match Self::call_chain_from_checked_receiver(&body, checked_src, bytes) {
            Some(chain) => chain,
            None => return Vec::new(),
        };

        if context.skip_direct_receiver_block_body_block_calls
            && chain.iter().any(|call| call.block().is_some())
        {
            return Vec::new();
        }

        if chain.len() > context.max_chain_length {
            return Vec::new();
        }

        if Self::chain_has_dotless_operator(&chain) {
            return Vec::new();
        }

        if Self::has_unsafe_method_after_checked_receiver(&chain, context.allowed_methods) {
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
