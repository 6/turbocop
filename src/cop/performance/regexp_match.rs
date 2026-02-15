use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RegexpMatch;

impl Cop for RegexpMatch {
    fn name(&self) -> &'static str {
        "Performance/RegexpMatch"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = ConditionVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct ConditionVisitor<'a, 'src> {
    cop: &'a RegexpMatch,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for ConditionVisitor<'_, '_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let body_range = body_range_of_if(node);
        check_condition(self.cop, self.source, &node.predicate(), body_range, &mut self.diagnostics);
        ruby_prism::visit_if_node(self, node);
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let body_range = node.statements().map(|s| {
            let loc = s.location();
            (loc.start_offset(), loc.end_offset())
        });
        check_condition(self.cop, self.source, &node.predicate(), body_range, &mut self.diagnostics);
        ruby_prism::visit_unless_node(self, node);
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let body_range = node.statements().map(|s| {
            let loc = s.location();
            (loc.start_offset(), loc.end_offset())
        });
        check_condition(self.cop, self.source, &node.predicate(), body_range, &mut self.diagnostics);
        ruby_prism::visit_while_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let body_range = node.statements().map(|s| {
            let loc = s.location();
            (loc.start_offset(), loc.end_offset())
        });
        check_condition(self.cop, self.source, &node.predicate(), body_range, &mut self.diagnostics);
        ruby_prism::visit_until_node(self, node);
    }
}

/// Get the body range for an if node (includes the main branch and else).
fn body_range_of_if(node: &ruby_prism::IfNode<'_>) -> Option<(usize, usize)> {
    // The body spans from after the predicate to before `end`
    let pred_end = node.predicate().location().end_offset();
    let node_end = node.location().end_offset();
    if node_end > pred_end {
        Some((pred_end, node_end))
    } else {
        None
    }
}

/// Check a condition expression for =~ usage.
/// `body_range` is the (start, end) byte offsets of the body following the condition,
/// used to check if MatchData ($~, $1, Regexp.last_match, etc.) is referenced.
fn check_condition(
    cop: &RegexpMatch,
    source: &SourceFile,
    cond: &ruby_prism::Node<'_>,
    body_range: Option<(usize, usize)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(call) = cond.as_call_node() {
        let method = call.name().as_slice();
        if method == b"=~" || method == b"!~" {
            // Don't flag if MatchData is used in the body (e.g., Regexp.last_match, $~, $1)
            if let Some((start, end)) = body_range {
                if last_match_used_in_range(source, start, end) {
                    return;
                }
            }
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                "Use `match?` instead of `=~` when `MatchData` is not used.".to_string(),
            ));
        }
    }

    // Also check through boolean operators (e.g., `x =~ /re/ && y`)
    if let Some(and_node) = cond.as_and_node() {
        check_condition(cop, source, &and_node.left(), body_range, diagnostics);
        check_condition(cop, source, &and_node.right(), body_range, diagnostics);
    }
    if let Some(or_node) = cond.as_or_node() {
        check_condition(cop, source, &or_node.left(), body_range, diagnostics);
        check_condition(cop, source, &or_node.right(), body_range, diagnostics);
    }
    if let Some(paren) = cond.as_parentheses_node() {
        if let Some(body) = paren.body() {
            check_condition(cop, source, &body, body_range, diagnostics);
        }
    }
    // Handle negation: `!x =~ /re/` — the `!` wraps a CallNode
    if let Some(call) = cond.as_call_node() {
        if call.name().as_slice() == b"!" {
            if let Some(recv) = call.receiver() {
                check_condition(cop, source, &recv, body_range, diagnostics);
            }
        }
    }
}

/// Check if the source in the given byte range contains references to
/// MatchData ($~, $1-$9, $&, $`, $', Regexp.last_match, etc.)
fn last_match_used_in_range(source: &SourceFile, start: usize, end: usize) -> bool {
    let bytes = source.as_bytes();
    let end = end.min(bytes.len());
    if start >= end {
        return false;
    }
    let body = &bytes[start..end];

    // Check for Regexp.last_match
    if body.windows(b"Regexp.last_match".len()).any(|w| w == b"Regexp.last_match") {
        return true;
    }

    // Check for $~ , $1-$9, $&, $`, $', $MATCH, $PREMATCH, $POSTMATCH, $LAST_PAREN_MATCH
    let mut i = 0;
    while i < body.len() {
        if body[i] == b'$' && i + 1 < body.len() {
            let next = body[i + 1];
            if next == b'~' || next == b'&' || next == b'`' || next == b'\''
                || next.is_ascii_digit()
            {
                return true;
            }
            // Check for $MATCH, $PREMATCH, etc. (uppercase letter after $)
            if next.is_ascii_uppercase() {
                return true;
            }
        }
        i += 1;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RegexpMatch, "cops/performance/regexp_match");
}
