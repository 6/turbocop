use crate::cop::shared::util::line_at;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks whether class/module/method definitions are separated by one or more empty lines.
///
/// ## Corpus investigation findings (44 FP, 323 FN):
///
/// **Root cause:** The original implementation used text-based backward scanning from each
/// def node to find the "previous definition boundary." This approach was fragile and caused:
///
/// **FN causes (323):**
/// - Endless methods (`def foo() = x`) were treated as scope openers by `is_opening_line`,
///   so the next def after an endless method never fired.
/// - Cross-type boundaries (class after def, module after class, etc.) were not always
///   detected because the backward scan only looked for `end` keywords and single-line defs.
/// - `AllowAdjacentOneLineDefs` was checked per-node rather than per-pair, so a multi-line
///   def following adjacent one-liners was missed.
///
/// **FP causes (44):**
/// - `end` keywords from non-definition constructs (rescue blocks, case/when in unusual
///   indentation) were sometimes misidentified as definition ends by `is_definition_end`.
/// - The conservative fallback (`true` when no opener found) caused FPs on malformed code.
///
/// **Fix:** Rewrote to use AST-based sibling detection matching RuboCop's `on_begin` approach.
/// Walks the AST to find `StatementsNode` containers, then checks consecutive pairs of
/// candidate children (def/class/module/macro). This eliminates text-based heuristics
/// entirely for boundary detection.
pub struct EmptyLineBetweenDefs;

fn is_blank(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

/// Check if a line is a comment line.
fn is_comment_line(line: &[u8]) -> bool {
    let trimmed: Vec<u8> = line
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect();
    trimmed.starts_with(b"#")
}

/// Information about a candidate definition node
struct CandidateInfo {
    /// 1-indexed start line of the definition keyword
    start_line: usize,
    /// Column of the definition keyword
    start_col: usize,
    /// 1-indexed end line of the node (last line of the whole node)
    end_line: usize,
    /// Whether this is a single-line definition
    is_single_line: bool,
    /// Type of definition for message formatting
    def_type: DefType,
}

#[derive(Clone, Copy)]
enum DefType {
    Method,
    Class,
    Module,
    Block,
    Send,
}

impl DefType {
    fn label(self) -> &'static str {
        match self {
            DefType::Method => "method",
            DefType::Class => "class",
            DefType::Module => "module",
            DefType::Block => "block",
            DefType::Send => "send",
        }
    }
}

/// Try to extract candidate info from a node.
fn candidate_info<'pr>(
    source: &SourceFile,
    node: &ruby_prism::Node<'pr>,
    empty_between_methods: bool,
    empty_between_classes: bool,
    empty_between_modules: bool,
    def_like_macros: &[String],
) -> Option<CandidateInfo> {
    if let Some(def_node) = node.as_def_node() {
        if !empty_between_methods {
            return None;
        }
        let loc = def_node.def_keyword_loc();
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        let end_offset = def_node
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(def_node.location().start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);
        return Some(CandidateInfo {
            start_line,
            start_col,
            end_line,
            is_single_line: start_line == end_line,
            def_type: DefType::Method,
        });
    }

    if let Some(class_node) = node.as_class_node() {
        if !empty_between_classes {
            return None;
        }
        let loc = class_node.class_keyword_loc();
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        let end_offset = class_node
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(class_node.location().start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);
        return Some(CandidateInfo {
            start_line,
            start_col,
            end_line,
            is_single_line: start_line == end_line,
            def_type: DefType::Class,
        });
    }

    if let Some(module_node) = node.as_module_node() {
        if !empty_between_modules {
            return None;
        }
        let loc = module_node.module_keyword_loc();
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        let end_offset = module_node
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(module_node.location().start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);
        return Some(CandidateInfo {
            start_line,
            start_col,
            end_line,
            is_single_line: start_line == end_line,
            def_type: DefType::Module,
        });
    }

    // Check for macro call (with or without block)
    // In Prism, `foo 'bar' do ... end` is a CallNode at the statement level
    // with a .block() child, not a standalone BlockNode.
    if let Some(call_node) = node.as_call_node() {
        if def_like_macros.is_empty() || call_node.receiver().is_some() {
            return None;
        }
        let name = std::str::from_utf8(call_node.name().as_slice()).unwrap_or("");
        if !def_like_macros.iter().any(|m| m == name) {
            return None;
        }

        // Determine if this is a block macro or a bare send macro
        let has_block = call_node.block().and_then(|b| b.as_block_node()).is_some();
        let def_type = if has_block {
            DefType::Block
        } else {
            DefType::Send
        };

        let loc = call_node.location();
        let (start_line, start_col) = source.offset_to_line_col(loc.start_offset());
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);
        return Some(CandidateInfo {
            start_line,
            start_col,
            end_line,
            is_single_line: start_line == end_line,
            def_type,
        });
    }

    None
}

