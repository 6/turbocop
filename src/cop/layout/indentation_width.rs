use crate::cop::util::expected_indent_for_body;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct IndentationWidth;

impl IndentationWidth {
    /// Check body indentation.
    /// `keyword_offset` is used to determine which line the keyword is on (for same-line skip).
    /// `base_col` is the column that expected indentation is relative to.
    fn check_body_indentation(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
        base_col: usize,
        body: Option<ruby_prism::Node<'_>>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let body = match body {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let children: Vec<_> = stmts.body().iter().collect();
        if children.is_empty() {
            return Vec::new();
        }

        let (kw_line, _) = source.offset_to_line_col(keyword_offset);
        let expected = expected_indent_for_body(base_col, width);

        // Only check the first child's indentation. Sibling consistency is
        // handled by Layout/IndentationConsistency.
        let first = &children[0];
        let loc = first.location();
        let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

        // Skip if body is on same line as keyword (single-line construct)
        if child_line == kw_line {
            return Vec::new();
        }

        if child_col != expected {
            let actual_indent = child_col as isize - base_col as isize;
            return vec![self.diagnostic(
                source,
                child_line,
                child_col,
                format!(
                    "Use {} (not {}) spaces for indentation.",
                    width, actual_indent
                ),
            )];
        }

        Vec::new()
    }

    fn check_statements_indentation(
        &self,
        source: &SourceFile,
        keyword_offset: usize,
        base_col: usize,
        stmts: Option<ruby_prism::StatementsNode<'_>>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let stmts = match stmts {
            Some(s) => s,
            None => return Vec::new(),
        };

        let children: Vec<_> = stmts.body().iter().collect();
        if children.is_empty() {
            return Vec::new();
        }

        let (kw_line, _) = source.offset_to_line_col(keyword_offset);
        let expected = expected_indent_for_body(base_col, width);

        // Only check the first child's indentation. Sibling consistency is
        // handled by Layout/IndentationConsistency.
        let first = &children[0];
        let loc = first.location();
        let (child_line, child_col) = source.offset_to_line_col(loc.start_offset());

        // Skip if body is on same line as keyword (single-line construct)
        // or before the keyword (modifier if/while/until)
        if child_line <= kw_line {
            return Vec::new();
        }

        if child_col != expected {
            let actual_indent = child_col as isize - base_col as isize;
            return vec![self.diagnostic(
                source,
                child_line,
                child_col,
                format!(
                    "Use {} (not {}) spaces for indentation.",
                    width, actual_indent
                ),
            )];
        }

        Vec::new()
    }
}

/// Determine the base column for if/while/until body indentation.
///
/// Uses the `end` keyword column when available, which correctly handles:
/// - Variable-style assignment context (`x = if ... end` where end aligns with x)
/// - Keyword-style assignment context (end aligns with if keyword)
/// - Non-assignment contexts like `x << if ... end`
/// Falls back to keyword column when no end keyword exists (modifier if/while/until).
fn base_col_from_end(
    source: &SourceFile,
    kw_col: usize,
    end_offset: Option<usize>,
) -> usize {
    if let Some(end_off) = end_offset {
        source.offset_to_line_col(end_off).1
    } else {
        kw_col
    }
}

impl Cop for IndentationWidth {
    fn name(&self) -> &'static str {
        "Layout/IndentationWidth"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let width = config.get_usize("Width", 2);
        let align_style = config.get_str("EnforcedStyleAlignWith", "start_of_line");
        let allowed_patterns = config.get_string_array("AllowedPatterns").unwrap_or_default();

