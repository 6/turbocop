use crate::cop::shared::node_type::{
    ARRAY_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, INTERPOLATED_STRING_NODE,
    INTERPOLATED_SYMBOL_NODE, INTERPOLATED_X_STRING_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE,
    SYMBOL_NODE, X_STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;

/// Style/PercentLiteralDelimiters enforces consistent %-literal delimiters.
///
/// ## Investigation findings (2026-03-18)
///
/// ### FP root cause fixed:
/// - `%w` and `%i` literals using non-preferred delimiters where the content
///   contains the same characters as the used delimiter's matchpair (e.g.,
///   `%w(foo( bar))`) — changing delimiters would require escaping. Added
///   `include_same_character_as_used_for_delimiter?` check matching RuboCop.
///
/// ## Investigation findings (2026-03-29)
///
/// ### FN root cause fixed (1831 FN):
/// - For interpolated percent literals (`%Q{...}`, `%{...}`, `%W|...|`,
///   `%x[...]`, etc.), the content-contains-delimiter check was scanning raw
///   source bytes including interpolation expressions like `#{pars.join(' ')}`.
///   This caused false skips when delimiter characters appeared inside `#{}`
///   but not in literal string content. Fixed by matching RuboCop's approach:
///   only check `StringNode` parts (literal text), skip `EmbeddedStatementsNode`
///   parts (interpolation). For arrays, only check `StringNode`/`SymbolNode`
///   elements.
///
/// ## Investigation findings (2026-03-30)
///
/// ### FN root cause fixed (1 FN):
/// - `%s` symbol literals with non-preferred delimiters were incorrectly
///   exempted when their raw symbol body contained the preferred delimiter
///   characters, e.g. `%s"()<>\[\]{}/%\s"`. RuboCop still flags non-preferred
///   `%s` delimiters in this case, so `%s` must not use the generic
///   content-contains-delimiter skip that applies to strings, regexps, and
///   arrays. Fixed by carving `%s` out of that skip logic while keeping the
///   existing `%w`/`%i` same-delimiter guard unchanged.
pub struct PercentLiteralDelimiters;

impl PercentLiteralDelimiters {
    /// Parse PreferredDelimiters config into a map from literal type to (open, close).
    /// RuboCop defaults: () for most, [] for %w/%W/%i/%I, {} for %r.
    fn preferred_delimiters(config: &CopConfig) -> HashMap<String, (u8, u8)> {
        let mut map = HashMap::new();
        // RuboCop vendor defaults
        let defaults: &[(&str, u8, u8)] = &[
            ("%w", b'[', b']'),
            ("%W", b'[', b']'),
            ("%i", b'[', b']'),
            ("%I", b'[', b']'),
            ("%r", b'{', b'}'),
            ("%q", b'(', b')'),
            ("%Q", b'(', b')'),
            ("%s", b'(', b')'),
            ("%x", b'(', b')'),
            ("%", b'(', b')'),
        ];
        for &(key, open, close) in defaults {
            map.insert(key.to_string(), (open, close));
        }

        if let Some(preferred) = config.get_string_hash("PreferredDelimiters") {
            // First check for a "default" key that overrides all
            if let Some(default_val) = preferred.get("default") {
                if default_val.len() >= 2 {
                    let bytes = default_val.as_bytes();
                    let open = bytes[0];
                    let close = bytes[1];
                    for v in map.values_mut() {
                        *v = (open, close);
                    }
                }
            }
            // Then apply per-type overrides
            for (key, val) in &preferred {
                if key == "default" {
                    continue;
                }
                if val.len() >= 2 {
                    let bytes = val.as_bytes();
                    let open = bytes[0];
                    let close = bytes[1];
                    map.insert(key.clone(), (open, close));
                }
            }
        }

        map
    }

    /// Check if the literal (non-interpolated) content of a percent literal contains
    /// either of the given delimiter bytes. For interpolated nodes, only StringNode
    /// parts are checked, matching RuboCop's `contains_delimiter?` + `string_source`.
    fn literal_content_contains(
        node: &ruby_prism::Node<'_>,
        source: &SourceFile,
        opening: &ruby_prism::Location<'_>,
        d1: u8,
        d2: u8,
    ) -> bool {
        let bytes_contain = |bytes: &[u8]| bytes.contains(&d1) || bytes.contains(&d2);

        // For interpolated nodes, only check StringNode parts
        if let Some(s) = node.as_interpolated_string_node() {
            return s.parts().iter().any(|p| {
                p.as_string_node()
                    .is_some_and(|s| bytes_contain(s.content_loc().as_slice()))
            });
        }
        if let Some(s) = node.as_interpolated_x_string_node() {
            return s.parts().iter().any(|p| {
                p.as_string_node()
                    .is_some_and(|s| bytes_contain(s.content_loc().as_slice()))
            });
        }
        if let Some(s) = node.as_interpolated_regular_expression_node() {
            return s.parts().iter().any(|p| {
                p.as_string_node()
                    .is_some_and(|s| bytes_contain(s.content_loc().as_slice()))
            });
        }
        if let Some(s) = node.as_interpolated_symbol_node() {
            return s.parts().iter().any(|p| {
                p.as_string_node()
                    .is_some_and(|s| bytes_contain(s.content_loc().as_slice()))
            });
        }
        // For arrays, only check StringNode and SymbolNode elements
        if let Some(a) = node.as_array_node() {
            return a.elements().iter().any(|elem| {
                if let Some(s) = elem.as_string_node() {
                    bytes_contain(s.content_loc().as_slice())
                } else if let Some(s) = elem.as_symbol_node() {
                    s.value_loc().is_some_and(|v| bytes_contain(v.as_slice()))
                } else {
                    false
                }
            });
        }

        // For non-interpolated nodes, check raw bytes between opening and closing
        let node_loc = node.location();
        let content_start = opening.end_offset();
        let content_end = node_loc.end_offset().saturating_sub(1);
        if content_end > content_start {
            let content = &source.as_bytes()[content_start..content_end];
            bytes_contain(content)
        } else {
            false
        }
    }

    /// Given a Prism node's opening bytes (e.g. `%w[`), extract the literal type and actual delimiter.
    fn parse_percent_opening(open_bytes: &[u8]) -> Option<(String, u8)> {
        if open_bytes.len() < 2 || open_bytes[0] != b'%' {
            return None;
        }
        let second = open_bytes[1];
        // Check for %w, %W, %i, %I, %q, %Q, %r, %s, %x
        if matches!(
            second,
            b'w' | b'W' | b'i' | b'I' | b'q' | b'Q' | b'r' | b's' | b'x'
        ) && open_bytes.len() >= 3
        {
            let literal_type = format!("%{}", second as char);
            let delimiter = open_bytes[2];
            return Some((literal_type, delimiter));
        }
        // Bare %( is same as %Q(
        if !second.is_ascii_alphabetic() {
            let literal_type = "%".to_string();
            let delimiter = second;
            return Some((literal_type, delimiter));
        }
        None
    }
}

impl Cop for PercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Style/PercentLiteralDelimiters"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            INTERPOLATED_STRING_NODE,
            INTERPOLATED_SYMBOL_NODE,
            INTERPOLATED_X_STRING_NODE,
            REGULAR_EXPRESSION_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            X_STRING_NODE,
        ]
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
        let preferred = Self::preferred_delimiters(config);

        // Get opening_loc from the node — only percent-literal node types have them
        let opening_loc = if let Some(a) = node.as_array_node() {
            // %w[], %W[], %i[], %I[] arrays
            a.opening_loc()
        } else if let Some(r) = node.as_regular_expression_node() {
            // %r{} regexp
            Some(r.opening_loc())
        } else if let Some(r) = node.as_interpolated_regular_expression_node() {
            // %r{} interpolated regexp
            Some(r.opening_loc())
        } else if let Some(s) = node.as_string_node() {
            // %q(), %Q() strings
            s.opening_loc()
        } else if let Some(s) = node.as_interpolated_string_node() {
            // %Q() interpolated strings
            s.opening_loc()
        } else if let Some(s) = node.as_x_string_node() {
            // %x() command strings
            Some(s.opening_loc())
        } else if let Some(s) = node.as_interpolated_x_string_node() {
            // %x() interpolated command strings
            Some(s.opening_loc())
        } else if let Some(s) = node.as_interpolated_symbol_node() {
            // %s() symbols
            s.opening_loc()
        } else if let Some(s) = node.as_symbol_node() {
            // %s() symbols
            s.opening_loc()
        } else {
            return;
        };

        let opening = match opening_loc {
            Some(loc) => loc,
            None => return,
        };

        let open_bytes = opening.as_slice();

        let (literal_type, actual_delim) = match Self::parse_percent_opening(open_bytes) {
            Some(r) => r,
            None => return,
        };

        let (expected_open, expected_close) = match preferred.get(&literal_type) {
            Some(&(o, c)) => (o, c),
            None => {
                // Try the default "%" key
                match preferred.get("%") {
                    Some(&(o, c)) => (o, c),
                    None => (b'(', b')'),
                }
            }
        };

        if actual_delim != expected_open {
            if literal_type != "%s" {
                // Check if the literal content contains the preferred delimiters.
                // For interpolated nodes, only check literal string parts (StringNode),
                // not interpolation expressions (EmbeddedStatementsNode).
                // This matches RuboCop's contains_preferred_delimiter? behavior.
                if Self::literal_content_contains(
                    node,
                    source,
                    &opening,
                    expected_open,
                    expected_close,
                ) {
                    return;
                }

                // For %w and %i literals, also check if content contains the same
                // characters as the currently-used delimiters (matchpairs).
                // RuboCop's include_same_character_as_used_for_delimiter? check.
                if literal_type == "%w" || literal_type == "%i" {
                    let (used_open, used_close) = matchpair(actual_delim);
                    if Self::literal_content_contains(node, source, &opening, used_open, used_close)
                    {
                        return;
                    }
                }
            }

            let loc = opening;
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "`{literal_type}`-literals should be delimited by `{}` and `{}`.",
                    expected_open as char, expected_close as char,
                ),
            ));
        }
    }
}

/// Return the open/close matchpair for a delimiter character.
fn matchpair(delim: u8) -> (u8, u8) {
    match delim {
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        b'{' => (b'{', b'}'),
        b'<' => (b'<', b'>'),
        _ => (delim, delim),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        PercentLiteralDelimiters,
        "cops/style/percent_literal_delimiters"
    );
}
