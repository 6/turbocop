use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RuboCop's NodePattern for this cop (`(dstr ({str dstr sym} ...) ...)`) requires
/// the first child of a dstr to be str/dstr/sym. Interpolated strings where the
/// first part is an EmbeddedStatementsNode (e.g., `"#{method} text "`) don't match
/// the pattern, so RuboCop skips them entirely. We mirror this by checking that
/// the first part of an InterpolatedStringNode is a StringNode before inspecting
/// for excessive whitespace.
///
/// Corpus FP fix: decidim/decidim — `"#{method} returns true for #{os} "` was
/// incorrectly flagged for trailing space because we checked raw source of all
/// interpolated strings. Now we skip strings whose first part is interpolation.
pub struct ExcessiveDocstringSpacing;

impl Cop for ExcessiveDocstringSpacing {
    fn name(&self) -> &'static str {
        "RSpec/ExcessiveDocstringSpacing"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Must be an RSpec method (example group, example, skip, its, etc.)
        let is_rspec = is_rspec_example_group(method_name)
            || is_rspec_example(method_name)
            || method_name == b"its";

        if !is_rspec {
            return;
        }

        // Must be receiverless or RSpec.describe / ::RSpec.describe
        if let Some(recv) = call.receiver() {
            if constant_predicates::constant_short_name(&recv).is_none_or(|n| n != b"RSpec") {
                return;
            }
        }

        // Get first argument — must be a string
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];

        // Get the string content
        let string_content = if let Some(s) = first_arg.as_string_node() {
            // Skip heredoc descriptions (opening_loc starts with <<)
            if let Some(open_loc) = s.opening_loc() {
                let open = &source.as_bytes()[open_loc.start_offset()..open_loc.end_offset()];
                if open.starts_with(b"<<") {
                    return;
                }
            }
            s.unescaped().to_vec()
        } else if let Some(s) = first_arg.as_interpolated_string_node() {
            if s.opening_loc().is_none() {
                // Implicit string concatenation (backslash continuation) —
                // concatenate all parts and check the combined result.
                let mut combined = Vec::new();
                for part in s.parts().iter() {
                    if let Some(str_part) = part.as_string_node() {
                        combined.extend_from_slice(str_part.unescaped());
                    } else {
                        // Has real interpolation — use raw source fallback
                        combined.clear();
                        let loc = s.location();
                        let raw = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
                        // Can't reliably extract content; skip
                        if raw.len() < 2 {
                            return;
                        }
                        // For mixed interpolation in concat, just grab each part's source
                        for inner_part in s.parts().iter() {
                            if let Some(sp) = inner_part.as_string_node() {
                                combined.extend_from_slice(sp.unescaped());
                            } else {
                                // Interpolation node — include its source as-is
                                let ploc = inner_part.location();
                                combined.extend_from_slice(
                                    &source.as_bytes()[ploc.start_offset()..ploc.end_offset()],
                                );
                            }
                        }
                        break;
                    }
                }
                combined
            } else {
                // Real interpolated string ("...#{...}...").
                // RuboCop's NodePattern `(dstr ({str dstr sym} ...) ...)` requires the
                // first child of the dstr to be a str/dstr/sym. If the first part is
                // interpolation (EmbeddedStatementsNode), the pattern doesn't match and
                // RuboCop skips the check entirely.
                let parts: Vec<_> = s.parts().iter().collect();
                if parts.is_empty() || parts[0].as_string_node().is_none() {
                    return;
                }
                // First part is a string literal — check raw source between quotes
                let loc = s.location();
                let raw = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
                if raw.len() >= 2 {
                    let inner = &raw[1..raw.len() - 1];
                    inner.to_vec()
                } else {
                    return;
                }
            }
        } else {
            return;
        };

        // Check for excessive whitespace: leading, trailing, or multiple consecutive spaces
        let content_str = match std::str::from_utf8(&string_content) {
            Ok(s) => s,
            Err(_) => return,
        };

        let has_leading_space = content_str.starts_with(' ')
            || content_str.starts_with('\u{3000}')
            || content_str.starts_with('\u{00a0}');
        let has_trailing_space = content_str.ends_with(' ')
            || content_str.ends_with('\u{3000}')
            || content_str.ends_with('\u{00a0}');
        // RuboCop checks: [^[[:space:]]][[:blank:]]{2,}[^[[:blank:]]]
        // Two or more consecutive blanks must be preceded by a non-whitespace character
        // (so leading indentation on continuation lines doesn't count).
        let has_multiple_spaces = {
            let bytes = content_str.as_bytes();
            let mut found = false;
            let mut i = 0;
            while i + 2 < bytes.len() {
                if !bytes[i].is_ascii_whitespace()
                    && (bytes[i + 1] == b' ' || bytes[i + 1] == b'\t')
                {
                    // Found a non-space char followed by blank; count consecutive blanks
                    let mut j = i + 1;
                    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                        j += 1;
                    }
                    if j - (i + 1) >= 2 && j < bytes.len() && bytes[j] != b' ' && bytes[j] != b'\t'
                    {
                        found = true;
                        break;
                    }
                    i = j;
                } else {
                    i += 1;
                }
            }
            found
        };

        if !has_leading_space && !has_trailing_space && !has_multiple_spaces {
            return;
        }

        let loc = first_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Excessive whitespace.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ExcessiveDocstringSpacing,
        "cops/rspec/excessive_docstring_spacing"
    );
}
