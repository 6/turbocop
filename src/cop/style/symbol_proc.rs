use crate::cop::shared::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, FORWARDING_SUPER_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, SUPER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/SymbolProc — suggests replacing `{ |x| x.foo }` with `(&:foo)`.
///
/// ## Investigation findings (2026-03-15)
///
/// Root causes of false positives (312 FP across 83 repos):
///
/// 1. **Missing `AllowedPatterns` support** — config was read but never applied.
///    RuboCop's `matches_allowed_pattern?` does regex matching on the outer method name.
///
/// 2. **Missing `AllowComments` support** — config was read but never checked.
///    When `AllowComments: true`, blocks containing comments should be skipped.
///
/// 3. **Missing `unsafe_hash_usage` check** — `{foo: 42}.select { |x| x.bar }` and
///    `{foo: 42}.reject { |x| x.bar }` should be skipped (hash literal + select/reject).
///
/// 4. **Missing `unsafe_array_usage` check** — `[1,2,3].min { |x| x.bar }` and
///    `[1,2,3].max { |x| x.bar }` should be skipped (array literal + min/max).
///
/// 5. **Missing `destructuring_block_argument` check** — blocks with trailing comma
///    like `{ |x,| x.foo }` should be skipped (destructuring hint).
///
/// 6. **Missing `ActiveSupportExtensionsEnabled` check** — when enabled (common in
///    Rails projects), `proc { |x| x.foo }`, `lambda { |x| x.foo }`, and
///    `Proc.new { |x| x.foo }` should NOT be flagged.
///
/// 7. **`AllowMethodsWithArguments` was incorrectly gating the inner-method-args
///    check** — inner method having arguments should ALWAYS skip (can't convert
///    `{ |x| x.foo(bar) }` to `&:foo`). `AllowMethodsWithArguments` controls whether
///    to skip when the *outer* method (the one receiving the block) has arguments.
///
/// All fixes applied in this revision.
///
/// ## Investigation findings (2026-03-17)
///
/// **FP root cause (remaining):** `proc`/`lambda`/`Proc.new` blocks were only
/// skipped when `ActiveSupportExtensionsEnabled: true`. RuboCop ALWAYS skips
/// these via `lambda_or_proc?` regardless of config. Fix: moved the skip
/// outside the ActiveSupport guard.
///
/// **FN root cause (100 FNs across 13 repos):** Ruby 3.4 `it` blocks and
/// numbered parameter `_1` blocks were not detected. These parse as blocks
/// with `ItParametersNode` or `NumberedParametersNode` params (not
/// `BlockParametersNode`). Fix: added detection for both patterns.
///
/// ## Investigation findings (2026-03-21)
///
/// **FN root cause (3 FNs across 2 repos):** `super { |x| x.foo }` and
/// `super(args) do |x| x.foo end` were not detected. The cop only handled
/// `CallNode` blocks but not `SuperNode` (explicit args) or
/// `ForwardingSuperNode` (no args). RuboCop's `symbol_proc_receiver?` matcher
/// explicitly handles `{(call ...) (super ...) zsuper}`. Fix: added
/// `check_super_block` and `check_forwarding_super_block` methods to handle
/// both cases, with appropriate config checks (AllowMethodsWithArguments
/// applies to SuperNode arguments; ForwardingSuperNode has no arguments).
pub struct SymbolProc;

impl Cop for SymbolProc {
    fn name(&self) -> &'static str {
        "Style/SymbolProc"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            FORWARDING_SUPER_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            SUPER_NODE,
        ]
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
        let allow_methods_with_arguments = config.get_bool("AllowMethodsWithArguments", false);
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let allow_comments = config.get_bool("AllowComments", false);
        let _active_support = config.get_bool("ActiveSupportExtensionsEnabled", false);

        // Handle three cases: CallNode, SuperNode, and ForwardingSuperNode
        // All three can have blocks attached
        if let Some(call_with_block) = node.as_call_node() {
            self.check_call_block(
                source,
                &call_with_block,
                parse_result,
                allow_methods_with_arguments,
                &allowed_methods,
                &allowed_patterns,
                allow_comments,
                diagnostics,
            );
        } else if let Some(super_node) = node.as_super_node() {
            self.check_super_block(
                source,
                &super_node,
                parse_result,
                allow_methods_with_arguments,
                allow_comments,
                diagnostics,
            );
        } else if let Some(fwd_super) = node.as_forwarding_super_node() {
            self.check_forwarding_super_block(
                source,
                &fwd_super,
                parse_result,
                allow_comments,
                diagnostics,
            );
        }
    }
}