/// RuboCop's `multiple_blank_lines_groups?` check.
/// Returns true when blank lines are interspersed with non-blank lines between the two defs,
/// indicating structured separation (e.g., section headers with blank lines on both sides).
fn multiple_blank_lines_groups(
    source: &SourceFile,
    prev_end_line: usize,
    next_start_line: usize,
) -> bool {
    let mut last_blank_idx: Option<usize> = None;
    let mut first_non_blank_idx: Option<usize> = None;
    for (i, line_num) in (prev_end_line + 1..next_start_line).enumerate() {
        let line = match line_at(source, line_num) {
            Some(l) => l,
            None => continue,
        };
        if is_blank(line) {
            last_blank_idx = Some(i);
        } else if first_non_blank_idx.is_none() {
            first_non_blank_idx = Some(i);
        }
    }
    if let (Some(last_blank), Some(first_non_blank)) = (last_blank_idx, first_non_blank_idx) {
        last_blank > first_non_blank
    } else {
        false
    }
}

/// Count blank lines in the gap, considering comments as transparent.
/// RuboCop counts blank lines between `end_loc(prev)` line and `def_start(next)` line.
/// Comments between defs don't prevent the offense but blank lines are still counted.
fn count_blank_lines_for_pair(
    source: &SourceFile,
    prev_end_line: usize,
    next_start_line: usize,
) -> usize {
    let mut count = 0;
    for line_num in (prev_end_line + 1)..next_start_line {
        if let Some(line) = line_at(source, line_num) {
            if is_blank(line) {
                count += 1;
            }
        }
    }
    count
}

struct EmptyLineBetweenDefsVisitor<'a> {
    cop: &'a EmptyLineBetweenDefs,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    corrections: Vec<crate::correction::Correction>,
    collecting_corrections: bool,
    empty_between_methods: bool,
    empty_between_classes: bool,
    empty_between_modules: bool,
    def_like_macros: Vec<String>,
    number_of_empty_lines: usize,
    allow_adjacent: bool,
}

