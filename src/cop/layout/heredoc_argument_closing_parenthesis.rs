use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Layout/HeredocArgumentClosingParenthesis
///
/// Investigation findings (2026-03-11):
/// Root cause of 1019 FPs: the original implementation was too simplistic compared
/// to RuboCop's complex algorithm. Key missing checks:
///
/// 1. **Non-heredoc args after heredoc body**: When there are non-heredoc arguments
///    between the heredoc body end and the closing paren (e.g., `foo(<<~SQL, opt: true\n)\n`),
///    the closing paren correctly goes with those trailing args, not the heredoc opener.
///    RuboCop's `exist_argument_between_heredoc_end_and_closing_parentheses?` handles this.
///    Fix: scan bytes between the last heredoc body end and the closing paren for
///    non-whitespace content; if found, skip.
///
/// 2. **`end` keyword ancestors**: Calls wrapped in `do..end`, `if/unless/while` blocks
///    should not fire because the `end` keyword sits before the closing paren.
///    RuboCop's `end_keyword_before_closing_parenthesis?` handles this.
///    Fix: walk the source bytes between the call's opening paren and closing paren to
///    detect `end` keywords, or more practically, check if the closing paren is on the
///    same line as an `end` keyword by checking `end)` pattern.
///
/// 3. **Heredoc with method chain on receiver** (e.g., `<<-SQL.tr(...)`): The heredoc
///    is a receiver of a method call, not a direct string argument.
///    Fix: also check for heredocs as receivers of call arguments.
///
/// 4. **Removed spurious `STRING_NODE`/`INTERPOLATED_STRING_NODE` from interested types**
///    since they always returned early at the `as_call_node()` check.
///
/// Investigation findings (2026-03-13):
/// Root cause of remaining 1,689 FPs: the `end` keyword ancestor check was far too
/// narrow — it only checked for `end` on the same line immediately before the closing
/// paren. RuboCop's `end_keyword_before_closing_parenthesis?` actually checks if ANY
/// ancestor of the send node has an `end` keyword via:
///   `parenthesized_send_node.ancestors.any? { |a| a.loc_is?(:end, 'end') }`
/// This suppresses the cop for any call inside a `def`, `class`, `module`, `do..end`
/// block, `if/unless/while/until/for/case/begin` statement, etc. In practice, the cop
/// only fires at the top level or inside brace-delimited constructs (lambdas, procs).
///
/// Fix: switched from `check_node` to `check_source` with a custom visitor that tracks
/// `end_depth` — how many `end`-bearing ancestor constructs we're inside. The cop only
/// fires when `end_depth == 0`. Node types that increment depth: DefNode, ClassNode,
/// ModuleNode, SingletonClassNode, BlockNode (do..end only, not braces), BeginNode,
/// IfNode/UnlessNode/WhileNode/UntilNode (statement forms with end_keyword), ForNode,
/// CaseNode, CaseMatchNode.
pub struct HeredocArgumentClosingParenthesis;

