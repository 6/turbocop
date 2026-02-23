use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

pub struct FormatStringToken;

impl FormatStringToken {
    /// Check for annotated tokens like %<name>s
    /// Requires a complete %<word_chars>type pattern matching RuboCop's NAME regex.
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
                    // Must have at least one word character followed by closing >
                    let mut j = i + 2;
                    let mut has_word_char = false;
                    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        has_word_char = true;
                        j += 1;
                    }
                    if has_word_char && j < bytes.len() && bytes[j] == b'>' {
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }

    /// Check for template tokens like %{name}
    /// Requires a complete %{word_chars} pattern matching RuboCop's TEMPLATE_NAME regex.
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
                    // Must have at least one word character followed by closing }
                    let mut j = i + 2;
                    let mut has_word_char = false;
                    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        has_word_char = true;
                        j += 1;
                    }
                    if has_word_char && j < bytes.len() && bytes[j] == b'}' {
                        return true;
                    }
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
            if bytes[i] == b'%' && i + 1 < bytes.len() {
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
                while j < bytes.len()
                    && (bytes[j] == b'-'
                        || bytes[j] == b'+'
                        || bytes[j] == b' '
                        || bytes[j] == b'0'
                        || bytes[j] == b'#'
                        || bytes[j].is_ascii_digit()
                        || bytes[j] == b'.'
                        || bytes[j] == b'*')
                {
                    j += 1;
                }
                if j < bytes.len()
                    && matches!(
                        bytes[j],
                        b's' | b'd'
                            | b'f'
                            | b'g'
                            | b'e'
                            | b'x'
                            | b'X'
                            | b'o'
                            | b'b'
                            | b'B'
                            | b'i'
                            | b'u'
                            | b'c'
                            | b'p'
                            | b'a'
                            | b'A'
                            | b'E'
                            | b'G'
                    )
                {
                    count += 1;
                }
            }
            i += 1;
        }
        count
    }

    /// Check whether the string has ONLY unannotated tokens (no template or annotated)
    fn only_unannotated_tokens(s: &str) -> bool {
        !Self::has_annotated_token(s)
            && !Self::has_template_token(s)
            && Self::count_unannotated_tokens(s) > 0
    }
}

impl Cop for FormatStringToken {
    fn name(&self) -> &'static str {
        "Style/FormatStringToken"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "annotated");
        let max_unannotated = config.get_usize("MaxUnannotatedPlaceholdersAllowed", 1);
        let mode = config.get_str("Mode", "aggressive");
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let mut visitor = FormatStringTokenVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            style: style.to_string(),
            max_unannotated,
            conservative: mode == "conservative",
            allowed_methods,
            allowed_patterns,
            format_context_offsets: HashSet::new(),
            allowed_method_string_offsets: HashSet::new(),
        };

        // First pass: collect offsets of strings in format contexts and allowed method contexts
        let mut collector = FormatContextCollector {
            source,
            format_context_offsets: &mut visitor.format_context_offsets,
            allowed_method_string_offsets: &mut visitor.allowed_method_string_offsets,
            allowed_methods: &visitor.allowed_methods,
            allowed_patterns: &visitor.allowed_patterns,
        };
        collector.visit(&parse_result.node());

        // Second pass: check strings
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Collects start offsets of string nodes that are in a format context
/// (first arg to format/sprintf/printf, or LHS of %).
struct FormatContextCollector<'a> {
    source: &'a SourceFile,
    format_context_offsets: &'a mut HashSet<usize>,
    allowed_method_string_offsets: &'a mut HashSet<usize>,
    allowed_methods: &'a Option<Vec<String>>,
    allowed_patterns: &'a Option<Vec<String>>,
}

