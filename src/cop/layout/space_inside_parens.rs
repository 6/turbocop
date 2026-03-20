use crate::cop::node_type::{
    ARRAY_PATTERN_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, DEF_NODE, DEFINED_NODE,
    HASH_PATTERN_NODE, MULTI_TARGET_NODE, PARENTHESES_NODE, PINNED_EXPRESSION_NODE, SUPER_NODE,
    YIELD_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-20)
///
/// Corpus oracle reported FP=1, FN=4,120. Local
/// `verify-cop-locations.py Layout/SpaceInsideParens` now shows the lone CI FP
/// already fixed, so the remaining work is entirely FN.
///
/// The missing detections came from two implementation gaps:
///
/// 1. **Method/lambda parameter parens were never inspected.** The previous
///    code only checked `CallNode` and `ParenthesesNode`, so definitions like
///    `def initialize( options )` and lambda params with `->( value ) { ... }`
///    were invisible.
/// 2. **Multiline parens were skipped wholesale.** RuboCop checks each side of
///    a paren pair independently using adjacent tokens. `deliver( payload,\n`
///    should still flag the opening side, and `format: :json )` should still
///    flag the closing side. The previous `open_line != close_line => skip`
///    shortcut missed both.
///
/// Fix: extract paren pairs from calls, grouping parens, defs, and
/// parenthesized block/lambda params, then apply side-specific same-line
/// checks that mirror RuboCop's token-pair behavior (including comment and
/// empty-parens exceptions). Follow-up investigation against Twilio's
/// generated client code showed that RuboCop also accepts multiline empty
/// parens in `no_space` style (`call(\n)`), so the whitespace-only fast path
/// now preserves that form while still flagging `call( )`. A later acl9
/// reduction showed one more token-driven asymmetry: command-style argument
/// parens like `check ( value)` and `yield ( value)` ignore the opening side
/// entirely, but still check the closing side. We mirror that here with a
/// source-context guard on `ParenthesesNode` opening checks. Remaining live FN
/// investigation in webistrano/rufo also showed extra paren carriers:
/// ternary branches like `? ( value)` are ordinary grouping parens and must
/// not be skipped by that guard, while pattern pins (`^ ( 1 + 2 )`) and block
/// destructuring params (`| ( x ) , y |`) store their delimiters on
/// `PinnedExpressionNode` and `MultiTargetNode`. Constant patterns
/// (`Point( 1 )`, `SuperPoint( x: 1 )`) and parenthesized `yield(...)` calls
/// also need their Prism-specific nodes (`ArrayPatternNode`,
/// `HashPatternNode`, `YieldNode`).
pub struct SpaceInsideParens;

const MSG: &str = "Space inside parentheses detected.";
const MSG_NO_SPACE: &str = "No space inside parentheses detected.";

