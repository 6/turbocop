use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceAfterMethodName;

impl Cop for SpaceAfterMethodName {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterMethodName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must have parenthesized parameters
        let lparen = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Check if there's a space between the method name and the opening paren
        let name_loc = def_node.name_loc();
        let name_end = name_loc.end_offset();
        let lparen_start = lparen.start_offset();

        if lparen_start > name_end {
            let between = &source.as_bytes()[name_end..lparen_start];
            if between.iter().any(|&b| b == b' ' || b == b'\t') {
                let (line, column) = source.offset_to_line_col(name_end);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not put a space between a method name and the opening parenthesis."
                        .to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: name_end,
                        end: lparen_start,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterMethodName, "cops/layout/space_after_method_name");
    crate::cop_autocorrect_fixture_tests!(
        SpaceAfterMethodName,
        "cops/layout/space_after_method_name"
    );
}
