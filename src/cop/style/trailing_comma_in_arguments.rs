use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Investigation (2026-03-03)
///
/// Found 12 FPs with `EnforcedStyleForMultiline: comma`. Root cause: Prism
/// collapses keyword args into a single KeywordHashNode. The
/// `no_elements_on_same_line` check iterated over top-level args, so with
/// 1 KeywordHashNode the consecutive-pairs check vacuously passed. Fix: expand
/// KeywordHashNode into individual assoc elements for line comparisons (dc856393).
///
/// Investigation (2026-03-29)
///
/// Root cause of 480 FNs: when any call argument contained a heredoc, the cop
/// scanned from the last argument end offset all the way to `)`. In Prism that
/// range includes heredoc body text, so opener-line commas such as
/// `<<~GRAPHQL,` or `body: <<~BODY,` were rejected as if they were content.
/// Fix: mirror RuboCop's heredoc path and, when any argument contains a heredoc,
/// only search for a trailing comma on the same opener line.
///
/// Investigation (2026-03-30)
///
/// Root cause of 8 FNs: `is_heredoc_argument` did not recurse into explicit
/// `HashNode` values (e.g., `{ text: <<-END }`), only `KeywordHashNode`. When a
/// heredoc was nested inside a hash literal, `has_heredoc` was false, causing the
/// scanner to read through heredoc body content and miss the trailing comma.
/// Fix: add `HashNode` handling to `is_heredoc_argument`.
pub struct TrailingCommaInArguments;

/// Collect effective element locations, expanding any KeywordHashNode into its
/// individual assoc elements. This matches RuboCop's behavior where keyword args
/// are treated as separate elements for the no_elements_on_same_line? check.
fn effective_element_locations<'a>(
    arg_list: impl Iterator<Item = ruby_prism::Node<'a>>,
) -> Vec<(usize, usize)> {
    let mut locations = Vec::new();
    for arg in arg_list {
        if let Some(kw_hash) = arg.as_keyword_hash_node() {
            for elem in kw_hash.elements().iter() {
                let loc = elem.location();
                locations.push((loc.start_offset(), loc.end_offset()));
            }
        } else {
            let loc = arg.location();
            locations.push((loc.start_offset(), loc.end_offset()));
        }
    }
    locations
}

/// Check if a byte range contains only whitespace and exactly one comma.
/// Returns false if there are other non-whitespace characters (e.g., heredoc content).
/// Comments (starting with #) are treated as whitespace.
fn is_only_whitespace_and_comma(bytes: &[u8]) -> bool {
    let mut found_comma = false;
    let mut in_comment = false;
    for &b in bytes {
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            continue;
        }
        match b {
            b',' => {
                if found_comma {
                    return false; // Multiple commas
                }
                found_comma = true;
            }
            b'#' => {
                in_comment = true;
            }
            b' ' | b'\t' | b'\n' | b'\r' => {}
            _ => return false, // Non-whitespace, non-comma character
        }
    }
    found_comma
}

/// Like `is_only_whitespace_and_comma`, but stops at the first newline. This
/// matches RuboCop's heredoc-specific comma detection and avoids scanning into
/// heredoc bodies.
fn is_only_horizontal_whitespace_and_comma(bytes: &[u8]) -> bool {
    for &b in bytes {
        match b {
            b' ' | b'\t' => {}
            b',' => return true,
            b'\n' | b'\r' => return false,
            _ => return false,
        }
    }
    false
}

fn find_trailing_comma_offset(
    bytes: &[u8],
    start: usize,
    end: usize,
    stop_at_newline: bool,
) -> Option<usize> {
    if start >= end || end > bytes.len() {
        return None;
    }

    for (idx, &b) in bytes[start..end].iter().enumerate() {
        if stop_at_newline && matches!(b, b'\n' | b'\r') {
            return None;
        }
        if b == b',' {
            return Some(start + idx);
        }
    }

    None
}

fn any_heredoc<'a>(mut args: impl Iterator<Item = ruby_prism::Node<'a>>) -> bool {
    args.any(|arg| is_heredoc_argument(&arg))
}

fn is_heredoc_argument(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(assoc) = node.as_assoc_node() {
        return is_heredoc_argument(&assoc.value());
    }

    if let Some(kw_hash) = node.as_keyword_hash_node() {
        return kw_hash
            .elements()
            .iter()
            .any(|elem| is_heredoc_argument(&elem));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash
            .elements()
            .iter()
            .any(|elem| is_heredoc_argument(&elem));
    }

    if let Some(s) = node.as_interpolated_string_node() {
        return s
            .opening_loc()
            .is_some_and(|open| open.as_slice().starts_with(b"<<"));
    }

    if let Some(s) = node.as_string_node() {
        return s
            .opening_loc()
            .is_some_and(|open| open.as_slice().starts_with(b"<<"));
    }

    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if is_heredoc_argument(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            return args.arguments().iter().any(|arg| is_heredoc_argument(&arg));
        }
    }

    false
}