impl Cop for SpaceInsideParens {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideParens"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_PATTERN_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            DEF_NODE,
            DEFINED_NODE,
            HASH_PATTERN_NODE,
            MULTI_TARGET_NODE,
            PARENTHESES_NODE,
            PINNED_EXPRESSION_NODE,
            SUPER_NODE,
            YIELD_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "no_space");
        let bytes = source.as_bytes();

        let Some((open_start, open_end, close_start)) = paren_offsets(node, bytes) else {
            return;
        };

        if close_start <= open_end {
            return;
        }

        let interior = &bytes[open_end..close_start];
        if interior.is_empty() {
            return;
        }

        // Empty parens always want `()`, even in `space` / `compact`.
        if interior.iter().all(|&b| is_paren_whitespace(b)) {
            if style == "no_space" && interior.contains(&b'\n') {
                return;
            }

            if !interior.is_empty() {
                push_remove_offense(
                    self,
                    source,
                    diagnostics,
                    &mut corrections,
                    open_end,
                    close_start,
                    MSG,
                );
            }
            return;
        }

        let open_side = next_same_line_item(bytes, open_end);
        let close_side = previous_same_line_code(bytes, close_start);

        let ignore_open_side = ignores_open_side(node, bytes, open_start);

        match style {
            "space" => {
                if !ignore_open_side {
                    check_missing_open_space(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        bytes,
                        open_side,
                        false,
                    );
                }
                check_missing_close_space(
                    self,
                    source,
                    diagnostics,
                    &mut corrections,
                    bytes,
                    close_side,
                    close_start,
                    false,
                );
            }
            "compact" => {
                if !ignore_open_side {
                    check_missing_open_space(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        bytes,
                        open_side,
                        true,
                    );
                }
                check_missing_close_space(
                    self,
                    source,
                    diagnostics,
                    &mut corrections,
                    bytes,
                    close_side,
                    close_start,
                    true,
                );
            }
            _ => {
                if !ignore_open_side {
                    check_extraneous_open_space(
                        self,
                        source,
                        diagnostics,
                        &mut corrections,
                        open_end,
                        open_side,
                    );
                }
                check_extraneous_close_space(
                    self,
                    source,
                    diagnostics,
                    &mut corrections,
                    close_start,
                    close_side,
                );
            }
        }
    }
}

fn paren_offsets(node: &ruby_prism::Node<'_>, bytes: &[u8]) -> Option<(usize, usize, usize)> {
    if let Some(parens) = node.as_parentheses_node() {
        let open = parens.opening_loc();
        let close = parens.closing_loc();
        return Some((open.start_offset(), open.end_offset(), close.start_offset()));
    }

    if let Some(call) = node.as_call_node() {
        let open = call.opening_loc()?;
        let close = call.closing_loc()?;
        if open.as_slice() == b"(" && close.as_slice() == b")" {
            return Some((open.start_offset(), open.end_offset(), close.start_offset()));
        }
    }

    if let Some(yield_node) = node.as_yield_node() {
        let open = yield_node.lparen_loc()?;
        let close = yield_node.rparen_loc()?;
        return Some((open.start_offset(), open.end_offset(), close.start_offset()));
    }

    if let Some(def) = node.as_def_node() {
        let open = def.lparen_loc()?;
        let close = def.rparen_loc()?;
        return Some((open.start_offset(), open.end_offset(), close.start_offset()));
    }

    if let Some(multi_target) = node.as_multi_target_node() {
        let open = multi_target.lparen_loc()?;
        let close = multi_target.rparen_loc()?;
        return Some((open.start_offset(), open.end_offset(), close.start_offset()));
    }

    if let Some(array_pattern) = node.as_array_pattern_node() {
        let open = array_pattern.opening_loc()?;
        let close = array_pattern.closing_loc()?;
        if open.as_slice() == b"(" && close.as_slice() == b")" {
            return Some((open.start_offset(), open.end_offset(), close.start_offset()));
        }
    }

    if let Some(hash_pattern) = node.as_hash_pattern_node() {
        let open = hash_pattern.opening_loc()?;
        let close = hash_pattern.closing_loc()?;
        if open.as_slice() == b"(" && close.as_slice() == b")" {
            return Some((open.start_offset(), open.end_offset(), close.start_offset()));
        }
    }

    if let Some(pinned_expression) = node.as_pinned_expression_node() {
        let open = pinned_expression.lparen_loc();
        let close = pinned_expression.rparen_loc();
        return Some((open.start_offset(), open.end_offset(), close.start_offset()));
    }

    if let Some(super_node) = node.as_super_node() {
        let open = super_node.lparen_loc()?;
        let close = super_node.rparen_loc()?;
        return Some((open.start_offset(), open.end_offset(), close.start_offset()));
    }

    if node.as_defined_node().is_some() {
        let loc = node.location();
        let slice = &bytes[loc.start_offset()..loc.end_offset()];
        if !slice.starts_with(b"defined?") {
            return None;
        }

        let mut open_start = loc.start_offset() + b"defined?".len();
        while open_start < loc.end_offset() && matches!(bytes[open_start], b' ' | b'\t' | b'\r') {
            open_start += 1;
        }
        if open_start >= loc.end_offset() || bytes[open_start] != b'(' {
            return None;
        }

        let close_start = slice.iter().rposition(|&b| b == b')')? + loc.start_offset();
        if close_start <= open_start {
            return None;
        }

        return Some((open_start, open_start + 1, close_start));
    }

    if let Some(block_params) = node.as_block_parameters_node() {
        let open = block_params.opening_loc()?;
        let close = block_params.closing_loc()?;
        if open.as_slice() == b"(" && close.as_slice() == b")" {
            return Some((open.start_offset(), open.end_offset(), close.start_offset()));
        }
    }

    None
}

