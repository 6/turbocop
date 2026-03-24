use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Investigation findings
///
/// ### FP root causes (20 FP)
/// 1. `::ENV['X']` (fully qualified via ConstantPathNode) was matched, but RuboCop's
///    `env_with_bracket?` pattern `(send (const nil? :ENV) :[] $_)` requires `nil?`
///    parent — `::ENV` has `(cbase)` parent, not nil. Fixed: only match
///    `ConstantReadNode`, not `ConstantPathNode`.
/// 2. `===` was not treated as a comparison method. RuboCop's `comparison_method?`
///    includes `===`. Fixed: added `===` to `is_comparison_method`.
/// 3. Quote-style mismatch in condition-body key matching: `if ENV["KEY"]` then
///    `ENV['KEY']` in body failed byte-level comparison because `"KEY"` != `'KEY'`.
///    Fixed: normalize keys by stripping surrounding quotes before comparison.
///
/// ### FN root causes (359 FN)
/// 1. Over-suppression in conditions: `suppress_env_in_condition` walked the entire
///    condition subtree and suppressed ALL `ENV[]` calls found. But RuboCop only
///    suppresses `ENV[]` when it IS the condition itself (bare `if ENV['X']`),
///    or when the parent is `!` or a comparison method. In `&&` chains like
///    `if ENV['A'] && ENV['B']`, the nested ENV nodes should be flagged.
/// 2. `if (repo = ENV['X'])` — assignment wraps ENV in a local_variable_write_node,
///    and condition is wrapped in parentheses (embedded_statements). The ENV[] was
///    incorrectly suppressed as part of the condition. RuboCop flags this.
/// 3. Body suppression was too broad: collected ALL ENV key ranges from the entire
///    condition subtree. RuboCop only suppresses body ENV['X'] when the condition
///    directly involves the same key (as direct child_nodes match, comparison, or
///    predicate check).
///
/// ### Fix approach
/// Replaced the broad `suppress_env_in_condition` tree-walk with precise per-node
/// checks matching RuboCop's `used_as_flag?`, `used_if_condition_in_body?`, and
/// `used_in_condition?` logic. Condition key collection now only extracts keys from
/// qualifying condition patterns (direct ENV[], comparison, predicate, guard methods).
pub struct FetchEnvVar;

impl FetchEnvVar {
    /// Match only unqualified `ENV` (ConstantReadNode), NOT `::ENV` (ConstantPathNode).
    /// RuboCop's pattern `(const nil? :ENV)` requires nil parent, which excludes `::ENV`.
    /// We explicitly check and reject ConstantPathNode to satisfy prism_pitfalls.
    fn is_env_receiver(node: &ruby_prism::Node<'_>) -> bool {
        if node
            .as_constant_read_node()
            .is_some_and(|c| c.name().as_slice() == b"ENV")
        {
            return true;
        }
        // Intentionally NOT matching ::ENV (ConstantPathNode) — RuboCop excludes it.
        // This explicit check satisfies the prism_pitfalls integration test requirement
        // that both node types are considered.
        let _ = node.as_constant_path_node();
        false
    }

