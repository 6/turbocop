use crate::cop::node_type::{CALL_NODE, FORWARDING_SUPER_NODE, LAMBDA_NODE, SUPER_NODE};
use crate::cop::util::{collect_foldable_ranges, collect_heredoc_ranges, count_body_lines_ex};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=105, FN=5.
///
/// A high-volume FP pattern was blocks whose body is only a heredoc expression:
/// `render do; <<~RUBY ... RUBY; end`.
///
/// In RuboCop (Parser AST), that body is a `str`/`dstr` node whose source range
/// is just the heredoc opening line (`<<~RUBY`), so it counts as one body line.
/// Our Prism implementation counted the full physical heredoc content range,
/// producing false positives on large documentation/example blocks.
///
/// Fix: detect "single heredoc expression body" and count it as one line.
///
/// Additional investigation (same run):
/// - FN: `super do ... end` blocks were not analyzed because only `CallNode`-backed blocks were handled.
/// - FP: `Data.define(...) do ... end` constructor blocks were counted, while RuboCop exempts them like `Struct.new`.
/// - FN: brace-style blocks where the last body token shares the closing `}` line
///   (e.g. `lambda { Hash.new(...) }`) were undercounted by one line.
///
/// Fixes:
/// - Analyze `SuperNode` and `ForwardingSuperNode` blocks as method name `super`.
/// - Extend constructor exemption to include `Data.define`.
/// - When block body and closing token share a line, count that trailing body line.
///
/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=39, FN=0.
///
/// Root cause: RuboCop's `source_from_node_with_heredoc` uses `each_descendant`
/// (which excludes the body node itself) to find max last_line. When a block body
/// contains heredocs, lines belonging to the root body node (e.g. a trailing `)`
/// on its own line in `Hash.new(...)`) are excluded from the count. nitrocop was
/// using `body.location().end_offset()` which includes the full body span,
/// overcounting by 1 line in these cases.
///
/// Fix: when the body contains heredoc descendants, compute the end line as the
/// max last_line across all descendants only (via `heredoc_descendant_max_line`),
/// matching RuboCop's `source_from_node_with_heredoc` algorithm.
///
/// Result: FP=0, FN=0 (within file-drop noise).
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=28, FN=193.
///
/// FN root cause identified: `heredoc_descendant_max_line` only tracked end
/// lines for StringNode and InterpolatedStringNode descendants. When a block
/// body contained heredocs AND code after the heredoc, the descendant max line
/// was set to the heredoc terminator line, cutting off subsequent code lines.
///
/// An attempt was made to rewrite `heredoc_descendant_max_line` to track ALL
/// descendant node end lines (commit 15aeae64, reverted). The approach used
/// visit_branch_node_enter/visit_leaf_node_enter callbacks with depth tracking
/// to exclude the root body node (matching Parser's each_descendant semantics).
///
/// The fix was reverted because corpus validation produced catastrophic 130K
/// offenses (vs 19K expected). Root cause of the regression was missing bundle
/// symlinks in the worktree (bench/corpus/vendor/bundle), causing nitrocop to
/// fall back to hardcoded defaults for ALL cops, not just BlockLength. The fix
/// itself was never validated in a proper environment.
///
/// A correct fix should: (1) rewrite heredoc_descendant_max_line to track all
/// descendant types, (2) validate in an environment with proper corpus bundle
/// symlinks, (3) handle the Parser each_descendant root-exclusion semantics.
pub struct BlockLength;

impl Cop for BlockLength {
    fn name(&self) -> &'static str {
        "Metrics/BlockLength"
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SUPER_NODE, FORWARDING_SUPER_NODE, LAMBDA_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle lambda nodes: ->(x) do...end / ->(x) {...}
        if let Some(lambda_node) = node.as_lambda_node() {
            self.check_lambda(source, &lambda_node, config, diagnostics);
            return;
        }

        if let Some(call_node) = node.as_call_node() {
            let block_node = match call_node.block().and_then(|b| b.as_block_node()) {
                Some(b) => b,
                None => return,
            };
            // RuboCop skips class constructor blocks (Struct.new, Class.new, etc.)
            if is_class_constructor(&call_node) {
                return;
            }
            self.check_invocation_block(
                source,
                std::str::from_utf8(call_node.name().as_slice()).unwrap_or(""),
                call_node.location().start_offset(),
                &block_node,
                config,
                diagnostics,
            );
            return;
        }

        if let Some(super_node) = node.as_super_node() {
            let block_node = match super_node.block().and_then(|b| b.as_block_node()) {
                Some(b) => b,
                None => return,
            };
            self.check_invocation_block(
                source,
                "super",
                super_node.location().start_offset(),
                &block_node,
                config,
                diagnostics,
            );
            return;
        }

