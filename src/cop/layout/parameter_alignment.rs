use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ParameterAlignment;

impl Cop for ParameterAlignment {
    fn name(&self) -> &'static str {
        "Layout/ParameterAlignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "with_first_parameter");
        let _indent_width = config.get_usize("IndentationWidth", 2);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let requireds: Vec<_> = params.requireds().iter().collect();
        let optionals: Vec<_> = params.optionals().iter().map(ruby_prism::Node::from).collect();
        let mut all_params: Vec<ruby_prism::Node<'_>> = Vec::new();
        all_params.extend(requireds);
        all_params.extend(optionals);
        if let Some(rest) = params.rest() {
            all_params.push(ruby_prism::Node::from(rest));
        }
        for kw in params.keywords().iter() {
            all_params.push(ruby_prism::Node::from(kw));
        }

        if all_params.len() < 2 {
            return Vec::new();
        }

        let first_param = &all_params[0];
        let (first_line, first_col) = source.offset_to_line_col(first_param.location().start_offset());

        let mut diagnostics = Vec::new();

        let base_col = match style {
            "with_fixed_indentation" => {
                let def_keyword_loc = def_node.def_keyword_loc();
                let (def_line, _) = source.offset_to_line_col(def_keyword_loc.start_offset());
                let def_line_bytes = util::line_at(source, def_line).unwrap_or(b"");
                util::indentation_of(def_line_bytes) + 2
            }
            _ => first_col, // with_first_parameter
        };

        for param in all_params.iter().skip(1) {
            let (param_line, param_col) = source.offset_to_line_col(param.location().start_offset());
            if param_line == first_line {
                continue; // Same line as first param, skip
            }
            if param_col != base_col {
                let msg = if style == "with_fixed_indentation" {
                    "Use one level of indentation for parameters following the first line of a multi-line method definition."
                } else {
                    "Align the parameters of a method definition if they span more than one line."
                };
                diagnostics.push(self.diagnostic(
                    source,
                    param_line,
                    param_col,
                    msg.to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ParameterAlignment, "cops/layout/parameter_alignment");
}