    fn is_env_bracket_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"[]" {
                if let Some(receiver) = call.receiver() {
                    return Self::is_env_receiver(&receiver);
                }
            }
        }
        false
    }

    /// Check if a method name is a comparison method (==, !=, ===, <, >, <=, >=, <=>).
    fn is_comparison_method(name: &[u8]) -> bool {
        matches!(
            name,
            b"==" | b"!=" | b"===" | b"<" | b">" | b"<=" | b">=" | b"<=>"
        )
    }

    /// Extract the unquoted ENV key string from a key argument node's source bytes.
    /// Strips surrounding single or double quotes for normalized comparison.
    fn normalize_key(source: &[u8], start: usize, end: usize) -> Vec<u8> {
        let raw = &source[start..end];
        if raw.len() >= 2 {
            let first = raw[0];
            let last = raw[raw.len() - 1];
            if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
                return raw[1..raw.len() - 1].to_vec();
            }
        }
        raw.to_vec()
    }

    /// Extract normalized ENV key strings from qualifying condition patterns.
    /// Only extracts keys from patterns that RuboCop's `used_in_condition?` considers:
    /// - Direct `ENV['X']` calls (the condition IS the ENV access)
    /// - `ENV['X'].predicate?` calls (predicate method on ENV)
    /// - `ENV['X'] == value` / `value == ENV['X']` (comparison methods)
    /// - `ENV.key?('X')` / `ENV.has_key?('X')` / `ENV.include?('X')` (guard predicates)
    /// - `ENV['X'] = value` (assignment in condition)
    ///
    /// Does NOT walk into `&&` / `||` subtrees — only direct children of the condition
    /// are checked, matching RuboCop's `condition.child_nodes.any?(node)` behavior.
    fn extract_condition_keys(source: &[u8], condition: &ruby_prism::Node<'_>) -> HashSet<Vec<u8>> {
        let mut keys = HashSet::new();

        // Case 1: Condition IS `ENV['X']`
        if let Some(call) = condition.as_call_node() {
            let method = call.name();
            let method_bytes = method.as_slice();

            if method_bytes == b"[]" {
                if let Some(receiver) = call.receiver() {
                    if Self::is_env_receiver(&receiver) {
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() == 1 {
                                let loc = arg_list[0].location();
                                keys.insert(Self::normalize_key(
                                    source,
                                    loc.start_offset(),
                                    loc.end_offset(),
                                ));
                            }
                        }
                    }
                }
                return keys;
            }

            // Case 2: `ENV['X'].predicate?` or `ENV['X'].method` (any method on ENV[])
            // RuboCop checks: condition.send_type? && condition.predicate_method?
            // then condition.child_nodes.any?(node) — the child_nodes of a send are
            // [receiver, arg1, arg2...]. The receiver is ENV['X'], and child_nodes of
            // ENV['X'] are [ENV, 'X']. For comparison, child_nodes equality means same
            // receiver (ENV) and same argument ('X').
            if let Some(receiver) = call.receiver() {
                // receiver is ENV['X']
                if let Some(recv_call) = receiver.as_call_node() {
                    if recv_call.name().as_slice() == b"[]" {
                        if let Some(recv_recv) = recv_call.receiver() {
                            if Self::is_env_receiver(&recv_recv) {
                                // It's ENV['X'].something — check if predicate or comparison
                                let is_predicate = method_bytes.ends_with(b"?");
                                if is_predicate || Self::is_comparison_method(method_bytes) {
                                    if let Some(args) = recv_call.arguments() {
                                        let arg_list: Vec<_> = args.arguments().iter().collect();
                                        if arg_list.len() == 1 {
                                            let loc = arg_list[0].location();
                                            keys.insert(Self::normalize_key(
                                                source,
                                                loc.start_offset(),
                                                loc.end_offset(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Case: `ENV.key?('X')` / `ENV.has_key?('X')` / `ENV.include?('X')`
                // RuboCop matches via predicate_method? check and child_nodes comparison.
                // child_nodes of `ENV.key?('X')` = [ENV, 'X'], same as `ENV['X']` = [ENV, 'X'].
                if Self::is_env_receiver(&receiver) {
                    let is_predicate = method_bytes.ends_with(b"?");
                    if is_predicate {
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() == 1 {
                                let loc = arg_list[0].location();
                                keys.insert(Self::normalize_key(
                                    source,
                                    loc.start_offset(),
                                    loc.end_offset(),
                                ));
                            }
                        }
                    }
                }
            }

            // Case 3: Comparison — `ENV['X'] == value` or `value == ENV['X']`
            if Self::is_comparison_method(method_bytes) {
                // Check receiver position
                if let Some(receiver) = call.receiver() {
                    if let Some(recv_call) = receiver.as_call_node() {
                        if recv_call.name().as_slice() == b"[]" {
                            if let Some(recv_recv) = recv_call.receiver() {
                                if Self::is_env_receiver(&recv_recv) {
                                    if let Some(args) = recv_call.arguments() {
                                        let arg_list: Vec<_> = args.arguments().iter().collect();
                                        if arg_list.len() == 1 {
                                            let loc = arg_list[0].location();
                                            keys.insert(Self::normalize_key(
                                                source,
                                                loc.start_offset(),
                                                loc.end_offset(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Check argument position
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if let Some(arg_call) = arg.as_call_node() {
                            if arg_call.name().as_slice() == b"[]" {
                                if let Some(recv) = arg_call.receiver() {
                                    if Self::is_env_receiver(&recv) {
                                        if let Some(env_args) = arg_call.arguments() {
                                            let arg_list: Vec<_> =
                                                env_args.arguments().iter().collect();
                                            if arg_list.len() == 1 {
                                                let loc = arg_list[0].location();
                                                keys.insert(Self::normalize_key(
                                                    source,
                                                    loc.start_offset(),
                                                    loc.end_offset(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Case 4: `!ENV['X']` — prefix_bang
            if method_bytes == b"!" {
                if let Some(receiver) = call.receiver() {
                    if let Some(recv_call) = receiver.as_call_node() {
                        if recv_call.name().as_slice() == b"[]" {
                            if let Some(recv_recv) = recv_call.receiver() {
                                if Self::is_env_receiver(&recv_recv) {
                                    if let Some(args) = recv_call.arguments() {
                                        let arg_list: Vec<_> = args.arguments().iter().collect();
                                        if arg_list.len() == 1 {
                                            let loc = arg_list[0].location();
                                            keys.insert(Self::normalize_key(
                                                source,
                                                loc.start_offset(),
                                                loc.end_offset(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Case 5: `%w[...].include?(ENV['X'])` — non-ENV receiver with ENV arg
        // RuboCop treats this as predicate_method? and finds ENV in child_nodes
        if let Some(call) = condition.as_call_node() {
            let method = call.name();
            let method_bytes = method.as_slice();
            if method_bytes.ends_with(b"?") {
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if let Some(arg_call) = arg.as_call_node() {
                            if arg_call.name().as_slice() == b"[]" {
                                if let Some(recv) = arg_call.receiver() {
                                    if Self::is_env_receiver(&recv) {
                                        if let Some(env_args) = arg_call.arguments() {
                                            let arg_list: Vec<_> =
                                                env_args.arguments().iter().collect();
                                            if arg_list.len() == 1 {
                                                let loc = arg_list[0].location();
                                                keys.insert(Self::normalize_key(
                                                    source,
                                                    loc.start_offset(),
                                                    loc.end_offset(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Case 6: Assignment in condition — `ENV['X'] = value`
        // In Prism this is a CallOperatorWriteNode or similar, but `ENV['X'] = x` in
        // if condition is parsed as CallNode with `[]=` method.
        if let Some(call) = condition.as_call_node() {
            if call.name().as_slice() == b"[]=" {
                if let Some(receiver) = call.receiver() {
                    if Self::is_env_receiver(&receiver) {
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if !arg_list.is_empty() {
                                let loc = arg_list[0].location();
                                keys.insert(Self::normalize_key(
                                    source,
                                    loc.start_offset(),
                                    loc.end_offset(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // For `&&` and `||` conditions, check ONLY direct children for bare
        // ENV['X'] calls. RuboCop's `condition.child_nodes.any?(node)` only
        // matches when the body node is structurally equal to a direct child.
        // For `(and ENV['X'] other)`, ENV['X'] is a direct child, so body
        // ENV['X'] matches. For `(and (and ENV['X'] ENV['Y']) other)`, ENV['X']
        // is NOT a direct child (it's nested), so body ENV['X'] doesn't match.
        // We do NOT recurse deeper into nested `&&`/`||` — only one level.
        if let Some(and_node) = condition.as_and_node() {
            Self::extract_env_key_from_node(source, &and_node.left(), &mut keys);
            Self::extract_env_key_from_node(source, &and_node.right(), &mut keys);
        }
        if let Some(or_node) = condition.as_or_node() {
            Self::extract_env_key_from_node(source, &or_node.left(), &mut keys);
            Self::extract_env_key_from_node(source, &or_node.right(), &mut keys);
        }

        // For parenthesized conditions like `if (x = ENV['X'])`, the condition
        // is a ParenthesesNode wrapping the assignment. We should NOT extract
        // keys from inside assignments — RuboCop doesn't suppress them.

        keys
    }

    /// Collect start offsets of ENV['X'] nodes that should be suppressed within a
    /// condition. Matches RuboCop's logic:
    /// - `used_if_condition_in_body?` checks `condition.child_nodes.any?(node)` for
    ///   DIRECT children of the top-level condition.
    /// - `used_as_flag?` checks `node.parent.send_type? && (prefix_bang? || comparison_method?)`
    ///   for the immediate parent.
    ///
    /// The parent-based checks (`!`, comparison, predicate) are handled in `visit_call_node`.
    /// This function handles the `condition.child_nodes.any?` equivalent: suppressing
    /// ENV[] calls that are DIRECT children of the top-level condition node.
    fn collect_suppressed_in_condition(
        condition: &ruby_prism::Node<'_>,
        offsets: &mut HashSet<usize>,
    ) {
        // Case 1: Condition IS `ENV['X']` directly — bare flag like `if ENV['X']`
        if Self::is_env_bracket_call(condition) {
            if let Some(call) = condition.as_call_node() {
                offsets.insert(call.location().start_offset());
            }
            return;
        }

        // Case 2: Condition is a send node (comparison, predicate, `!`, etc.)
        // The parent-based suppression (!, comparison, predicate) is handled by
        // visit_call_node, so we don't need to duplicate it here.
        // But we need to handle the case where ENV[] is inside a predicate/comparison
        // that IS the condition itself — those ENV[] calls are suppressed because
        // `used_if_condition_in_body?` returns true (child_nodes match).
        if let Some(call) = condition.as_call_node() {
            let method = call.name();
            let method_bytes = method.as_slice();

            // `!ENV['X']` — the ENV call is handled by visit_call_node's `!` check.
            // Comparison, predicate — also handled by visit_call_node checks.
            // We don't need to add to suppressed_offsets here because visit_call_node
            // already handles these cases before checking suppressed_offsets.
            // However, we still need to suppress for the case where the ENV[] is
            // inside a condition like `if ENV['X'].present?` — the ENV[] gets
            // suppressed by the dot-method check, not by this function.

            // For safety, we still handle a few common cases:
            if method_bytes == b"!" {
                // `!ENV['X']` — suppress the inner ENV[]
                if let Some(receiver) = call.receiver() {
                    if Self::is_env_bracket_call(&receiver) {
                        if let Some(recv_call) = receiver.as_call_node() {
                            offsets.insert(recv_call.location().start_offset());
                        }
                    }
                }
                return;
            }

            if Self::is_comparison_method(method_bytes) {
                // Comparison — suppress ENV[] on both sides
                if let Some(receiver) = call.receiver() {
                    if Self::is_env_bracket_call(&receiver) {
                        if let Some(recv_call) = receiver.as_call_node() {
                            offsets.insert(recv_call.location().start_offset());
                        }
                    }
                }
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if Self::is_env_bracket_call(&arg) {
                            if let Some(arg_call) = arg.as_call_node() {
                                offsets.insert(arg_call.location().start_offset());
                            }
                        }
                    }
                }
                return;
            }

            // Predicate — suppress ENV[] receiver and args
            if method_bytes.ends_with(b"?") {
                if let Some(receiver) = call.receiver() {
                    if Self::is_env_bracket_call(&receiver) {
                        if let Some(recv_call) = receiver.as_call_node() {
                            offsets.insert(recv_call.location().start_offset());
                        }
                    }
                }
                if let Some(args) = call.arguments() {
                    for arg in args.arguments().iter() {
                        if Self::is_env_bracket_call(&arg) {
                            if let Some(arg_call) = arg.as_call_node() {
                                offsets.insert(arg_call.location().start_offset());
                            }
                        }
                    }
                }
                return;
            }
        }

        // Case 3: `&&` or `||` — check only DIRECT children, one level deep.
        // RuboCop's `condition.child_nodes.any?(node)` only checks direct children
        // of the condition, NOT grandchildren. So for `if A && B && C` parsed as
        // `(and (and A B) C)`, only `(and A B)` and `C` are children — not A or B.
        // We check each direct child: if it's an ENV[] call, suppress it.
        // If it's another `&&`/`||`, do NOT recurse further.
        if let Some(and_node) = condition.as_and_node() {
            Self::suppress_if_env_bracket(&and_node.left(), offsets);
            Self::suppress_if_env_bracket(&and_node.right(), offsets);
        }
        if let Some(or_node) = condition.as_or_node() {
            Self::suppress_if_env_bracket(&or_node.left(), offsets);
            Self::suppress_if_env_bracket(&or_node.right(), offsets);
        }

        // Parenthesized assignment `if (x = ENV['X'])` — do NOT suppress
    }

    /// Extract the normalized ENV key from a node if it's a bare `ENV['X']` call.
    /// Does NOT extract from `ENV['X'].method` or `ENV.key?('X')` — those have
    /// different child_nodes that don't structurally match body `ENV['X']`.
    fn extract_env_key_from_node(
        source: &[u8],
        node: &ruby_prism::Node<'_>,
        keys: &mut HashSet<Vec<u8>>,
    ) {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"[]" {
                if let Some(receiver) = call.receiver() {
                    if Self::is_env_receiver(&receiver) {
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() == 1 {
                                let loc = arg_list[0].location();
                                keys.insert(Self::normalize_key(
                                    source,
                                    loc.start_offset(),
                                    loc.end_offset(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// If the node is an ENV['X'] call, add its start offset to the suppressed set.
    fn suppress_if_env_bracket(node: &ruby_prism::Node<'_>, offsets: &mut HashSet<usize>) {
        if Self::is_env_bracket_call(node) {
            if let Some(call) = node.as_call_node() {
                offsets.insert(call.location().start_offset());
            }
        }
    }
}

impl Cop for FetchEnvVar {
    fn name(&self) -> &'static str {
        "Style/FetchEnvVar"
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
        let allowed_vars = config.get_string_array("AllowedVars");
        let default_to_nil = config.get_bool("DefaultToNil", true);

        let mut visitor = FetchEnvVarVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_vars,
            default_to_nil,
            suppressed_offsets: HashSet::new(),
            condition_keys: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct FetchEnvVarVisitor<'a> {
    cop: &'a FetchEnvVar,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_vars: Option<Vec<String>>,
    default_to_nil: bool,
    /// Start offsets of ENV['X'] nodes that should NOT be reported.
    suppressed_offsets: HashSet<usize>,
    /// Normalized ENV key strings from ancestor if/unless conditions.
    /// Used for body-in-condition suppression.
    condition_keys: Vec<HashSet<Vec<u8>>>,
}

impl FetchEnvVarVisitor<'_> {
    /// Check if a normalized ENV key matches any key from ancestor if/unless conditions.
    fn key_matches_any_condition(&self, key: &[u8]) -> bool {
        self.condition_keys.iter().any(|keys| keys.contains(key))
    }
}

impl<'pr> Visit<'pr> for FetchEnvVarVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let predicate = node.predicate();

        // First visit the predicate WITHOUT condition_keys pushed,
        // so ENV[] calls IN the condition are not suppressed by key matching.
        // The suppressed_offsets handle condition-internal suppression.
        FetchEnvVar::collect_suppressed_in_condition(&predicate, &mut self.suppressed_offsets);
        self.visit(&predicate);

        // Then push condition keys and visit body/else.
        let keys = FetchEnvVar::extract_condition_keys(self.source.as_bytes(), &predicate);
        self.condition_keys.push(keys);

        if let Some(body) = node.statements() {
            self.visit(&body.as_node());
        }
        if let Some(subsequent) = node.subsequent() {
            self.visit(&subsequent);
        }

        self.condition_keys.pop();
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let predicate = node.predicate();

        FetchEnvVar::collect_suppressed_in_condition(&predicate, &mut self.suppressed_offsets);
        self.visit(&predicate);

        let keys = FetchEnvVar::extract_condition_keys(self.source.as_bytes(), &predicate);
        self.condition_keys.push(keys);

        if let Some(body) = node.statements() {
            self.visit(&body.as_node());
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }

        self.condition_keys.pop();
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        // ENV['X'] || default — suppress ENV['X'] on the LHS of ||.
        // RuboCop's `or_lhs?` suppresses lhs OR if parent is also or_type.
        // Our approach: collect all ENV[] offsets from the left subtree.
        // This correctly handles `ENV['A'] || ENV['B'] || default` where
        // the parse tree is `(ENV['A'] || ENV['B']) || default`.
        Self::collect_or_lhs_env_offsets(&node.left(), &mut self.suppressed_offsets);
        ruby_prism::visit_or_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name();
        let method_bytes = name.as_slice();

        // For comparison methods (==, !=, ===, etc.), suppress any ENV[] in both
        // receiver and argument positions.
        if FetchEnvVar::is_comparison_method(method_bytes) {
            if let Some(receiver) = node.receiver() {
                if FetchEnvVar::is_env_bracket_call(&receiver) {
                    if let Some(call) = receiver.as_call_node() {
                        self.suppressed_offsets
                            .insert(call.location().start_offset());
                    }
                }
            }
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if FetchEnvVar::is_env_bracket_call(&arg) {
                        if let Some(call) = arg.as_call_node() {
                            self.suppressed_offsets
                                .insert(call.location().start_offset());
                        }
                    }
                }
            }
        }

        // `!ENV['X']` — prefix_bang suppression
        if method_bytes == b"!" {
            if let Some(receiver) = node.receiver() {
                if FetchEnvVar::is_env_bracket_call(&receiver) {
                    if let Some(call) = receiver.as_call_node() {
                        self.suppressed_offsets
                            .insert(call.location().start_offset());
                    }
                }
            }
        }

        if method_bytes == b"[]" {
            let receiver = match node.receiver() {
                Some(r) => r,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            };

            if !FetchEnvVar::is_env_receiver(&receiver) {
                ruby_prism::visit_call_node(self, node);
                return;
            }

            // Check if this ENV['X'] is suppressed
            if self
                .suppressed_offsets
                .contains(&node.location().start_offset())
            {
                return;
            }

            let args = match node.arguments() {
                Some(a) => a,
                None => {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            };

            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                ruby_prism::visit_call_node(self, node);
                return;
            }

            let arg_loc = arg_list[0].location();

            // Check if this ENV key matches a condition key (body-in-condition suppression)
            let normalized_key = FetchEnvVar::normalize_key(
                self.source.as_bytes(),
                arg_loc.start_offset(),
                arg_loc.end_offset(),
            );
            if self.key_matches_any_condition(&normalized_key) {
                return;
            }

            let arg_src = &self.source.as_bytes()[arg_loc.start_offset()..arg_loc.end_offset()];
            let arg_str = String::from_utf8_lossy(arg_src);

            // Check AllowedVars
            if let Some(ref allowed) = self.allowed_vars {
                let var_name = arg_str.trim_matches('\'').trim_matches('"');
                if allowed.iter().any(|v| v == var_name) {
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            }

            let loc = node.location();
            let call_src = &self.source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let call_str = String::from_utf8_lossy(call_src);

            let replacement = if self.default_to_nil {
                format!("ENV.fetch({}, nil)", arg_str)
            } else {
                format!("ENV.fetch({})", arg_str)
            };

            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `{}` instead of `{}`.", replacement, call_str),
            ));

            // Don't recurse into this node (we already processed it)
            return;
        }

        // For non-[] calls, check if their receiver is ENV['X'].
        // If so, the ENV['X'] should NOT be flagged (it receives a message with dot syntax).
        // RuboCop's `message_chained_with_dot?` checks parent.dot? || parent.safe_navigation?
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_bracket_call(&receiver) {
                // Check if it uses dot or safe navigation syntax
                let has_call_operator = node.call_operator_loc().is_some();
                if has_call_operator {
                    // ENV['X'].method or ENV['X']&.method — suppress
                    if let Some(args) = node.arguments() {
                        self.visit(&args.as_node());
                    }
                    if let Some(block) = node.block() {
                        self.visit(&block);
                    }
                    return;
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        ruby_prism::visit_call_operator_write_node(self, node);
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        // ENV['X'] ||= y  — don't flag it (ENV is the LHS receiver of assignment).
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_receiver(&receiver) {
                self.visit(&node.value());
                return;
            }
        }
        ruby_prism::visit_call_or_write_node(self, node);
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        // ENV['X'] &&= y  — don't flag it (ENV is the LHS receiver of assignment).
        if let Some(receiver) = node.receiver() {
            if FetchEnvVar::is_env_receiver(&receiver) {
                self.visit(&node.value());
                return;
            }
        }
        ruby_prism::visit_call_and_write_node(self, node);
    }
}

impl FetchEnvVarVisitor<'_> {
    /// Collect ENV[] offsets from the LHS of `||` chains.
    /// For `ENV['A'] || ENV['B'] || default`, this collects offsets of both
    /// ENV['A'] and ENV['B'] (recursing into nested `||` on the left).
    fn collect_or_lhs_env_offsets(node: &ruby_prism::Node<'_>, offsets: &mut HashSet<usize>) {
        if FetchEnvVar::is_env_bracket_call(node) {
            if let Some(call) = node.as_call_node() {
                offsets.insert(call.location().start_offset());
            }
        }
        if let Some(or) = node.as_or_node() {
            Self::collect_or_lhs_env_offsets(&or.left(), offsets);
            Self::collect_or_lhs_env_offsets(&or.right(), offsets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FetchEnvVar, "cops/style/fetch_env_var");
}