impl Cop for HeredocArgumentClosingParenthesis {
    fn name(&self) -> &'static str {
        "Layout/HeredocArgumentClosingParenthesis"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = HeredocParenVisitor {
            source,
            cop: self,
            end_depth: 0,
            end_stack: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HeredocParenVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a HeredocArgumentClosingParenthesis,
    end_depth: usize,
    /// Stack of booleans: true if the corresponding branch node had an `end` keyword.
    end_stack: Vec<bool>,
    diagnostics: Vec<Diagnostic>,
}

impl HeredocParenVisitor<'_> {
    /// Check if a branch node is an `end`-bearing construct.
    fn has_end_keyword(&self, node: &ruby_prism::Node<'_>) -> bool {
        let bytes = self.source.as_bytes();

        // DefNode, ClassNode, ModuleNode, SingletonClassNode always have `end`
        if node.as_def_node().is_some()
            || node.as_class_node().is_some()
            || node.as_module_node().is_some()
            || node.as_singleton_class_node().is_some()
        {
            return true;
        }

        // BlockNode: only do..end (not brace blocks)
        if let Some(block) = node.as_block_node() {
            return block.closing_loc().as_slice() == b"end";
        }

        // BeginNode (explicit begin..end)
        if node.as_begin_node().is_some() {
            return true;
        }

        // IfNode, UnlessNode — only statement form (has end_keyword_loc)
        if let Some(if_node) = node.as_if_node() {
            // Statement if has end_keyword_loc; modifier if does not
            if if_node.end_keyword_loc().is_some() {
                return true;
            }
        }
        if let Some(unless_node) = node.as_unless_node() {
            if unless_node.end_keyword_loc().is_some() {
                return true;
            }
        }

        // WhileNode, UntilNode — only statement form
        if let Some(while_node) = node.as_while_node() {
            // Statement while has closing_loc with "end"
            if let Some(closing) = while_node.closing_loc() {
                if closing.as_slice() == b"end" {
                    return true;
                }
            }
        }
        if let Some(until_node) = node.as_until_node() {
            if let Some(closing) = until_node.closing_loc() {
                if closing.as_slice() == b"end" {
                    return true;
                }
            }
        }

        // ForNode
        if node.as_for_node().is_some() {
            return true;
        }

        // CaseNode, CaseMatchNode
        if node.as_case_node().is_some() || node.as_case_match_node().is_some() {
            return true;
        }

        // Lambda body with do..end (LambdaNode closing is `end` or `}`)
        if let Some(lambda) = node.as_lambda_node() {
            let closing = lambda.closing_loc();
            let slice = &bytes[closing.start_offset()..closing.end_offset()];
            if slice == b"end" {
                return true;
            }
        }

        false
    }

    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must have parenthesized call
        let open_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let close_loc = match call.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        if open_loc.as_slice() != b"(" || close_loc.as_slice() != b")" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let bytes = self.source.as_bytes();

        // Collect heredoc info
        let mut has_heredoc = false;
        let mut max_heredoc_body_end: usize = 0;
        let mut last_heredoc_opener_line: usize = 0;

        for arg in args.arguments().iter() {
            if let Some((opener_offset, body_end)) = heredoc_info(bytes, &arg) {
                has_heredoc = true;
                if body_end > max_heredoc_body_end {
                    max_heredoc_body_end = body_end;
                    let (line, _) = self.source.offset_to_line_col(opener_offset);
                    last_heredoc_opener_line = line;
                }
            }
        }

        if !has_heredoc {
            return;
        }

        let (close_line, close_col) = self.source.offset_to_line_col(close_loc.start_offset());

        // If the closing paren is on the same line as the last heredoc opener, it's correct
        if close_line == last_heredoc_opener_line {
            return;
        }

        // Check if there's non-whitespace content between the last heredoc body end
        // and the closing paren. If so, there are non-heredoc arguments after the
        // heredoc body and the closing paren correctly goes with those args.
        if max_heredoc_body_end > 0 && max_heredoc_body_end < close_loc.start_offset() {
            let between = &bytes[max_heredoc_body_end..close_loc.start_offset()];
            let has_content = between.iter().any(|&b| !b.is_ascii_whitespace());
            if has_content {
                return;
            }
        }

        // Check if the closing paren is preceded by `end` on the same line.
        // This handles `foo(bar do ... end)` and `foo(unless cond ... end)`.
        if has_end_keyword_before_close_paren(bytes, close_loc.start_offset()) {
            return;
        }

        // RuboCop's end_keyword_before_closing_parenthesis? checks ALL ancestors.
        // If any ancestor has an `end` keyword, suppress.
        if self.end_depth > 0 {
            return;
        }

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            close_line,
            close_col,
            "Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.".to_string(),
        ));
    }
}