impl Cop for TrailingCommaInArguments {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArguments"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
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
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let closing_loc = match call_node.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let arguments = match call_node.arguments() {
            Some(args) => args,
            None => return,
        };

        let arg_list = arguments.arguments();
        let last_arg = match arg_list.last() {
            Some(a) => a,
            None => return,
        };

        let last_end = last_arg.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();
        let has_heredoc = any_heredoc(arg_list.iter());

        // Skip if there's a block argument (&block) between last arg and closing paren.
        // The comma before &block is a separator, not a trailing comma.
        if let Some(block) = call_node.block() {
            if block.as_block_argument_node().is_some() {
                return;
            }
        }

        // Check for a trailing comma between the last argument and closing paren.
        if closing_start > bytes.len() {
            return;
        }

        let has_comma = if last_end < closing_start {
            let search_range = &bytes[last_end..closing_start];
            if has_heredoc {
                is_only_horizontal_whitespace_and_comma(search_range)
            } else {
                is_only_whitespace_and_comma(search_range)
            }
        } else {
            false
        };

        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");

        // Determine if the call is multiline and whether a trailing comma should be present
        let close_line = source.offset_to_line_col(closing_start).0;
        let call_start_line = source
            .offset_to_line_col(call_node.location().start_offset())
            .0;
        let call_is_multiline = close_line > call_start_line;

