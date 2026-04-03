//! Shared infrastructure for the four `MultilineBraceLayout` cops.
//!
//! Mirrors RuboCop's `MultilineLiteralBraceLayout` mixin. All four cops
//! enforce the same symmetrical / new_line / same_line style on the closing
//! brace relative to the opening brace and the first/last element. The only
//! differences are:
//! - which AST node supplies the opening/closing locations and elements,
//! - the noun used in diagnostic messages ("array", "hash", etc.), and
//! - how heredocs are detected (simple recursive walk vs. visitor-based).

use crate::cop::Cop;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Describes the kind of brace construct, used to generate cop-specific
/// diagnostic messages that exactly match RuboCop's wording.
pub struct BraceKind {
    /// Phrase for the closing brace, e.g. "The closing array brace" or
    /// "Closing hash brace".
    pub closing_phrase: &'static str,
    /// Phrase for the opening brace, e.g. "the opening brace" or
    /// "opening brace".
    pub opening_phrase: &'static str,
    /// Noun for the elements, e.g. "array element" or "argument".
    pub element_noun: &'static str,
}

pub const ARRAY_BRACE: BraceKind = BraceKind {
    closing_phrase: "The closing array brace",
    opening_phrase: "the opening brace",
    element_noun: "array element",
};

pub const HASH_BRACE: BraceKind = BraceKind {
    closing_phrase: "Closing hash brace",
    opening_phrase: "opening brace",
    element_noun: "hash element",
};

pub const METHOD_CALL_BRACE: BraceKind = BraceKind {
    closing_phrase: "Closing method call brace",
    opening_phrase: "opening brace",
    element_noun: "argument",
};

pub const METHOD_DEFINITION_BRACE: BraceKind = BraceKind {
    closing_phrase: "Closing method definition brace",
    opening_phrase: "opening brace",
    element_noun: "parameter",
};

/// The computed positions extracted by each cop before calling the shared
/// style check.
pub struct BracePositions {
    pub open_line: usize,
    pub close_line: usize,
    pub close_col: usize,
    pub first_elem_line: usize,
    pub last_elem_line: usize,
}

/// Shared style enforcement for all four multiline brace layout cops.
///
/// Returns early (no diagnostic) when the construct is single-line.
pub fn check_brace_layout(
    cop: &dyn Cop,
    source: &SourceFile,
    enforced_style: &str,
    kind: &BraceKind,
    pos: &BracePositions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Only check multiline constructs
    if pos.open_line == pos.close_line {
        return;
    }

    let open_same_as_first = pos.open_line == pos.first_elem_line;
    let close_same_as_last = pos.close_line == pos.last_elem_line;

    let closing = kind.closing_phrase;
    let element = kind.element_noun;
    let opening = kind.opening_phrase;

    match enforced_style {
        "symmetrical" => {
            if open_same_as_first && !close_same_as_last {
                diagnostics.push(cop.diagnostic(
                    source,
                    pos.close_line,
                    pos.close_col,
                    format!(
                        "{closing} must be on the same line as the last {element} \
                         when {opening} is on the same line as the first {element}."
                    ),
                ));
            }
            if !open_same_as_first && close_same_as_last {
                diagnostics.push(cop.diagnostic(
                    source,
                    pos.close_line,
                    pos.close_col,
                    format!(
                        "{closing} must be on the line after the last {element} \
                         when {opening} is on a separate line from the first {element}."
                    ),
                ));
            }
        }
        "new_line" => {
            if close_same_as_last {
                diagnostics.push(cop.diagnostic(
                    source,
                    pos.close_line,
                    pos.close_col,
                    format!("{closing} must be on the line after the last {element}."),
                ));
            }
        }
        "same_line" => {
            if !close_same_as_last {
                diagnostics.push(cop.diagnostic(
                    source,
                    pos.close_line,
                    pos.close_col,
                    format!("{closing} must be on the same line as the last {element}."),
                ));
            }
        }
        _ => {}
    }
}

// ── Heredoc helpers ───────────────────────────────────────────────────

/// Simple recursive heredoc check used by array and hash cops.
///
/// Returns `true` if *node* (or a nested call receiver/argument/assoc value)
/// is a heredoc string (opening starts with `<<`).
pub fn contains_heredoc(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_interpolated_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    if let Some(s) = node.as_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if contains_heredoc(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if contains_heredoc(&arg) {
                    return true;
                }
            }
        }
    }
    if let Some(assoc) = node.as_assoc_node() {
        return contains_heredoc(&assoc.value());
    }
    false
}

/// Visitor-based heredoc check used by the method-call cop.
///
/// Returns `true` when the node contains a heredoc whose closing delimiter
/// ends on or after the last line of the node itself (i.e. the heredoc body
/// forces the closing brace placement).
pub fn last_line_heredoc(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    use ruby_prism::Visit;

    struct LastLineHeredocDetector<'a> {
        source: &'a SourceFile,
        parent_last_line: usize,
        found: bool,
    }

    impl LastLineHeredocDetector<'_> {
        fn visit_heredoc<'pr>(
            &mut self,
            opening: Option<ruby_prism::Location<'pr>>,
            closing: Option<ruby_prism::Location<'pr>>,
        ) {
            let Some(opening) = opening else {
                return;
            };
            if self.found || !opening.as_slice().starts_with(b"<<") {
                return;
            }
            let Some(closing) = closing else {
                return;
            };

            let end_off = closing
                .end_offset()
                .saturating_sub(1)
                .max(closing.start_offset());
            let (closing_line, _) = self.source.offset_to_line_col(end_off);
            if closing_line >= self.parent_last_line {
                self.found = true;
            }
        }
    }

    impl<'pr> Visit<'pr> for LastLineHeredocDetector<'_> {
        fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
            self.visit_heredoc(node.opening_loc(), node.closing_loc());
            if !self.found {
                ruby_prism::visit_string_node(self, node);
            }
        }

        fn visit_interpolated_string_node(
            &mut self,
            node: &ruby_prism::InterpolatedStringNode<'pr>,
        ) {
            self.visit_heredoc(node.opening_loc(), node.closing_loc());
            if !self.found {
                ruby_prism::visit_interpolated_string_node(self, node);
            }
        }
    }

    let parent_last_line = node_last_line(source, node);
    let mut detector = LastLineHeredocDetector {
        source,
        parent_last_line,
        found: false,
    };
    detector.visit(node);
    detector.found
}

fn node_last_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    let loc = node.location();
    let end_off = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    source.offset_to_line_col(end_off).0
}