impl Visit<'_> for HeredocParenVisitor<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'_>) {
        let has_end = self.has_end_keyword(&node);
        self.end_stack.push(has_end);
        if has_end {
            self.end_depth += 1;
        }
    }

    fn visit_branch_node_leave(&mut self) {
        if let Some(had_end) = self.end_stack.pop() {
            if had_end {
                self.end_depth -= 1;
            }
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        self.check_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

/// Returns (opener_start_offset, body_end_offset) for a heredoc argument.
/// The opener_start_offset is the offset of `<<~SQL` etc.
/// The body_end_offset is the offset after the closing delimiter line.
fn heredoc_info(bytes: &[u8], node: &ruby_prism::Node<'_>) -> Option<(usize, usize)> {
    // Direct heredoc: InterpolatedStringNode or StringNode with `<<` opening
    if let Some(info) = direct_heredoc_info(bytes, node) {
        return Some(info);
    }

    // Heredoc as receiver of a method call on the same arg
    // e.g., `<<-SQL.tr("z", "t")` — the arg is a CallNode whose receiver is the heredoc
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if let Some(info) = direct_heredoc_info(bytes, &recv) {
                return Some(info);
            }
        }
    }

    // Hash argument containing a heredoc value (e.g., `foo: <<-SQL`)
    if let Some(kw_hash) = node.as_keyword_hash_node() {
        for elem in kw_hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if let Some(info) = direct_heredoc_info(bytes, &assoc.value()) {
                    return Some(info);
                }
                // Also check if value is a call on a heredoc
                if let Some(call) = assoc.value().as_call_node() {
                    if let Some(recv) = call.receiver() {
                        if let Some(info) = direct_heredoc_info(bytes, &recv) {
                            return Some(info);
                        }
                    }
                }
            }
        }
    }
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if let Some(info) = direct_heredoc_info(bytes, &assoc.value()) {
                    return Some(info);
                }
            }
        }
    }

    None
}

/// Check if a node is directly a heredoc (StringNode or InterpolatedStringNode with `<<` opening).
/// Returns (opener_start_offset, body_end_offset).
fn direct_heredoc_info(bytes: &[u8], node: &ruby_prism::Node<'_>) -> Option<(usize, usize)> {
    if let Some(istr) = node.as_interpolated_string_node() {
        if let Some(opening) = istr.opening_loc() {
            let slice = &bytes[opening.start_offset()..opening.end_offset()];
            if slice.starts_with(b"<<") {
                let body_end = istr
                    .closing_loc()
                    .map(|c| c.end_offset())
                    .unwrap_or(istr.location().end_offset());
                return Some((opening.start_offset(), body_end));
            }
        }
    }
    if let Some(str_node) = node.as_string_node() {
        if let Some(opening) = str_node.opening_loc() {
            let slice = &bytes[opening.start_offset()..opening.end_offset()];
            if slice.starts_with(b"<<") {
                let body_end = str_node
                    .closing_loc()
                    .map(|c| c.end_offset())
                    .unwrap_or(str_node.location().end_offset());
                return Some((opening.start_offset(), body_end));
            }
        }
    }
    None
}

/// Check if there's an `end` keyword immediately before the closing paren on the same line.
/// Scans backwards from the `)` looking for `end` preceded by whitespace or line start.
fn has_end_keyword_before_close_paren(bytes: &[u8], close_paren_offset: usize) -> bool {
    // Scan backwards from the close paren, skipping whitespace (not newlines)
    let mut pos = close_paren_offset;
    while pos > 0 {
        pos -= 1;
        if bytes[pos] == b'\n' {
            // Reached a newline — no `end` on this line before the paren
            return false;
        }
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            break;
        }
    }
    // Check if the non-whitespace content ends with `end`
    if pos >= 2 {
        let end_candidate = &bytes[pos - 2..=pos];
        if end_candidate == b"end" {
            // Verify it's a word boundary (preceded by whitespace or line start)
            if pos < 3 {
                return true;
            }
            let before = bytes[pos - 3];
            return before == b' ' || before == b'\t' || before == b'\n' || before == b'\r';
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        HeredocArgumentClosingParenthesis,
        "cops/layout/heredoc_argument_closing_parenthesis"
    );
}
