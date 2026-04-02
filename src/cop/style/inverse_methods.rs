use crate::cop::node_type::{CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// Corpus investigation (2026-04-02):
///
/// - Repo config files that declared only a partial `InverseMethods` or
///   `InverseBlocks` hash caused nitrocop to treat that hash as a full
///   replacement. RuboCop starts from its defaults, applies overrides, then adds
///   the inverted direction, so we now merge project config with the built-ins.
/// - RuboCop's matcher only targets calls with an explicit receiver. Keeping that
///   guard avoids flagging implicit-self definitions like `def empty?; !any?; end`
///   or `def without_platforms; select { ... }; end`.
/// - RuboCop does not treat `!(/pattern/ =~ value)` like a normal `!(foo =~ bar)`
///   inversion because regexp-left matches use a different AST shape there. We
///   now skip only the regexp-left `=~` form, while still flagging ordinary
///   `!(foo =~ /bar/)`.
/// - RuboCop only suppresses comparison inversions for CamelCase constant
///   hierarchy checks like `!(Foo < Bar)`. The earlier "any constant" guard was
///   too broad and hid real offenses such as `!(@file.class <= IO)` and
///   `!(RUBY_VERSION >= '2.8.0')`.
pub struct InverseMethods;

impl InverseMethods {
    fn build_inverse_map(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        const DEFAULTS: &[(&str, &str)] = &[
            ("any?", "none?"),
            ("even?", "odd?"),
            ("==", "!="),
            ("=~", "!~"),
            ("<", ">="),
            (">", "<="),
        ];

        Self::build_symmetric_inverse_map(config, "InverseMethods", DEFAULTS)
    }

    fn build_inverse_blocks(config: &CopConfig) -> HashMap<Vec<u8>, String> {
        const DEFAULTS: &[(&str, &str)] = &[("select", "reject"), ("select!", "reject!")];

        Self::build_symmetric_inverse_map(config, "InverseBlocks", DEFAULTS)
    }

    fn build_symmetric_inverse_map(
        config: &CopConfig,
        key: &str,
        defaults: &[(&str, &str)],
    ) -> HashMap<Vec<u8>, String> {
        let mut one_way = HashMap::new();
        for &(method, inverse) in defaults {
            one_way.insert(method.as_bytes().to_vec(), inverse.to_string());
        }

        if let Some(configured) = config.options.get(key).and_then(|value| value.as_mapping()) {
            for (configured_key, configured_value) in configured {
                let Some(configured_key) = configured_key.as_str() else {
                    continue;
                };

                let trimmed_key = configured_key.trim_start_matches(':');
                let key_bytes = trimmed_key.as_bytes().to_vec();

                if configured_value.is_null() {
                    one_way.remove(&key_bytes);
                    continue;
                }

                let Some(configured_value) = configured_value.as_str() else {
                    continue;
                };

                let trimmed_value = configured_value.trim_start_matches(':');
                one_way.insert(key_bytes, trimmed_value.to_string());
            }
        }

        let mut symmetric = one_way.clone();
        for (method, inverse) in one_way {
            symmetric.insert(
                inverse.as_bytes().to_vec(),
                String::from_utf8_lossy(&method).into(),
            );
        }

        symmetric
    }

    /// Check if this `!` call is the inner part of a double negation `!!`.
    /// Returns true if the byte immediately preceding the `!` operator in source is also `!`,
    /// indicating a `!!expr` pattern used for boolean coercion (not true inversion).
    fn is_double_negation(call: &ruby_prism::CallNode<'_>, source: &SourceFile) -> bool {
        // Use message_loc to find the exact position of the `!` operator
        if let Some(msg_loc) = call.message_loc() {
            let bang_start = msg_loc.start_offset();
            if bang_start > 0 {
                let bytes = source.as_bytes();
                // Scan backwards past whitespace to find preceding character
                let mut pos = bang_start - 1;
                while pos > 0 && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                    pos -= 1;
                }
                if bytes[pos] == b'!' {
                    return true;
                }
            }
        }
        false
    }

    /// Methods that are incompatible with safe navigation (`&.`).
    /// `any?` and `none?` return booleans; `nil&.any?` would raise NoMethodError.
    /// Comparison operators also can't be used with `&.` in this context.
    const SAFE_NAVIGATION_INCOMPATIBLE: &'static [&'static [u8]] =
        &[b"any?", b"none?", b"<", b">", b"<=", b">="];

    /// Check if the inner call uses safe navigation (`&.`) with a method that is
    /// incompatible with inversion. E.g., `!foo&.any?` can't become `foo&.none?`
    /// because `nil.none?` doesn't exist.
    fn is_safe_navigation_incompatible(
        inner_call: &ruby_prism::CallNode<'_>,
        source: &SourceFile,
    ) -> bool {
        if let Some(op_loc) = inner_call.call_operator_loc() {
            let op = source.byte_slice(op_loc.start_offset(), op_loc.end_offset(), "");
            if op == "&." {
                let method = inner_call.name().as_slice();
                return Self::SAFE_NAVIGATION_INCOMPATIBLE.contains(&method);
            }
        }
        false
    }

    /// Check if the last expression of a block body is a negation.
    /// Returns true for: !expr, expr != ..., expr !~ ...
    fn last_expr_is_negated(block: &ruby_prism::BlockNode<'_>) -> bool {
        let body = match block.body() {
            Some(b) => b,
            None => return false,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return false,
        };
        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.is_empty() {
            return false;
        }
        let last = &body_nodes[body_nodes.len() - 1];
        Self::is_negated_expr(last)
    }

    fn is_negated_expr(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            let name = call.name().as_slice();
            // !expr
            if name == b"!" && call.receiver().is_some() {
                return true;
            }
            // expr != ...  or  expr !~ ...
            if name == b"!=" || name == b"!~" {
                return true;
            }
        }
        // For begin/parenthesized bodies, check the last statement
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    if let Some(last) = body_nodes.last() {
                        return Self::is_negated_expr(last);
                    }
                }
            }
        }
        false
    }

    /// Check if the block contains any `next` statements (guard clauses).
    fn has_next_statements(block: &ruby_prism::BlockNode<'_>) -> bool {
        let body = match block.body() {
            Some(b) => b,
            None => return false,
        };
        let mut finder = NextFinder { found: false };
        ruby_prism::Visit::visit(&mut finder, &body);
        finder.found
    }

    /// RuboCop suppresses nested `!any?` / `!(x =~ y)` offenses when they sit
    /// inside the block of a larger `select`/`reject` inverse-block offense.
    fn nested_inside_inverse_block(
        target: &ruby_prism::CallNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> bool {
        let inverse_blocks = Self::build_inverse_blocks(config);
        let mut finder = NestedInverseBlockFinder {
            target_start: target.location().start_offset(),
            target_end: target.location().end_offset(),
            inverse_blocks: &inverse_blocks,
            found: false,
        };
        ruby_prism::Visit::visit(&mut finder, &parse_result.node());
        finder.found
    }

    /// RuboCop accepts `foo || !(bar.any? { ... }) ? a : b` and similar ternary
    /// predicates whose condition is an `||` expression. Keep the suppression
    /// scoped to that exact enclosing shape instead of skipping ternaries
    /// broadly, because plain `!(foo =~ /bar/) ? a : b` is still an offense.
    fn inside_or_ternary_predicate(
        target: &ruby_prism::CallNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
    ) -> bool {
        let mut finder = OrTernaryPredicateFinder {
            target_start: target.location().start_offset(),
            target_end: target.location().end_offset(),
            found: false,
        };
        ruby_prism::Visit::visit(&mut finder, &parse_result.node());
        finder.found
    }
}

