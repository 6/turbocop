use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct MultilineMethodDefinitionBraceLayout;

impl Cop for MultilineMethodDefinitionBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodDefinitionBraceLayout"
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
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must have explicit parentheses
        let lparen_loc = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return,
        };
        let rparen_loc = match def_node.rparen_loc() {
            Some(loc) => loc,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let (open_line, _) = source.offset_to_line_col(lparen_loc.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(rparen_loc.start_offset());

        // Only check multiline parameter lists
        if open_line == close_line {
            return;
        }

        // Find the first and last parameter locations
        let mut first_offset: Option<usize> = None;
        let mut last_end_offset: Option<usize> = None;

        // Collect all parameter locations from requireds (Node type)
        let requireds: Vec<ruby_prism::Node<'_>> = params.requireds().iter().collect();
        for p in &requireds {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Optionals
        let optionals: Vec<ruby_prism::Node<'_>> =
            params.optionals().iter().collect();
        for p in &optionals {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Rest
        if let Some(p) = params.rest() {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Posts
        let posts: Vec<ruby_prism::Node<'_>> = params.posts().iter().collect();
        for p in &posts {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Keywords
        let keywords: Vec<ruby_prism::Node<'_>> = params.keywords().iter().collect();
        for p in &keywords {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Keyword rest
        if let Some(p) = params.keyword_rest() {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        // Block parameter
        if let Some(p) = params.block() {
            let start = p.location().start_offset();
            let end = p.location().end_offset();
            if first_offset.is_none() || start < first_offset.unwrap() {
                first_offset = Some(start);
            }
            if last_end_offset.is_none() || end > last_end_offset.unwrap() {
                last_end_offset = Some(end);
            }
        }

        let first_off = match first_offset {
            Some(o) => o,
            None => return,
        };
        let last_end = match last_end_offset {
            Some(o) => o,
            None => return,
        };

        let (first_param_line, _) = source.offset_to_line_col(first_off);
        let (last_param_line, _) = source.offset_to_line_col(last_end.saturating_sub(1));

        let open_same_as_first = open_line == first_param_line;
        let close_same_as_last = close_line == last_param_line;

        match enforced_style {
            "symmetrical" => {
                if open_same_as_first && !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method definition brace must be on the same line as the last parameter when opening brace is on the same line as the first parameter.".to_string(),
                    ));
                }
                if !open_same_as_first && close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method definition brace must be on the line after the last parameter when opening brace is on a separate line from the first parameter.".to_string(),
                    ));
                }
            }
            "new_line" => {
                if close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method definition brace must be on the line after the last parameter."
                            .to_string(),
                    ));
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "Closing method definition brace must be on the same line as the last parameter."
                            .to_string(),
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

    crate::cop_fixture_tests!(
        MultilineMethodDefinitionBraceLayout,
        "cops/layout/multiline_method_definition_brace_layout"
    );
}
