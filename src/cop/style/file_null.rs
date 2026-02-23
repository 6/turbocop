use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct FileNull;

impl Cop for FileNull {
    fn name(&self) -> &'static str {
        "Style/FileNull"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // First pass: check if the file contains any "/dev/null" string
        // (needed for bare "NUL" detection)
        let root = parse_result.node();
        let mut dev_null_finder = DevNullFinder { found: false };
        dev_null_finder.visit(&root);
        let contain_dev_null = dev_null_finder.found;

        // Second pass: find offenses
        let mut visitor = FileNullVisitor {
            source,
            cop: self,
            diagnostics: Vec::new(),
            contain_dev_null,
            in_array_or_pair: false,
        };
        visitor.visit(&root);
        diagnostics.extend(visitor.diagnostics);
    }
}

struct DevNullFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for DevNullFinder {
    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        let content = node.unescaped();
        if let Ok(s) = std::str::from_utf8(content) {
            if s.eq_ignore_ascii_case("/dev/null") {
                self.found = true;
            }
        }
    }
}

struct FileNullVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a FileNull,
    diagnostics: Vec<Diagnostic>,
    contain_dev_null: bool,
    in_array_or_pair: bool,
}

impl<'a, 'pr> Visit<'pr> for FileNullVisitor<'a> {
    fn visit_array_node(&mut self, node: &ruby_prism::ArrayNode<'pr>) {
        let prev = self.in_array_or_pair;
        self.in_array_or_pair = true;
        ruby_prism::visit_array_node(self, node);
        self.in_array_or_pair = prev;
    }

    fn visit_assoc_node(&mut self, node: &ruby_prism::AssocNode<'pr>) {
        let prev = self.in_array_or_pair;
        self.in_array_or_pair = true;
        ruby_prism::visit_assoc_node(self, node);
        self.in_array_or_pair = prev;
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        // Skip strings inside arrays or hash pairs
        if self.in_array_or_pair {
            return;
        }

        let content_bytes = node.unescaped();
        let content_str = match std::str::from_utf8(content_bytes) {
            Ok(s) => s,
            Err(_) => return,
        };

        if content_str.is_empty() {
            return;
        }

        let lower = content_str.to_lowercase();

        let matched =
            if lower == "/dev/null" || lower == "nul:" || (lower == "nul" && self.contain_dev_null)
            {
                Some(content_str)
            } else {
                None
            };

        if let Some(matched_str) = matched {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Use `File::NULL` instead of `{}`.", matched_str),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FileNull, "cops/style/file_null");
}