impl Cop for InverseMethods {
    fn name(&self) -> &'static str {
        "Style/InverseMethods"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, PARENTHESES_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_bytes = call.name().as_slice();

        // Pattern 1: !receiver.method — the call is `!` with the inner being a method call
        if method_bytes == b"!" {
            // Skip double negation `!!expr` — used for boolean coercion, not inversion
            if Self::is_double_negation(&call, source) {
                return;
            }

            if Self::nested_inside_inverse_block(&call, parse_result, config) {
                return;
            }

            if Self::inside_or_ternary_predicate(&call, parse_result) {
                return;
            }

            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            // Try to get the inner call - either directly from receiver or by unwrapping parens
            let inner_call = if let Some(c) = receiver.as_call_node() {
                c
            } else if let Some(parens) = receiver.as_parentheses_node() {
                let body = match parens.body() {
                    Some(b) => b,
                    None => return,
                };
                let stmts = match body.as_statements_node() {
                    Some(s) => s,
                    None => return,
                };
                let stmts_list: Vec<_> = stmts.body().iter().collect();
                if stmts_list.len() != 1 {
                    return;
                }
                match stmts_list[0].as_call_node() {
                    Some(c) => c,
                    None => return,
                }
            } else {
                return;
            };

            let inner_method = inner_call.name().as_slice();

            // Skip safe navigation with incompatible methods (e.g., !foo&.any?)
            if Self::is_safe_navigation_incompatible(&inner_call, source) {
                return;
            }

            // RuboCop only matches explicit receivers for inverse method checks.
            if inner_call.receiver().is_none() {
                return;
            }

            // Check InverseMethods (predicate methods: !foo.any? -> foo.none?)
            let inverse_methods = InverseMethods::build_inverse_map(config);
            if let Some(inv) = inverse_methods.get(inner_method) {
                // RuboCop only skips CamelCase module/class hierarchy checks.
                if is_comparison_operator(inner_method)
                    && possible_class_hierarchy_check(&inner_call, source)
                {
                    return;
                }

                // Parser/RuboCop treat regexp-left `=~` matches differently from
                // ordinary send nodes, so `!(/pattern/ =~ value)` is accepted.
                if inner_method == b"=~" && regexp_left_match(&inner_call) {
                    return;
                }

                let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
                ));
            }

            // Check InverseBlocks (block methods: !foo.select { } -> foo.reject { })
            let inverse_blocks = InverseMethods::build_inverse_blocks(config);
            if inner_call.block().is_some() {
                if let Some(inv) = inverse_blocks.get(inner_method) {
                    let inner_name = std::str::from_utf8(inner_method).unwrap_or("method");
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use `{}` instead of inverting `{}`.", inv, inner_name),
                    ));
                }
            }

            return;
        }

        // Pattern 2: foo.select { |f| !f.even? } or foo.reject { |k, v| v != :a }
        // Block where the method is in InverseBlocks and the last expression is negated
        if call.receiver().is_none() {
            return;
        }

        let inverse_blocks = InverseMethods::build_inverse_blocks(config);
        if let Some(inv) = inverse_blocks.get(method_bytes) {
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if InverseMethods::last_expr_is_negated(&block_node)
                        && !InverseMethods::has_next_statements(&block_node)
                    {
                        let method_name = std::str::from_utf8(method_bytes).unwrap_or("method");
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use `{}` instead of inverting `{}`.", inv, method_name),
                        ));
                    }
                }
            }
        }
    }
}