impl FormatContextCollector<'_> {
    fn is_allowed_method(&self, method_name: &str) -> bool {
        if let Some(methods) = self.allowed_methods {
            if methods.iter().any(|m| m == method_name) {
                return true;
            }
        }
        if let Some(patterns) = self.allowed_patterns {
            for pat in patterns {
                if method_name.contains(pat.as_str()) {
                    return true;
                }
            }
        }
        false
    }

    /// Collect start offsets of all string/interpolated-string nodes in a subtree
    fn collect_string_offsets(node: &ruby_prism::Node<'_>, offsets: &mut HashSet<usize>) {
        if node.as_string_node().is_some() || node.as_interpolated_string_node().is_some() {
            offsets.insert(node.location().start_offset());
        }
        // Also check inside interpolated strings for their parts
        struct StringCollector<'a> {
            offsets: &'a mut HashSet<usize>,
        }
        impl<'pr> Visit<'pr> for StringCollector<'_> {
            fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
                self.offsets.insert(node.location().start_offset());
                ruby_prism::visit_string_node(self, node);
            }
            fn visit_interpolated_string_node(
                &mut self,
                node: &ruby_prism::InterpolatedStringNode<'pr>,
            ) {
                self.offsets.insert(node.location().start_offset());
                ruby_prism::visit_interpolated_string_node(self, node);
            }
        }
        let mut sc = StringCollector { offsets };
        sc.visit(node);
    }
}

impl<'pr> Visit<'pr> for FormatContextCollector<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name();
        let method_name = std::str::from_utf8(name.as_slice()).unwrap_or("");

        // Check if this is a format method: format, sprintf, printf
        if matches!(method_name, "format" | "sprintf" | "printf") {
            // The first argument is the format string
            if let Some(args) = node.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if !arg_list.is_empty() {
                    Self::collect_string_offsets(&arg_list[0], self.format_context_offsets);
                }
            }
        }

        // Check if this is the % operator: "string" % args
        if method_name == "%" {
            if let Some(receiver) = node.receiver() {
                Self::collect_string_offsets(&receiver, self.format_context_offsets);
            }
        }

        // Check if any ancestor method is in AllowedMethods
        if self.is_allowed_method(method_name) {
            // All string arguments to this method should be suppressed
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    Self::collect_string_offsets(&arg, self.allowed_method_string_offsets);
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

struct FormatStringTokenVisitor<'a> {
    cop: &'a FormatStringToken,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    style: String,
    max_unannotated: usize,
    conservative: bool,
    allowed_methods: Option<Vec<String>>,
    allowed_patterns: Option<Vec<String>>,
    /// Offsets of strings that are in a format context (format/sprintf/printf/%)
    format_context_offsets: HashSet<usize>,
    /// Offsets of strings that are args to allowed methods
    allowed_method_string_offsets: HashSet<usize>,
}

impl<'pr> Visit<'pr> for FormatStringTokenVisitor<'_> {
    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        let content_bytes = node.unescaped();
        let content_str = match std::str::from_utf8(content_bytes) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Skip strings inside regexp or xstr (handled by format_string_token? in RuboCop)
        if !content_str.contains('%') {
            return;
        }

        let offset = node.location().start_offset();

        // Skip if this string is an argument to an allowed method
        if self.allowed_method_string_offsets.contains(&offset) {
            return;
        }

        let has_annotated = FormatStringToken::has_annotated_token(content_str);
        let has_template = FormatStringToken::has_template_token(content_str);
        let unannotated_count = FormatStringToken::count_unannotated_tokens(content_str);

        let in_format_context = self.format_context_offsets.contains(&offset);

        // Per RuboCop: unannotated tokens are always treated conservatively.
        // They are only flagged when the string is in a format context.
        // In conservative mode, ALL token types are only flagged in format context.
        let check_unannotated = in_format_context;
        let check_named = if self.conservative {
            in_format_context
        } else {
            true
        };

        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());

        match self.style.as_str() {
            "annotated" => {
                if has_template && check_named {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).".to_string(),
                    ));
                    return;
                }
                if unannotated_count > self.max_unannotated && check_unannotated {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).".to_string(),
                    ));
                }
            }
            "template" => {
                if has_annotated && check_named {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer template tokens (like `%{foo}`) over annotated tokens (like `%<foo>s`).".to_string(),
                    ));
                    return;
                }
                if unannotated_count > self.max_unannotated && check_unannotated {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer template tokens (like `%{foo}`) over unannotated tokens (like `%s`).".to_string(),
                    ));
                }
            }
            "unannotated" => {
                if has_annotated && check_named {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer unannotated tokens (like `%s`) over annotated tokens (like `%<foo>s`).".to_string(),
                    ));
                    return;
                }
                if has_template && check_named {
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer unannotated tokens (like `%s`) over template tokens (like `%{foo}`).".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FormatStringToken, "cops/style/format_string_token");
}