fn ignores_open_side(node: &ruby_prism::Node<'_>, bytes: &[u8], open_start: usize) -> bool {
    if node.as_parentheses_node().is_none() {
        return false;
    }

    command_form_prefix(bytes, open_start).is_some()
}

fn command_form_prefix(bytes: &[u8], open_start: usize) -> Option<&[u8]> {
    if open_start == 0 {
        return None;
    }

    let line_start = bytes[..open_start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);

    let mut word_end = open_start;
    while word_end > line_start && matches!(bytes[word_end - 1], b' ' | b'\t' | b'\r') {
        word_end -= 1;
    }
    if word_end == open_start || word_end == line_start {
        return None;
    }

    let mut word_start = word_end;
    while word_start > line_start && is_identifier_tail(bytes[word_start - 1]) {
        word_start -= 1;
    }
    if word_start == word_end {
        return None;
    }

    if word_start > line_start {
        let prev = bytes[word_start - 1];
        if is_identifier_tail(prev) || matches!(prev, b'.' | b':' | b'@') {
            return None;
        }
    }

    let word = &bytes[word_start..word_end];
    if !matches!(word[0], b'a'..=b'z' | b'A'..=b'Z' | b'_') {
        return None;
    }
    if denied_command_prefix(word) {
        return None;
    }

    Some(word)
}

fn is_identifier_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'?' | b'!')
}

fn denied_command_prefix(word: &[u8]) -> bool {
    matches!(
        word,
        b"if"
            | b"unless"
            | b"while"
            | b"until"
            | b"case"
            | b"for"
            | b"return"
            | b"break"
            | b"next"
            | b"redo"
            | b"retry"
            | b"then"
            | b"elsif"
            | b"when"
            | b"rescue"
            | b"super"
            | b"defined?"
    )
}

#[derive(Clone, Copy)]
enum NextSameLineItem {
    None,
    Comment,
    Code(usize),
}

fn next_same_line_item(bytes: &[u8], offset: usize) -> NextSameLineItem {
    let line_end = bytes[offset..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|idx| offset + idx)
        .unwrap_or(bytes.len());

    let mut idx = offset;
    while idx < line_end && matches!(bytes[idx], b' ' | b'\t' | b'\r') {
        idx += 1;
    }

    if idx >= line_end {
        NextSameLineItem::None
    } else if bytes[idx] == b'#' {
        NextSameLineItem::Comment
    } else {
        NextSameLineItem::Code(idx)
    }
}

fn previous_same_line_code(bytes: &[u8], close_start: usize) -> Option<usize> {
    let line_start = bytes[..close_start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|idx| idx + 1)
        .unwrap_or(0);

    let mut idx = close_start;
    while idx > line_start && matches!(bytes[idx - 1], b' ' | b'\t' | b'\r') {
        idx -= 1;
    }

    if idx == line_start {
        None
    } else {
        Some(idx - 1)
    }
}