struct NextFinder {
    found: bool,
}

impl<'pr> ruby_prism::Visit<'pr> for NextFinder {
    fn visit_next_node(&mut self, _node: &ruby_prism::NextNode<'pr>) {
        self.found = true;
    }

    // Don't recurse into nested blocks/lambdas/defs
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

struct NestedInverseBlockFinder<'a> {
    target_start: usize,
    target_end: usize,
    inverse_blocks: &'a HashMap<Vec<u8>, String>,
    found: bool,
}

impl<'pr> ruby_prism::Visit<'pr> for NestedInverseBlockFinder<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found {
            return;
        }

        if node.receiver().is_some() {
            if let Some(block) = node.block().and_then(|block| block.as_block_node()) {
                let block_loc = block.location();
                if block_loc.start_offset() <= self.target_start
                    && self.target_end <= block_loc.end_offset()
                    && self.inverse_blocks.contains_key(node.name().as_slice())
                    && InverseMethods::last_expr_is_negated(&block)
                    && !InverseMethods::has_next_statements(&block)
                {
                    self.found = true;
                    return;
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

struct OrTernaryPredicateFinder {
    target_start: usize,
    target_end: usize,
    found: bool,
}

impl<'pr> ruby_prism::Visit<'pr> for OrTernaryPredicateFinder {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if self.found {
            return;
        }

        if node.if_keyword_loc().is_none() {
            let predicate = node.predicate();
            if let Some(or_node) = predicate.as_or_node() {
                let loc = or_node.location();
                if loc.start_offset() <= self.target_start && self.target_end <= loc.end_offset() {
                    self.found = true;
                    return;
                }
            }
        }

        ruby_prism::visit_if_node(self, node);
    }
}

/// Returns true if the method name is a comparison operator.
fn is_comparison_operator(method: &[u8]) -> bool {
    matches!(method, b"<" | b">" | b"<=" | b">=")
}

fn possible_class_hierarchy_check(call: &ruby_prism::CallNode<'_>, source: &SourceFile) -> bool {
    let lhs_is_camel_case = call
        .receiver()
        .is_some_and(|receiver| camel_case_constant(&receiver, source));

    let rhs_is_single_camel_case = call.arguments().is_some_and(|args| {
        let args: Vec<_> = args.arguments().iter().collect();
        args.len() == 1 && camel_case_constant(&args[0], source)
    });

    lhs_is_camel_case || rhs_is_single_camel_case
}

fn camel_case_constant(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if node.as_constant_read_node().is_none() && node.as_constant_path_node().is_none() {
        return false;
    }

    let loc = node.location();
    let text = source.byte_slice(loc.start_offset(), loc.end_offset(), "");
    contains_camel_case(text.as_bytes())
}

fn contains_camel_case(bytes: &[u8]) -> bool {
    let mut uppercase_run = 0usize;

    for &byte in bytes {
        if byte.is_ascii_uppercase() {
            uppercase_run += 1;
            continue;
        }

        if byte.is_ascii_lowercase() {
            if uppercase_run > 0 {
                return true;
            }
            uppercase_run = 0;
            continue;
        }

        uppercase_run = 0;
    }

    false
}

fn regexp_left_match(call: &ruby_prism::CallNode<'_>) -> bool {
    let Some(receiver) = call.receiver() else {
        return false;
    };

    receiver.as_regular_expression_node().is_some()
        || receiver.as_interpolated_regular_expression_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InverseMethods, "cops/style/inverse_methods");

    #[test]
    fn merges_partial_inverse_methods_config_with_defaults() {
        use crate::testutil::assert_cop_offenses_full_with_config;

        let fixture = br#"!items.none?
^ Style/InverseMethods: Use `any?` instead of inverting `none?`.
!(a <= b)
^ Style/InverseMethods: Use `>` instead of inverting `<=`.
"#;

        let config = CopConfig {
            options: HashMap::from([(
                "InverseMethods".into(),
                serde_yml::from_str("present?: blank?\ninclude?: exclude?\n").unwrap(),
            )]),
            ..CopConfig::default()
        };

        assert_cop_offenses_full_with_config(&InverseMethods, fixture, config);
    }

    #[test]
    fn merges_partial_inverse_blocks_config_with_defaults() {
        use crate::testutil::assert_cop_offenses_full_with_config;

        let fixture = br#"items.reject { |x| !x.valid? }
^ Style/InverseMethods: Use `select` instead of inverting `reject`.
"#;

        let config = CopConfig {
            options: HashMap::from([(
                "InverseBlocks".into(),
                serde_yml::from_str("filter_map: compact_map\n").unwrap(),
            )]),
            ..CopConfig::default()
        };

        assert_cop_offenses_full_with_config(&InverseMethods, fixture, config);
    }
}