impl SymbolProc {
    /// Check a CallNode with an attached block (e.g., `items.map { |x| x.foo }`).
    #[allow(clippy::too_many_arguments)]
    fn check_call_block(
        &self,
        source: &SourceFile,
        call_with_block: &ruby_prism::CallNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        allow_methods_with_arguments: bool,
        allowed_methods: &Option<Vec<String>>,
        allowed_patterns: &Option<Vec<String>>,
        allow_comments: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let block = match call_with_block.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Always skip proc/lambda/Proc.new blocks — RuboCop's `lambda_or_proc?` check
        // skips these unconditionally (not just when ActiveSupportExtensionsEnabled).
        // `proc { |x| x.foo }` creates a Proc object, not a method call with a block.
        {
            let outer_name = call_with_block.name().as_slice();
            if (outer_name == b"proc" || outer_name == b"lambda")
                && call_with_block.receiver().is_none()
            {
                return;
            }
            // Skip `Proc.new { |x| x.foo }` and `::Proc.new { |x| x.foo }`
            if outer_name == b"new" {
                if let Some(receiver) = call_with_block.receiver() {
                    if is_proc_constant(&receiver) {
                        return;
                    }
                }
            }
        }

        // Check unsafe_hash_usage: skip {}.select/reject (see rubocop#10864)
        let outer_method = call_with_block.name().as_slice();
        if (outer_method == b"select" || outer_method == b"reject")
            && is_hash_literal_receiver(call_with_block)
        {
            return;
        }

        // Check unsafe_array_usage: skip [].min/max
        if (outer_method == b"min" || outer_method == b"max")
            && is_array_literal_receiver(call_with_block)
        {
            return;
        }

        // Check outer method name against AllowedMethods
        if let Some(allowed) = allowed_methods {
            if let Ok(name_str) = std::str::from_utf8(outer_method) {
                if allowed.iter().any(|m| m == name_str) {
                    return;
                }
            }
        }

        // Check outer method name against AllowedPatterns (regex)
        if let Some(patterns) = allowed_patterns {
            if let Ok(name_str) = std::str::from_utf8(outer_method) {
                for pattern in patterns {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if re.is_match(name_str) {
                            return;
                        }
                    }
                }
            }
        }

        // AllowMethodsWithArguments: when true, skip if the outer method has arguments
        if allow_methods_with_arguments && call_with_block.arguments().is_some() {
            return;
        }