        // Skip if the node's source line matches any allowed pattern
        if !allowed_patterns.is_empty() {
            let (node_line, _) = source.offset_to_line_col(node.location().start_offset());
            if let Some(line_bytes) = source.lines().nth(node_line - 1) {
                if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                    for pattern in &allowed_patterns {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            if re.is_match(line_str) {
                                return Vec::new();
                            }
                        }
                    }
                }
            }
        }

        if let Some(class_node) = node.as_class_node() {
            let kw_offset = class_node.class_keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);
            return self.check_body_indentation(
                source,
                kw_offset,
                kw_col,
                class_node.body(),
                width,
            );
        }

        if let Some(module_node) = node.as_module_node() {
            let kw_offset = module_node.module_keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);
            return self.check_body_indentation(
                source,
                kw_offset,
                kw_col,
                module_node.body(),
                width,
            );
        }

        if let Some(def_node) = node.as_def_node() {
            let kw_offset = def_node.def_keyword_loc().start_offset();
            let base_col = if align_style == "keyword" {
                // EnforcedStyleAlignWith: keyword — indent relative to `def` keyword column
                source.offset_to_line_col(kw_offset).1
            } else {
                // EnforcedStyleAlignWith: start_of_line (default) — indent relative to the
                // start of the line, using `end` keyword column as proxy for line-start indent.
                if let Some(end_loc) = def_node.end_keyword_loc() {
                    source.offset_to_line_col(end_loc.start_offset()).1
                } else {
                    source.offset_to_line_col(kw_offset).1
                }
            };
            return self.check_body_indentation(
                source,
                kw_offset,
                base_col,
                def_node.body(),
                width,
            );
        }

        if let Some(if_node) = node.as_if_node() {
            if let Some(kw_loc) = if_node.if_keyword_loc() {
                let kw_offset = kw_loc.start_offset();
                let (_, kw_col) = source.offset_to_line_col(kw_offset);
                let base_col = base_col_from_end(
                    source, kw_col,
                    if_node.end_keyword_loc().map(|l| l.start_offset()),
                );
                return self.check_statements_indentation(
                    source,
                    kw_offset,
                    base_col,
                    if_node.statements(),
                    width,
                );
            }
        }

        // Handle block body indentation from CallNode (since BlockNode is
        // always a child of CallNode in Prism, and we need access to the
        // call's dot for chained method detection).
        if let Some(call_node) = node.as_call_node() {
            if let Some(block_ref) = call_node.block() {
                if let Some(block) = block_ref.as_block_node() {
                    let opening_offset = block.opening_loc().start_offset();
                    let closing_offset = block.closing_loc().start_offset();
                    let (_, closing_col) = source.offset_to_line_col(closing_offset);

                    // Skip if closing brace/end is not on its own line (inline
                    // block that wraps, e.g., `lambda { |req|\n  body }`).
                    let bytes = source.as_bytes();
                    let mut line_start = closing_offset;
                    while line_start > 0 && bytes[line_start - 1] != b'\n' {
                        line_start -= 1;
                    }
                    if !bytes[line_start..closing_offset].iter().all(|&b| b == b' ' || b == b'\t') {
                        return Vec::new();
                    }

                    // Skip if block parameters are on the same line as the
                    // first body statement (e.g., `reject { \n |x| body }`).
                    if let Some(params) = block.parameters() {
                        if let Some(body_node) = block.body() {
                            if let Some(stmts) = body_node.as_statements_node() {
                                if let Some(first) = stmts.body().iter().next() {
                                    let (params_line, _) = source.offset_to_line_col(params.location().end_offset());
                                    let (first_line, _) = source.offset_to_line_col(first.location().start_offset());
                                    if first_line == params_line {
                                        return Vec::new();
                                    }
                                }
                            }
                        }
                    }

                    // RuboCop's block_body_indentation_base: with the default
                    // `start_of_line` style, always use the `end` keyword column.
                    // Only with `relative_to_receiver` do we use the dot column
                    // when the dot is on a new line.
                    let base_col = if align_style == "relative_to_receiver" {
                        if let Some(dot_loc) = call_node.call_operator_loc() {
                            if let Some(recv) = call_node.receiver() {
                                let (recv_last_line, _) =
                                    source.offset_to_line_col(recv.location().end_offset());
                                let (dot_line, dot_col) =
                                    source.offset_to_line_col(dot_loc.start_offset());
                                if recv_last_line < dot_line {
                                    dot_col
                                } else {
                                    closing_col
                                }
                            } else {
                                closing_col
                            }
                        } else {
                            closing_col
                        }
                    } else {
                        closing_col
                    };
                    return self.check_body_indentation(
                        source,
                        opening_offset,
                        base_col,
                        block.body(),
                        width,
                    );
                }
            }
        }

        // Check body indentation inside when clauses (when keyword
        // positioning is handled by Layout/CaseIndentation, not here).
        if let Some(when_node) = node.as_when_node() {
            let kw_offset = when_node.keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);

            // Skip if body is on the same line as `then` keyword in a
            // multi-line when clause (e.g., `when :a,\n  :b then nil`).
            if let Some(then_loc) = when_node.then_keyword_loc() {
                let (then_line, _) = source.offset_to_line_col(then_loc.start_offset());
                if let Some(stmts) = when_node.statements() {
                    if let Some(first) = stmts.body().iter().next() {
                        let (first_line, _) = source.offset_to_line_col(first.location().start_offset());
                        if first_line == then_line {
                            return Vec::new();
                        }
                    }
                }
            }

            return self.check_statements_indentation(
                source,
                kw_offset,
                kw_col,
                when_node.statements(),
                width,
            );
        }

        if let Some(while_node) = node.as_while_node() {
            let kw_offset = while_node.keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);
            let base_col = base_col_from_end(
                source, kw_col,
                while_node.closing_loc().map(|l| l.start_offset()),
            );
            return self.check_statements_indentation(
                source,
                kw_offset,
                base_col,
                while_node.statements(),
                width,
            );
        }

        if let Some(until_node) = node.as_until_node() {
            let kw_offset = until_node.keyword_loc().start_offset();
            let (_, kw_col) = source.offset_to_line_col(kw_offset);
            let base_col = base_col_from_end(
                source, kw_col,
                until_node.closing_loc().map(|l| l.start_offset()),
            );
            return self.check_statements_indentation(
                source,
                kw_offset,
                base_col,
                until_node.statements(),
                width,
            );
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(IndentationWidth, "cops/layout/indentation_width");

    #[test]
    fn custom_width() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("Width".into(), serde_yml::Value::Number(4.into()))]),
            ..CopConfig::default()
        };
        // Body indented 2 instead of 4
        let source = b"def foo\n  x = 1\nend\n";
        let diags = run_cop_full_with_config(&IndentationWidth, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Use 4 (not 2) spaces"));
    }

    #[test]
    fn enforced_style_keyword_aligns_to_def() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyleAlignWith".into(), serde_yml::Value::String("keyword".into())),
            ]),
            ..CopConfig::default()
        };
        // Body indented 2 from column 0, but `def` is at column 8 (after `private `)
        // With keyword style, body should be at column 10 (8 + 2)
        let source = b"private def foo\n  bar\nend\n";
        let diags = run_cop_full_with_config(&IndentationWidth, source, config);
        assert_eq!(diags.len(), 1, "keyword style should flag body not aligned with def keyword");
        assert!(diags[0].message.contains("Use 2"));
    }

    #[test]
    fn allowed_patterns_skips_matching() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("AllowedPatterns".into(), serde_yml::Value::Sequence(vec![
                    serde_yml::Value::String("^\\s*module".into()),
                ])),
            ]),
            ..CopConfig::default()
        };
        // Module with wrong indentation but matches AllowedPatterns
        let source = b"module Foo\n      x = 1\nend\n";
        let diags = run_cop_full_with_config(&IndentationWidth, source, config);
        assert!(diags.is_empty(), "AllowedPatterns should skip matching lines");
    }

    #[test]
    fn assignment_context_if() {
        use crate::testutil::run_cop_full;
        // Body indented from LHS (column 0), not from `if` (column 4)
        let source = b"x = if foo\n  bar\nend\n";
        let diags = run_cop_full(&IndentationWidth, source);
        assert!(diags.is_empty(), "assignment context if should not flag: {:?}", diags);
    }

    #[test]
    fn assignment_context_if_wrong_indent() {
        use crate::testutil::run_cop_full;
        // Body at column 6 — should be column 2 (LHS 0 + 2)
        let source = b"x = if foo\n      bar\nend\n";
        let diags = run_cop_full(&IndentationWidth, source);
        assert_eq!(diags.len(), 1, "should flag wrong indentation in assignment context");
    }

    #[test]
    fn assignment_context_compound_operator() {
        use crate::testutil::run_cop_full;
        // x ||= if foo ... body indented from column 0, end at col 0 (variable style)
        let source = b"x ||= if foo\n  bar\nend\n";
        let diags = run_cop_full(&IndentationWidth, source);
        assert!(diags.is_empty(), "compound assignment context should work: {:?}", diags);
    }

    #[test]
    fn assignment_context_keyword_style() {
        use crate::testutil::run_cop_full;
        // Keyword style: end aligned with `if`, body indented from `if`
        // @links = if enabled?
        //            body
        //          end
        let source = b"    @links = if enabled?\n               body\n             end\n";
        let diags = run_cop_full(&IndentationWidth, source);
        assert!(diags.is_empty(), "keyword style assignment should not flag: {:?}", diags);
    }
}