        if let Some(forwarding_super_node) = node.as_forwarding_super_node() {
            let block_node = match forwarding_super_node.block() {
                Some(b) => b,
                None => return,
            };
            self.check_invocation_block(
                source,
                "super",
                forwarding_super_node.location().start_offset(),
                &block_node,
                config,
                diagnostics,
            );
        }
    }
}

impl BlockLength {
    fn check_invocation_block(
        &self,
        source: &SourceFile,
        method_name: &str,
        offense_offset: usize,
        block_node: &ruby_prism::BlockNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let max = config.get_usize("Max", 25);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            for pat in patterns {
                if let Ok(re) = regex::Regex::new(pat) {
                    if re.is_match(method_name) {
                        return;
                    }
                }
            }
        }

        let end_offset = block_node.closing_loc().start_offset();
        let count = count_block_lines(
            source,
            block_node.opening_loc().start_offset(),
            end_offset,
            block_node.closing_loc().end_offset(),
            block_node.body(),
            count_comments,
            &count_as_one,
        );

        if count > max {
            let (line, column) = source.offset_to_line_col(offense_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Block has too many lines. [{count}/{max}]"),
            ));
        }
    }

    fn check_lambda(
        &self,
        source: &SourceFile,
        lambda_node: &ruby_prism::LambdaNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let max = config.get_usize("Max", 25);
        let count_comments = config.get_bool("CountComments", false);
        let count_as_one = config.get_string_array("CountAsOne");

        let end_offset = lambda_node.closing_loc().start_offset();
        let count = count_block_lines(
            source,
            lambda_node.opening_loc().start_offset(),
            end_offset,
            lambda_node.closing_loc().end_offset(),
            lambda_node.body(),
            count_comments,
            &count_as_one,
        );

        if count > max {
            let (line, column) = source.offset_to_line_col(lambda_node.location().start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Block has too many lines. [{count}/{max}]"),
            ));
        }
    }
}

/// Count body lines for a block, folding heredocs and CountAsOne constructs.
/// Uses the body node's start offset (not opening_loc) to avoid counting
/// heredoc content lines that physically appear before the body starts.
fn count_block_lines(
    source: &SourceFile,
    opening_offset: usize,
    end_offset: usize,
    closing_end_offset: usize,
    body: Option<ruby_prism::Node<'_>>,
    count_comments: bool,
    count_as_one: &Option<Vec<String>>,
) -> usize {
    let body = match body {
        Some(b) => b,
        None => return 0,
    };

    // Parser/RuboCop behavior: when a block body is a single heredoc expression,
    // code length is based on the heredoc opener node source, not heredoc content.
    // This makes the body count as one line.
    if is_single_heredoc_expression(source, &body) {
        return 1;
    }

    // Use body start offset to skip heredoc content that appears before body.
    // Same approach as method_length.rs.
    let (body_start_line, _) = source.offset_to_line_col(body.location().start_offset());
    let effective_start_offset = if body_start_line > 1 {
        source
            .line_col_to_offset(body_start_line - 1, 0)
            .unwrap_or(opening_offset)
    } else {
        opening_offset
    };

    // RuboCop's `source_from_node_with_heredoc`: when the body contains any
    // heredoc descendants, RuboCop computes the end line as the max last_line
    // across all *descendants* (not the body node itself). In Parser AST,
    // `each_descendant` excludes the root node, so structural delimiters like
    // a trailing `)` on its own line (part of the root send/call node) are not
    // counted. Without this adjustment, nitrocop overcounts by including lines
    // that RuboCop's descendant-based algorithm skips.
    let mut effective_end_offset = end_offset;
    if let Some(desc_end_line) = heredoc_descendant_max_line(source, &body) {
        // Use descendant max line as the end boundary (matches RuboCop behavior)
        if let Some(next_line_start) = source.line_col_to_offset(desc_end_line + 1, 0) {
            effective_end_offset = next_line_start;
        }
    } else {
        // No heredocs: for brace blocks like `lambda { Hash.new(...) }`, the final
        // body token can share the same line as the closing `}`. RuboCop counts that
        // final line. `count_body_lines_ex` excludes the end line, so move end to
        // next line start.
        let (body_end_line, _) =
            source.offset_to_line_col(body.location().end_offset().saturating_sub(1));
        let (closing_line, _) = source.offset_to_line_col(end_offset);
        if body_end_line == closing_line {
            if let Some(next_line_start) = source.line_col_to_offset(closing_line + 1, 0) {
                effective_end_offset = next_line_start;
            } else {
                effective_end_offset = closing_end_offset;
            }
        }
    }

    // Collect foldable ranges from CountAsOne config. Heredocs are only
    // folded when "heredoc" is explicitly in CountAsOne (default: []).
    // For non-bare-heredoc bodies, RuboCop's CodeLengthCalculator includes
    // heredoc content lines by default. We replicate that here.
    let mut all_foldable: Vec<(usize, usize)> = Vec::new();
    if let Some(cao) = count_as_one {
        if !cao.is_empty() {
            all_foldable.extend(collect_foldable_ranges(source, &body, cao));
            if cao.iter().any(|s| s == "heredoc") {
                all_foldable.extend(collect_heredoc_ranges(source, &body));
            }
        }
    }
    all_foldable.sort();
    all_foldable.dedup();

    count_body_lines_ex(
        source,
        effective_start_offset,
        effective_end_offset,
        count_comments,
        &all_foldable,
    )
}

