use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct EndlessMethod;

impl Cop for EndlessMethod {
    fn name(&self) -> &'static str {
        "Style/EndlessMethod"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let style = config.get_str("EnforcedStyle", "allow_single_line");

        // Check if this is an endless method (has = sign, no end keyword)
        let is_endless = def_node.end_keyword_loc().is_none()
            && def_node.equal_loc().is_some();

        match style {
            "disallow" => {
                if is_endless {
                    let loc = def_node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid endless method definitions.".to_string(),
                    )];
                }
            }
            "allow_single_line" => {
                if is_endless {
                    let loc = def_node.location();
                    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
                    let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                    if end_line > start_line {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid endless method definitions with multiple lines.".to_string(),
                        )];
                    }
                }
            }
            "allow_always" => {
                // No offenses for endless methods
            }
            "require_single_line" | "require_always" => {
                // These styles want endless methods to be used
                // We skip enforcement of "use endless" to keep this simple
                // and focus on the "avoid" cases
                if is_endless {
                    let loc = def_node.location();
                    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
                    let (end_line, _) = source.offset_to_line_col(loc.end_offset());
                    if end_line > start_line && style == "require_single_line" {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid endless method definitions with multiple lines.".to_string(),
                        )];
                    }
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
    crate::cop_fixture_tests!(EndlessMethod, "cops/style/endless_method");
}