impl EmptyLineBetweenDefsVisitor<'_> {
    fn check_statements(&mut self, children: &[ruby_prism::Node<'_>]) {
        // Iterate over consecutive pairs of children, collecting candidates
        let mut prev_candidate: Option<(CandidateInfo, usize)> = None;

        for (i, child) in children.iter().enumerate() {
            let info = candidate_info(
                self.source,
                child,
                self.empty_between_methods,
                self.empty_between_classes,
                self.empty_between_modules,
                &self.def_like_macros,
            );

            if let Some(curr_info) = info {
                if let Some((ref prev_info, _prev_idx)) = prev_candidate {
                    self.check_pair(prev_info, &curr_info);
                }
                prev_candidate = Some((curr_info, i));
            } else {
                // Non-candidate node breaks the chain
                prev_candidate = None;
            }
        }
    }

    fn check_pair(&mut self, prev: &CandidateInfo, curr: &CandidateInfo) {
        let blank_count = count_blank_lines_for_pair(self.source, prev.end_line, curr.start_line);

        // Check if the blank count is within the allowed range
        if blank_count == self.number_of_empty_lines {
            return;
        }

        // Check for multiple blank line groups (structured separation)
        if multiple_blank_lines_groups(self.source, prev.end_line, curr.start_line) {
            return;
        }

        // AllowAdjacentOneLineDefs: skip if BOTH defs are single-line
        if self.allow_adjacent && prev.is_single_line && curr.is_single_line {
            return;
        }

        let type_label = curr.def_type.label();
        let msg = if blank_count > self.number_of_empty_lines {
            format!(
                "Expected {} empty line between {} definitions; found {}.",
                self.number_of_empty_lines, type_label, blank_count
            )
        } else if self.number_of_empty_lines == 1 {
            format!("Use empty lines between {type_label} definitions.")
        } else {
            format!(
                "Use {} empty lines between {} definitions.",
                self.number_of_empty_lines, type_label
            )
        };

        let mut diag = self
            .cop
            .diagnostic(self.source, curr.start_line, curr.start_col, msg);

        if self.collecting_corrections && blank_count < self.number_of_empty_lines {
            // Insert missing blank lines before the def.
            // RuboCop inserts after the end of the previous def's last line.
            // We insert blank lines right before the current def's line.
            // But first, find the right insertion point: right after prev's end line.
            // We need to find where comments start (if any) between the two defs
            // and insert the blank line there.
            let insert_line = self.find_correction_insert_line(prev.end_line, curr.start_line);
            let lines_to_add = self.number_of_empty_lines - blank_count;
            if let Some(offset) = self.source.line_col_to_offset(insert_line, 0) {
                self.corrections.push(crate::correction::Correction {
                    start: offset,
                    end: offset,
                    replacement: "\n".repeat(lines_to_add),
                    cop_name: self.cop.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
        }
        // TODO: autocorrect for excess blank lines (remove extra lines)

        self.diagnostics.push(diag);
    }

    /// Find the line number where to insert blank lines for autocorrect.
    /// RuboCop inserts after the newline at the end of the previous def.
    /// If there are comments between defs, insert before the comment block.
    fn find_correction_insert_line(&self, prev_end_line: usize, next_start_line: usize) -> usize {
        // Scan from prev_end_line+1 forward, looking for the first comment
        // that's part of a continuous comment block leading to the next def.
        // The blank line should go before that comment block.
        let mut first_comment_line = None;
        for line_num in (prev_end_line + 1)..next_start_line {
            if let Some(line) = line_at(self.source, line_num) {
                if is_comment_line(line) {
                    if first_comment_line.is_none() {
                        first_comment_line = Some(line_num);
                    }
                } else if is_blank(line) {
                    // Blank line resets the comment block tracking
                    // (a blank before comments means the insertion point
                    // should be earlier)
                    continue;
                } else {
                    // Non-blank non-comment line - shouldn't happen between
                    // two consecutive candidates
                    first_comment_line = None;
                }
            }
        }

        // If there are comments leading into the next def, insert before them
        if let Some(comment_start) = first_comment_line {
            comment_start
        } else {
            // Insert right before the next def line
            next_start_line
        }
    }
}

impl<'pr> Visit<'pr> for EmptyLineBetweenDefsVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let children: Vec<_> = node.body().iter().collect();
        self.check_statements(&children);

        // Continue visiting child nodes to find nested StatementsNodes
        ruby_prism::visit_statements_node(self, node);
    }
}

impl Cop for EmptyLineBetweenDefs {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineBetweenDefs"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let empty_between_methods = config.get_bool("EmptyLineBetweenMethodDefs", true);
        let empty_between_classes = config.get_bool("EmptyLineBetweenClassDefs", true);
        let empty_between_modules = config.get_bool("EmptyLineBetweenModuleDefs", true);
        let def_like_macros = config.get_string_array("DefLikeMacros").unwrap_or_default();
        let number_of_empty_lines = config.get_usize("NumberOfEmptyLines", 1);
        let allow_adjacent = config.get_bool("AllowAdjacentOneLineDefs", true);

        let collecting_corrections = corrections.is_some();

        let mut visitor = EmptyLineBetweenDefsVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            corrections: Vec::new(),
            collecting_corrections,
            empty_between_methods,
            empty_between_classes,
            empty_between_modules,
            def_like_macros,
            number_of_empty_lines,
            allow_adjacent,
        };

        visitor.visit(&parse_result.node());

        diagnostics.extend(visitor.diagnostics);
        if let Some(corr) = corrections {
            corr.extend(visitor.corrections);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(EmptyLineBetweenDefs, "cops/layout/empty_line_between_defs");
    crate::cop_autocorrect_fixture_tests!(
        EmptyLineBetweenDefs,
        "cops/layout/empty_line_between_defs"
    );

    #[test]
    fn single_def_no_offense() {
        let src = b"class Foo\n  def bar\n    1\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert!(diags.is_empty(), "Single def should not trigger offense");
    }

    #[test]
    fn def_after_end_without_blank_line() {
        let src = b"class Foo\n  def bar\n    1\n  end\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line between defs should trigger"
        );
        assert_eq!(diags[0].location.line, 5);
    }

    #[test]
    fn number_of_empty_lines_requires_multiple() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "NumberOfEmptyLines".into(),
                serde_yml::Value::Number(2.into()),
            )]),
            ..CopConfig::default()
        };
        // One blank line between defs should be flagged when 2 required
        let src = b"class Foo\n  def bar\n    1\n  end\n\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config.clone());
        assert_eq!(
            diags.len(),
            1,
            "Should flag when fewer than NumberOfEmptyLines blank lines"
        );

        // Two blank lines should be accepted
        let src2 = b"class Foo\n  def bar\n    1\n  end\n\n\n  def baz\n    2\n  end\nend\n";
        let diags2 = run_cop_full_with_config(&EmptyLineBetweenDefs, src2, config);
        assert!(
            diags2.is_empty(),
            "Should accept when NumberOfEmptyLines blank lines present"
        );
    }

    #[test]
    fn def_like_macros_flags_missing_blank_line() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // With AllowAdjacentOneLineDefs: false, single-line macros should be flagged
        let config = CopConfig {
            options: HashMap::from([
                (
                    "DefLikeMacros".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("scope".into())]),
                ),
                (
                    "AllowAdjacentOneLineDefs".into(),
                    serde_yml::Value::Bool(false),
                ),
            ]),
            ..CopConfig::default()
        };
        // Two scope macros without blank line
        let src = b"class Foo\n  scope :active, -> { where(active: true) }\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config.clone());
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line between def-like macros should trigger"
        );

        // With blank line — no offense
        let src2 = b"class Foo\n  scope :active, -> { where(active: true) }\n\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags2 = run_cop_full_with_config(&EmptyLineBetweenDefs, src2, config);
        assert!(
            diags2.is_empty(),
            "Blank line between def-like macros should be accepted"
        );
    }

    #[test]
    fn def_like_macros_single_line_allowed_by_default() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // With AllowAdjacentOneLineDefs: true (default), single-line macros are NOT flagged
        let config = CopConfig {
            options: HashMap::from([(
                "DefLikeMacros".into(),
                serde_yml::Value::Sequence(vec![serde_yml::Value::String("scope".into())]),
            )]),
            ..CopConfig::default()
        };
        let src = b"class Foo\n  scope :active, -> { where(active: true) }\n  scope :recent, -> { where(recent: true) }\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config);
        assert!(
            diags.is_empty(),
            "Single-line macros should be allowed when AllowAdjacentOneLineDefs is true"
        );
    }

    #[test]
    fn empty_between_method_defs_false_skips_methods() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EmptyLineBetweenMethodDefs".into(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let src = b"class Foo\n  def bar\n    1\n  end\n  def baz\n    2\n  end\nend\n";
        let diags = run_cop_full_with_config(&EmptyLineBetweenDefs, src, config);
        assert!(
            diags.is_empty(),
            "Should not flag when EmptyLineBetweenMethodDefs is false"
        );
    }

    #[test]
    fn endless_method_followed_by_regular_method() {
        let src = b"def compute() = x + y\ndef process\n  z\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line after endless method should trigger"
        );
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn class_after_class_no_blank_line() {
        let src = b"class Alpha\nend\nclass Bravo\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line between classes should trigger"
        );
        assert_eq!(diags[0].location.line, 3);
    }

    #[test]
    fn def_after_class_no_blank_line() {
        let src = b"class Epsilon\nend\ndef zeta\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert_eq!(
            diags.len(),
            1,
            "Missing blank line between class and def should trigger"
        );
    }

    #[test]
    fn defs_inside_conditional_no_offense() {
        let src =
            b"if condition\n  def foo\n    true\n  end\nelse\n  def foo\n    false\n  end\nend\n";
        let diags = run_cop_full(&EmptyLineBetweenDefs, src);
        assert!(
            diags.is_empty(),
            "Defs in different branches should not trigger"
        );
    }
}