/// When the body contains heredoc descendants, compute the max last_line across
/// all descendants (matching RuboCop's `source_from_node_with_heredoc`).
/// Returns `Some(max_line)` if heredocs are found, `None` otherwise.
fn heredoc_descendant_max_line(source: &SourceFile, body: &ruby_prism::Node<'_>) -> Option<usize> {
    use ruby_prism::Visit;

    struct DescendantMaxLineFinder<'s> {
        source: &'s SourceFile,
        max_line: usize,
        has_heredoc: bool,
    }

    impl<'pr> Visit<'pr> for DescendantMaxLineFinder<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    self.has_heredoc = true;
                    // Use closing_loc (the EOS terminator) as the end line
                    if let Some(closing) = node.closing_loc() {
                        let (line, _) = self
                            .source
                            .offset_to_line_col(closing.end_offset().saturating_sub(1));
                        self.max_line = self.max_line.max(line);
                    }
                    return;
                }
            }
            let (line, _) = self
                .source
                .offset_to_line_col(node.location().end_offset().saturating_sub(1));
            self.max_line = self.max_line.max(line);
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            if let Some(opening) = node.opening_loc() {
                let bytes = &self.source.as_bytes()[opening.start_offset()..opening.end_offset()];
                if bytes.starts_with(b"<<") {
                    self.has_heredoc = true;
                    if let Some(closing) = node.closing_loc() {
                        let (line, _) = self
                            .source
                            .offset_to_line_col(closing.end_offset().saturating_sub(1));
                        self.max_line = self.max_line.max(line);
                    }
                    return;
                }
            }
            let (line, _) = self
                .source
                .offset_to_line_col(node.location().end_offset().saturating_sub(1));
            self.max_line = self.max_line.max(line);
            ruby_prism::visit_interpolated_string_node(self, node);
        }
    }

    let mut finder = DescendantMaxLineFinder {
        source,
        max_line: 0,
        has_heredoc: false,
    };
    // visit() traverses body and all descendants. Since body is typically
    // StatementsNode/BeginNode (not a string node), the visitor methods
    // only fire on actual descendants, matching RuboCop's each_descendant.
    finder.visit(body);

    if finder.has_heredoc {
        Some(finder.max_line)
    } else {
        None
    }
}

fn is_single_heredoc_expression(source: &SourceFile, body: &ruby_prism::Node<'_>) -> bool {
    if is_heredoc_node(source, body) {
        return true;
    }

    if let Some(stmts) = body.as_statements_node() {
        let mut iter = stmts.body().iter();
        if let Some(first) = iter.next() {
            return iter.next().is_none() && is_heredoc_node(source, &first);
        }
    }

    false
}

fn is_heredoc_node(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_string_node() {
        return s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
    }

    if let Some(s) = node.as_interpolated_string_node() {
        return s
            .opening_loc()
            .map(|o| source.as_bytes()[o.start_offset()..o.end_offset()].starts_with(b"<<"))
            .unwrap_or(false);
    }

    false
}

/// Check if a call is a class constructor like `Struct.new`, `Class.new`, `Module.new`, etc.
/// RuboCop's Metrics/BlockLength does not count these blocks.
fn is_class_constructor(call: &ruby_prism::CallNode<'_>) -> bool {
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };
    let recv_name = crate::cop::util::constant_name(&recv).unwrap_or_default();

    match call.name().as_slice() {
        b"new" => matches!(recv_name, b"Struct" | b"Class" | b"Module"),
        // Data.define is also a class constructor in RuboCop.
        b"define" => recv_name == b"Data",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockLength, "cops/metrics/block_length");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(3.into()))]),
            ..CopConfig::default()
        };
        // 4 body lines exceeds Max:3
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  c = 3\n  d = 4\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:3 on 4-line block");
        assert!(diags[0].message.contains("[4/3]"));
    }

    #[test]
    fn config_count_as_one_array() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountAsOne".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("array".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Body: a, b, [\n1,\n2\n] = 2 + 1 folded = 3 lines
        let source = b"items.each do |x|\n  a = 1\n  b = 2\n  arr = [\n    1,\n    2\n  ]\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire when array is folded (3/3)"
        );
    }

    #[test]
    fn allowed_methods_refine() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "AllowedMethods".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("refine".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // refine block with 4 lines should NOT fire because refine is allowed
        let source =
            b"refine String do\n  def a; end\n  def b; end\n  def c; end\n  def d; end\nend\n";
        let diags = run_cop_full_with_config(&BlockLength, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire on allowed method 'refine'"
        );
    }
}