fn check_extraneous_open_space(
    cop: &SpaceInsideParens,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    open_end: usize,
    open_side: NextSameLineItem,
) {
    if let NextSameLineItem::Code(code_start) = open_side {
        if code_start > open_end {
            push_remove_offense(
                cop,
                source,
                diagnostics,
                corrections,
                open_end,
                code_start,
                MSG,
            );
        }
    }
}

fn check_extraneous_close_space(
    cop: &SpaceInsideParens,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    close_start: usize,
    close_side: Option<usize>,
) {
    let Some(prev_code) = close_side else {
        return;
    };
    let space_start = prev_code + 1;
    if space_start < close_start {
        push_remove_offense(
            cop,
            source,
            diagnostics,
            corrections,
            space_start,
            close_start,
            MSG,
        );
    }
}

fn check_missing_open_space(
    cop: &SpaceInsideParens,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    bytes: &[u8],
    open_side: NextSameLineItem,
    allow_consecutive_left_parens: bool,
) {
    let NextSameLineItem::Code(code_start) = open_side else {
        return;
    };
    if allow_consecutive_left_parens && bytes.get(code_start) == Some(&b'(') {
        return;
    }
    if code_start == 0 {
        return;
    }
    if bytes.get(code_start - 1) == Some(&b' ') {
        return;
    }
    push_insert_offense(
        cop,
        source,
        diagnostics,
        corrections,
        code_start,
        MSG_NO_SPACE,
    );
}

#[allow(clippy::too_many_arguments)]
fn check_missing_close_space(
    cop: &SpaceInsideParens,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    bytes: &[u8],
    close_side: Option<usize>,
    close_start: usize,
    allow_consecutive_right_parens: bool,
) {
    let Some(prev_code) = close_side else {
        return;
    };
    if allow_consecutive_right_parens && bytes.get(prev_code) == Some(&b')') {
        return;
    }
    if prev_code + 1 != close_start {
        return;
    }
    push_insert_offense(
        cop,
        source,
        diagnostics,
        corrections,
        close_start,
        MSG_NO_SPACE,
    );
}

fn push_remove_offense(
    cop: &SpaceInsideParens,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    start: usize,
    end: usize,
    message: &str,
) {
    let (line, column) = source.offset_to_line_col(start);
    let mut diag = cop.diagnostic(source, line, column, message.to_string());
    if let Some(corrs) = corrections.as_deref_mut() {
        corrs.push(crate::correction::Correction {
            start,
            end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn push_insert_offense(
    cop: &SpaceInsideParens,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    offset: usize,
    message: &str,
) {
    let (line, column) = source.offset_to_line_col(offset);
    let mut diag = cop.diagnostic(source, line, column, message.to_string());
    if let Some(corrs) = corrections.as_deref_mut() {
        corrs.push(crate::correction::Correction {
            start: offset,
            end: offset,
            replacement: " ".to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn is_paren_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceInsideParens, "cops/layout/space_inside_parens");
    crate::cop_autocorrect_fixture_tests!(SpaceInsideParens, "cops/layout/space_inside_parens");

    #[test]
    fn space_style_flags_missing_spaces() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"x = (1 + 2)\n";
        let diags = run_cop_full_with_config(&SpaceInsideParens, src, config);
        assert_eq!(
            diags.len(),
            2,
            "space style should flag missing spaces inside parens"
        );
        assert!(diags[0].message.contains("No space"));
    }

    #[test]
    fn space_style_accepts_spaces() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"x = ( 1 + 2 )\n";
        assert_cop_no_offenses_full_with_config(&SpaceInsideParens, src, config);
    }

    #[test]
    fn space_style_command_form_only_requires_closing_space() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("space".into()),
            )]),
            ..CopConfig::default()
        };
        let src = b"check ( value)\n";
        let diags = run_cop_full_with_config(&SpaceInsideParens, src, config);
        assert_eq!(
            diags.len(),
            1,
            "command-form parens should only check the closing side"
        );
        assert_eq!(diags[0].location.column, 13);
        assert!(diags[0].message.contains("No space"));
    }
}
