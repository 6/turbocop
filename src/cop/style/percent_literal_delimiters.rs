use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;

pub struct PercentLiteralDelimiters;

impl PercentLiteralDelimiters {
    /// Parse PreferredDelimiters config into a map from literal type to (open, close).
    /// RuboCop defaults: () for most, [] for %w/%W/%i/%I, {} for %r.
    fn preferred_delimiters(config: &CopConfig) -> HashMap<String, (u8, u8)> {
        let mut map = HashMap::new();
        // RuboCop vendor defaults
        let defaults: &[(&str, u8, u8)] = &[
            ("%w",  b'[', b']'),
            ("%W",  b'[', b']'),
            ("%i",  b'[', b']'),
            ("%I",  b'[', b']'),
            ("%r",  b'{', b'}'),
            ("%q",  b'(', b')'),
            ("%Q",  b'(', b')'),
            ("%s",  b'(', b')'),
            ("%x",  b'(', b')'),
            ("%",   b'(', b')'),
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

    /// Given a Prism node's opening bytes (e.g. `%w[`), extract the literal type and actual delimiter.
    fn parse_percent_opening(open_bytes: &[u8]) -> Option<(String, u8)> {
        if open_bytes.len() < 2 || open_bytes[0] != b'%' {
            return None;
        }
        let second = open_bytes[1];
        // Check for %w, %W, %i, %I, %q, %Q, %r, %s, %x
        if matches!(second, b'w' | b'W' | b'i' | b'I' | b'q' | b'Q' | b'r' | b's' | b'x') {
            if open_bytes.len() >= 3 {
                let literal_type = format!("%{}", second as char);
                let delimiter = open_bytes[2];
                return Some((literal_type, delimiter));
            }
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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
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
            return Vec::new();
        };

        let opening = match opening_loc {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let open_bytes = opening.as_slice();

        let (literal_type, actual_delim) = match Self::parse_percent_opening(open_bytes) {
            Some(r) => r,
            None => return Vec::new(),
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
            let loc = opening;
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "`{literal_type}`-literals should be delimited by `{}` and `{}`.",
                    expected_open as char, expected_close as char,
                ),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PercentLiteralDelimiters, "cops/style/percent_literal_delimiters");
}