        // For single-argument calls where closing bracket is on the same line as
        // the end of the argument, RuboCop does not consider it multiline.
        // Expand KeywordHashNode to count individual keyword args.
        let effective_args = {
            let mut count = 0usize;
            for arg in arg_list.iter() {
                if let Some(kw_hash) = arg.as_keyword_hash_node() {
                    count += kw_hash.elements().len();
                } else {
                    count += 1;
                }
            }
            count
        };
        if effective_args == 1 {
            let last_arg_end_line = source.offset_to_line_col(last_end).0;
            if close_line == last_arg_end_line {
                // Single arg with closing bracket on same line — not considered multiline
                // for trailing comma purposes (but unwanted commas still detected below)
                if has_comma && last_end < closing_start {
                    if let Some(abs_offset) =
                        find_trailing_comma_offset(bytes, last_end, closing_start, has_heredoc)
                    {
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last parameter of a method call.".to_string(),
                        ));
                    }
                }
                return;
            }
        }

        let is_multiline = match style {
            "consistent_comma" => {
                // For consistent_comma: multiline means the call spans multiple lines
                // AND the method name is NOT on the same line as the last argument's last line.
                // This mirrors RuboCop's method_name_and_arguments_on_same_line? check.
                if !call_is_multiline {
                    false
                } else {
                    // Get the method name line (message_loc or call start)
                    let method_line = call_node
                        .message_loc()
                        .map(|loc| source.offset_to_line_col(loc.start_offset()).0)
                        .unwrap_or(call_start_line);
                    let last_arg_end_line = source.offset_to_line_col(last_end).0;
                    method_line != last_arg_end_line
                }
            }
            _ => {
                let last_line = source.offset_to_line_col(last_end).0;
                close_line > last_line
            }
        };

        match style {
            "comma" | "consistent_comma" => {
                // For "comma" style, RuboCop also checks no_elements_on_same_line:
                // each pair of consecutive elements (plus closing bracket) must be
                // on separate lines. If any two share a line, no trailing comma needed.
                let all_on_own_line = if style == "comma" {
                    // Expand KeywordHashNode to individual elements so that
                    // keyword args sharing a line are correctly detected.
                    let elem_locs = effective_element_locations(arg_list.iter());
                    let mut lines: Vec<(usize, usize)> = Vec::new(); // (last_line, next_first_line)
                    for i in 0..elem_locs.len() {
                        let end_line = source.offset_to_line_col(elem_locs[i].1).0;
                        let next_start_line = if i + 1 < elem_locs.len() {
                            source.offset_to_line_col(elem_locs[i + 1].0).0
                        } else {
                            close_line
                        };
                        lines.push((end_line, next_start_line));
                    }
                    lines.iter().all(|(a, b)| a != b)
                } else {
                    true
                };
                if is_multiline && !has_comma && all_on_own_line {
                    let (line, column) = source.offset_to_line_col(last_end);
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            line,
                            column,
                            "Put a comma after the last parameter of a multiline method call."
                                .to_string(),
                        ),
                    );
                }
            }
            _ => {
                if has_comma && last_end < closing_start {
                    if let Some(abs_offset) =
                        find_trailing_comma_offset(bytes, last_end, closing_start, has_heredoc)
                    {
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last parameter of a method call.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(
        TrailingCommaInArguments,
        "cops/style/trailing_comma_in_arguments"
    );

    fn consistent_comma_config() -> CopConfig {
        use std::collections::HashMap;
        CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForMultiline".into(),
                serde_yml::Value::String("consistent_comma".into()),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn consistent_comma_multiline_closing_on_same_line_as_last_arg() {
        // The closing paren is on the same line as the last arg, but the method name
        // is on a different line — this should require a trailing comma.
        let source = b"matching_token_for(\n  application, resource_owner, scopes, include_expired: false)\n";
        let diags =
            run_cop_full_with_config(&TrailingCommaInArguments, source, consistent_comma_config());
        assert_eq!(
            diags.len(),
            1,
            "consistent_comma should flag multiline call even when ) is on same line as last arg"
        );
    }

    #[test]
    fn consistent_comma_multiline_positional_args_closing_same_line() {
        // Same pattern but with only positional args (no keyword hash)
        let source = b"foo(\n  1, 2, 3)\n";
        let diags =
            run_cop_full_with_config(&TrailingCommaInArguments, source, consistent_comma_config());
        assert_eq!(
            diags.len(),
            1,
            "consistent_comma should flag multiline positional args"
        );
    }

    #[test]
    fn consistent_comma_single_line_no_offense() {
        let source = b"foo(1, 2, 3)\n";
        let diags =
            run_cop_full_with_config(&TrailingCommaInArguments, source, consistent_comma_config());
        assert!(
            diags.is_empty(),
            "Single line should not require trailing comma"
        );
    }

    #[test]
    fn consistent_comma_multiline_with_comma_no_offense() {
        let source = b"foo(\n  1,\n  2,\n)\n";
        let diags =
            run_cop_full_with_config(&TrailingCommaInArguments, source, consistent_comma_config());
        assert!(
            diags.is_empty(),
            "Multiline with trailing comma should be ok"
        );
    }

    fn comma_config() -> CopConfig {
        use std::collections::HashMap;
        CopConfig {
            options: HashMap::from([(
                "EnforcedStyleForMultiline".into(),
                serde_yml::Value::String("comma".into()),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn comma_style_args_on_same_line_no_offense() {
        // When multiple args share a line, comma style does NOT require trailing comma.
        // This matches RuboCop's no_elements_on_same_line? check.
        let source = b"not_change(\n  event.class, :count\n)\n";
        let diags = run_cop_full_with_config(&TrailingCommaInArguments, source, comma_config());
        assert!(
            diags.is_empty(),
            "comma style should not flag when args share a line"
        );
    }

    #[test]
    fn comma_style_each_arg_own_line_offense() {
        // When each arg is on its own line, comma style requires trailing comma.
        let source = b"not_change(\n  event.class,\n  :count\n)\n";
        let diags = run_cop_full_with_config(&TrailingCommaInArguments, source, comma_config());
        assert_eq!(
            diags.len(),
            1,
            "comma style should flag when each arg is on its own line"
        );
    }

    #[test]
    fn comma_style_keyword_args_sharing_line_no_offense() {
        // Keyword args form a single KeywordHashNode in Prism, but the
        // no_elements_on_same_line check must expand it to individual elements.
        let source =
            b"Retriable.retriable(\n  on: StandardError,\n  tries: 7, base_interval: 1.0\n)\n";
        let diags = run_cop_full_with_config(&TrailingCommaInArguments, source, comma_config());
        assert!(
            diags.is_empty(),
            "comma style should not flag when keyword args share a line"
        );
    }

    #[test]
    fn comma_style_keyword_args_each_own_line_offense() {
        // Each keyword arg on its own line — should require trailing comma.
        let source = b"foo(\n  on: StandardError,\n  tries: 7\n)\n";
        let diags = run_cop_full_with_config(&TrailingCommaInArguments, source, comma_config());
        assert_eq!(
            diags.len(),
            1,
            "comma style should flag when each keyword arg is on its own line"
        );
    }

    #[test]
    fn comma_style_mixed_args_keyword_sharing_line_no_offense() {
        // Positional arg + keyword args where keywords share a line
        let source = b"foo(\n  1,\n  a: 2, b: 3\n)\n";
        let diags = run_cop_full_with_config(&TrailingCommaInArguments, source, comma_config());
        assert!(
            diags.is_empty(),
            "comma style should not flag when keyword args share a line (mixed args)"
        );
    }
}
