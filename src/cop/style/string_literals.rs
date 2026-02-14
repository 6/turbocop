use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct StringLiterals;

impl Cop for StringLiterals {
    fn name(&self) -> &'static str {
        "Style/StringLiterals"
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

        let opening = match string_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let opening_byte = opening.as_slice().first().copied().unwrap_or(0);

        // Skip %q, %Q, heredocs, ? prefix
        if matches!(opening_byte, b'%' | b'<' | b'?') {
            return Vec::new();
        }

        let enforced_style = config
            .options
            .get("EnforcedStyle")
            .and_then(|v| v.as_str())
            .unwrap_or("single_quotes");

        let content = string_node.content_loc().as_slice();

        match enforced_style {
            "single_quotes" => {
                if opening_byte == b'"' {
                    // Check if single quotes can be used:
                    // - No single quotes in content
                    // - No escape sequences (no backslash in content)
                    if !content.contains(&b'\'') && !content.contains(&b'\\') {
                        let (line, column) = source.offset_to_line_col(opening.start_offset());
                        return vec![Diagnostic {
                            path: source.path_str().to_string(),
                            location: Location { line, column },
                            severity: Severity::Convention,
                            cop_name: self.name().to_string(),
                            message: "Prefer single-quoted strings when you don't need string interpolation or special symbols.".to_string(),
                        }];
                    }
                }
            }
            "double_quotes" => {
                if opening_byte == b'\'' {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    return vec![Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line, column },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Prefer double-quoted strings unless you need single quotes within your string.".to_string(),
                    }];
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
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &StringLiterals,
            include_bytes!("../../../testdata/cops/style/string_literals/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &StringLiterals,
            include_bytes!("../../../testdata/cops/style/string_literals/no_offense.rb"),
        );
    }
}