        // Check the block body for symbol proc pattern
        self.check_block_body(
            source,
            &block,
            parse_result,
            "the method",
            allow_comments,
            diagnostics,
        );
    }

    /// Check a SuperNode with an attached block (e.g., `super(args) { |x| x.foo }`).
    fn check_super_block(
        &self,
        source: &SourceFile,
        super_node: &ruby_prism::SuperNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        allow_methods_with_arguments: bool,
        allow_comments: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let block = match super_node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // AllowMethodsWithArguments: when true, skip if super has arguments
        if allow_methods_with_arguments && super_node.arguments().is_some() {
            return;
        }

        // Check the block body for symbol proc pattern
        self.check_block_body(
            source,
            &block,
            parse_result,
            "`super`",
            allow_comments,
            diagnostics,
        );
    }

    /// Check a ForwardingSuperNode with an attached block (e.g., `super { |x| x.foo }`).
    fn check_forwarding_super_block(
        &self,
        source: &SourceFile,
        fwd_super: &ruby_prism::ForwardingSuperNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        allow_comments: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let block = match fwd_super.block() {
            Some(bn) => bn,
            None => return,
        };

        // Check the block body for symbol proc pattern
        self.check_block_body(
            source,
            &block,
            parse_result,
            "`super`",
            allow_comments,
            diagnostics,
        );
    }

    /// Check the body of a block for the symbol proc pattern.
    fn check_block_body(
        &self,
        source: &SourceFile,
        block: &ruby_prism::BlockNode<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        method_desc: &str,
        allow_comments: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Extract the single method call from the block body
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        let call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must not use safe navigation (&.) - can't convert to &:method
        if let Some(op) = call.call_operator_loc() {
            if op.as_slice() == b"&." {
                return;
            }
        }

        // Inner method must not have arguments — can't convert { |x| x.foo(bar) } to &:foo
        if call.arguments().is_some() {
            return;
        }

        // Must not have a block
        if call.block().is_some() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if receiver matches the block parameter (three paths)
        let receiver_is_param = match block.parameters() {
            Some(params) => {
                // Path 1: Explicit params `{ |x| x.foo }`
                if let Some(block_params) = params.as_block_parameters_node() {
                    // Check for destructuring block argument: `{ |x,| x.foo }`
                    let params_loc = block_params.location();
                    let params_source =
                        &source.as_bytes()[params_loc.start_offset()..params_loc.end_offset()];
                    if params_source.contains(&b',') {
                        return;
                    }

                    let param_node = match block_params.parameters() {
                        Some(p) => p,
                        None => return,
                    };

                    let requireds: Vec<_> = param_node.requireds().iter().collect();
                    if requireds.len() != 1 {
                        return;
                    }

                    let param_name = if let Some(rp) = requireds[0].as_required_parameter_node() {
                        rp.name().as_slice()
                    } else {
                        return;
                    };

                    // Must have no rest, keyword, optional, or block params
                    if param_node.optionals().iter().count() > 0
                        || param_node.rest().is_some()
                        || param_node.keywords().iter().count() > 0
                        || param_node.keyword_rest().is_some()
                        || param_node.block().is_some()
                    {
                        return;
                    }

                    receiver
                        .as_local_variable_read_node()
                        .is_some_and(|lv| lv.name().as_slice() == param_name)
                } else if params.as_numbered_parameters_node().is_some() {
                    // Path 2: Numbered params `{ _1.foo }`
                    // Only _1 (first numbered param) is convertible to &:foo
                    receiver
                        .as_local_variable_read_node()
                        .is_some_and(|lv| lv.name().as_slice() == b"_1")
                } else if params.as_it_parameters_node().is_some() {
                    // Path 3: Ruby 3.4 `it` blocks `{ it.foo }`
                    receiver.as_it_local_variable_read_node().is_some()
                } else {
                    false
                }
            }
            None => false,
        };

        if !receiver_is_param {
            return;
        }

        // AllowComments: when true, skip if the block contains any comments
        if allow_comments {
            let block_loc = block.location();
            if has_comment_in_range(
                parse_result,
                block_loc.start_offset(),
                block_loc.end_offset(),
            ) {
                return;
            }
        }

        let method_name = call.name().as_slice();

        let loc = block.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Pass `&:{}` as an argument to {} instead of a block.",
                String::from_utf8_lossy(method_name),
                method_desc,
            ),
        ));
    }
}

/// Check if a node is the constant `Proc` or `::Proc`.
fn is_proc_constant(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == b"Proc";
    }
    if let Some(cp) = node.as_constant_path_node() {
        // ::Proc — parent is None (cbase), child is Proc
        if cp.parent().is_none() {
            if let Some(name) = cp.name() {
                return name.as_slice() == b"Proc";
            }
        }
    }
    false
}

/// Check if the call's receiver is a hash literal.
fn is_hash_literal_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver()
        .as_ref()
        .is_some_and(|r| r.as_hash_node().is_some() || r.as_keyword_hash_node().is_some())
}

/// Check if the call's receiver is an array literal.
fn is_array_literal_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver()
        .as_ref()
        .and_then(|r| r.as_array_node())
        .is_some()
}

