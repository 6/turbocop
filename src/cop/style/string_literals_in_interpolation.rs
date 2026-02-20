use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StringLiteralsInInterpolation;

impl Cop for StringLiteralsInInterpolation {
    fn name(&self) -> &'static str {
        "Style/StringLiteralsInInterpolation"
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
        let enforced_style = config.get_str("EnforcedStyle", "single_quotes").to_string();

        let mut visitor = InterpStringVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            enforced_style,
            in_interpolation: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InterpStringVisitor<'a> {
    cop: &'a StringLiteralsInInterpolation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    enforced_style: String,
    in_interpolation: bool,
}

impl<'pr> Visit<'pr> for InterpStringVisitor<'_> {
    fn visit_embedded_statements_node(&mut self, node: &ruby_prism::EmbeddedStatementsNode<'pr>) {
        let was = self.in_interpolation;
        self.in_interpolation = true;
        ruby_prism::visit_embedded_statements_node(self, node);
        self.in_interpolation = was;
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if !self.in_interpolation {
            return;
        }

        let opening = match node.opening_loc() {
            Some(o) => o,
            None => return,
        };

        let open_bytes = opening.as_slice();

        // Check if the string uses the wrong quote style
        match self.enforced_style.as_str() {
            "single_quotes" => {
                if open_bytes == b"\"" {
                    // Check if it needs double quotes (has escape sequences)
                    let content = node.content_loc().as_slice();
                    if !needs_double_quotes(content) {
                        let loc = node.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Prefer single-quoted strings inside interpolations.".to_string(),
                        ));
                    }
                }
            }
            "double_quotes" => {
                if open_bytes == b"'" {
                    let loc = node.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer double-quoted strings inside interpolations.".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

fn needs_double_quotes(content: &[u8]) -> bool {
    let mut i = 0;
    while i < content.len() {
        // If the content contains a single quote, it can't use single-quoted style
        if content[i] == b'\'' {
            return true;
        }
        if content[i] == b'\\' && i + 1 < content.len() {
            match content[i + 1] {
                b'n' | b't' | b'r' | b'\\' | b'"' | b'0' | b'a' | b'b' | b'e' | b'f' | b's' | b'v' => return true,
                b'x' | b'u' => return true,
                _ => {}
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringLiteralsInInterpolation, "cops/style/string_literals_in_interpolation");
}
