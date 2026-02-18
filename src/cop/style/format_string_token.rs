use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FormatStringToken;

impl FormatStringToken {
    /// Check for annotated tokens like %<name>s
    fn has_annotated_token(s: &str) -> bool {
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
                    i += 2;
                    continue;
                }
                if i + 1 < bytes.len() && bytes[i + 1] == b'<' {
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    /// Check for template tokens like %{name}
    fn has_template_token(s: &str) -> bool {
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
                    i += 2;
                    continue;
                }
                if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    /// Check for unannotated tokens like %s, %d, %f
    fn count_unannotated_tokens(s: &str) -> usize {
        let bytes = s.as_bytes();
        let mut count = 0;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if i + 1 < bytes.len() {
                    let next = bytes[i + 1];
                    if next == b'%' {
                        i += 2;
                        continue;
                    }
                    if next == b'<' || next == b'{' {
                        i += 2;
                        continue;
                    }
                    // Skip flags and width
                    let mut j = i + 1;
                    while j < bytes.len() && (bytes[j] == b'-' || bytes[j] == b'+' || bytes[j] == b' ' || bytes[j] == b'0' || bytes[j] == b'#' || bytes[j].is_ascii_digit() || bytes[j] == b'.' || bytes[j] == b'*') {
                        j += 1;
                    }
                    if j < bytes.len() && matches!(bytes[j], b's' | b'd' | b'f' | b'g' | b'e' | b'x' | b'X' | b'o' | b'b' | b'B' | b'i' | b'u' | b'c' | b'p' | b'a' | b'A' | b'E' | b'G') {
                        count += 1;
                    }
                }
            }
            i += 1;
        }
        count
    }
}

impl Cop for FormatStringToken {
    fn name(&self) -> &'static str {
        "Style/FormatStringToken"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let string_node = match node.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content_bytes = string_node.unescaped();
        let content_str = match std::str::from_utf8(&content_bytes) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let style = config.get_str("EnforcedStyle", "annotated");
        let max_unannotated = config.get_usize("MaxUnannotatedPlaceholdersAllowed", 1);
        let _mode = config.get_str("Mode", "aggressive");

        let has_annotated = Self::has_annotated_token(content_str);
        let has_template = Self::has_template_token(content_str);
        let unannotated_count = Self::count_unannotated_tokens(content_str);

        let loc = string_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        match style {
            "annotated" => {
                if has_template {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).".to_string(),
                    )];
                }
                if unannotated_count > max_unannotated {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).".to_string(),
                    )];
                }
            }
            "template" => {
                if has_annotated {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer template tokens (like `%{foo}`) over annotated tokens (like `%<foo>s`).".to_string(),
                    )];
                }
                if unannotated_count > max_unannotated {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer template tokens (like `%{foo}`) over unannotated tokens (like `%s`).".to_string(),
                    )];
                }
            }
            "unannotated" => {
                if has_annotated {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer unannotated tokens (like `%s`) over annotated tokens (like `%<foo>s`).".to_string(),
                    )];
                }
                if has_template {
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Prefer unannotated tokens (like `%s`) over template tokens (like `%{foo}`).".to_string(),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FormatStringToken, "cops/style/format_string_token");
}