/// Check if any comment falls within the given byte range.
fn has_comment_in_range(
    parse_result: &ruby_prism::ParseResult<'_>,
    start: usize,
    end: usize,
) -> bool {
    for comment in parse_result.comments() {
        let comment_start = comment.location().start_offset();
        if comment_start >= start && comment_start < end {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full_with_config, run_cop_full_with_config};

    crate::cop_fixture_tests!(SymbolProc, "cops/style/symbol_proc");

    fn config_with_allowed(methods: &[&str]) -> CopConfig {
        let mut config = CopConfig::default();
        let allowed: Vec<serde_yml::Value> = methods
            .iter()
            .map(|m| serde_yml::Value::String(m.to_string()))
            .collect();
        config.options.insert(
            "AllowedMethods".to_string(),
            serde_yml::Value::Sequence(allowed),
        );
        config
    }

    #[test]
    fn allowed_methods_skips_outer_method() {
        let config = config_with_allowed(&["respond_to"]);
        let source = b"respond_to do |format|\n  format.html\nend\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn non_allowed_method_still_fires() {
        let config = config_with_allowed(&["respond_to"]);
        let source = b"items.map { |x| x.to_s }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn allowed_patterns_skips_matching_method() {
        let mut config = CopConfig::default();
        config.options.insert(
            "AllowedPatterns".to_string(),
            serde_yml::Value::Sequence(vec![serde_yml::Value::String("respond_".to_string())]),
        );
        let source = b"respond_to { |format| format.xml }\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn allow_comments_skips_block_with_comment() {
        let mut config = CopConfig::default();
        config
            .options
            .insert("AllowComments".to_string(), serde_yml::Value::Bool(true));
        let source = b"something do |e|\n  # comment\n  e.upcase\nend\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn allow_comments_false_still_fires_with_comment() {
        let mut config = CopConfig::default();
        config
            .options
            .insert("AllowComments".to_string(), serde_yml::Value::Bool(false));
        let source = b"something do |e|\n  # comment\n  e.upcase\nend\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn active_support_skips_proc_blocks() {
        let mut config = CopConfig::default();
        config.options.insert(
            "ActiveSupportExtensionsEnabled".to_string(),
            serde_yml::Value::Bool(true),
        );
        let source = b"proc { |x| x.foo }\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn active_support_skips_proc_new_blocks() {
        let mut config = CopConfig::default();
        config.options.insert(
            "ActiveSupportExtensionsEnabled".to_string(),
            serde_yml::Value::Bool(true),
        );
        let source = b"Proc.new { |x| x.foo }\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn active_support_disabled_still_skips_proc_blocks() {
        // proc/lambda/Proc.new are ALWAYS skipped regardless of ActiveSupport
        let mut config = CopConfig::default();
        config.options.insert(
            "ActiveSupportExtensionsEnabled".to_string(),
            serde_yml::Value::Bool(false),
        );
        let source = b"proc { |x| x.foo }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn hash_select_reject_skipped() {
        let source = b"{foo: 42}.select { |item| item.bar }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);

        let source = b"{foo: 42}.reject { |item| item.bar }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn array_min_max_skipped() {
        let source = b"[1, 2, 3].min { |item| item.foo }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);

        let source = b"[1, 2, 3].max { |item| item.foo }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn array_select_reject_still_fires() {
        let source = b"[1, 2, 3].select { |item| item.foo }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn hash_min_max_still_fires() {
        let source = b"{foo: 42}.min { |item| item.foo }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn destructuring_trailing_comma_skipped() {
        let source = b"something { |x,| x.first }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn allow_methods_with_arguments_skips_outer_args() {
        let mut config = CopConfig::default();
        config.options.insert(
            "AllowMethodsWithArguments".to_string(),
            serde_yml::Value::Bool(true),
        );
        let source = b"do_something(one, two) { |x| x.test }\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn allow_methods_with_arguments_false_fires_with_outer_args() {
        let mut config = CopConfig::default();
        config.options.insert(
            "AllowMethodsWithArguments".to_string(),
            serde_yml::Value::Bool(false),
        );
        let source = b"do_something(one, two) { |x| x.test }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, config);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn it_block_fires() {
        let source = b"items.map { it.to_s }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn numbered_param_fires() {
        let source = b"items.map { _1.to_s }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn numbered_param_2_no_offense() {
        let source = b"something { _2.first }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn active_support_skips_proc_it_block() {
        let mut config = CopConfig::default();
        config.options.insert(
            "ActiveSupportExtensionsEnabled".to_string(),
            serde_yml::Value::Bool(true),
        );
        let source = b"proc { it.method }\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn active_support_skips_proc_numbered_param() {
        let mut config = CopConfig::default();
        config.options.insert(
            "ActiveSupportExtensionsEnabled".to_string(),
            serde_yml::Value::Bool(true),
        );
        let source = b"proc { _1.method }\n";
        assert_cop_no_offenses_full_with_config(&SymbolProc, source, config);
    }

    #[test]
    fn it_block_safe_nav_no_offense() {
        let source = b"items.map { it&.name }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn it_block_with_inner_args_no_offense() {
        let source = b"items.map { it.to_s(16) }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn it_block_hash_select_no_offense() {
        let source = b"{foo: 42}.select { it.bar }\n";
        let diags = run_cop_full_with_config(&SymbolProc, source, CopConfig::default());
        assert_eq!(diags.len(), 0);
    }
}
