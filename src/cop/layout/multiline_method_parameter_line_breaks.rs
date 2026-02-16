use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodParameterLineBreaks;

impl Cop for MultilineMethodParameterLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodParameterLineBreaks"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let lparen_loc = match def_node.lparen_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let rparen_loc = match def_node.rparen_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let (open_line, _) = source.offset_to_line_col(lparen_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(rparen_loc.start_offset());

        // Only check multiline parameter lists
        if open_line == close_line {
            return Vec::new();
        }

        // Collect all parameter locations
        let mut param_locs: Vec<(usize, usize)> = Vec::new(); // (start_offset, end_offset)

        for p in params.requireds().iter() {
            let loc = p.location();
            param_locs.push((loc.start_offset(), loc.end_offset()));
        }
        for p in params.optionals().iter() {
            let loc = p.location();
            param_locs.push((loc.start_offset(), loc.end_offset()));
        }
        if let Some(rest) = params.rest() {
            let loc = rest.location();
            param_locs.push((loc.start_offset(), loc.end_offset()));
        }
        for p in params.keywords().iter() {
            let loc = p.location();
            param_locs.push((loc.start_offset(), loc.end_offset()));
        }
        if let Some(kw_rest) = params.keyword_rest() {
            let loc = kw_rest.location();
            param_locs.push((loc.start_offset(), loc.end_offset()));
        }
        if let Some(block_param) = params.block() {
            let loc = block_param.location();
            param_locs.push((loc.start_offset(), loc.end_offset()));
        }

        // Sort by start offset
        param_locs.sort_by_key(|&(start, _)| start);

        if param_locs.len() < 2 {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        for i in 1..param_locs.len() {
            let (_, prev_end) = param_locs[i - 1];
            let (curr_start, _) = param_locs[i];

            let (prev_line, _) = source.offset_to_line_col(prev_end.saturating_sub(1));
            let (curr_line, curr_col) = source.offset_to_line_col(curr_start);

            if prev_line == curr_line {
                diagnostics.push(self.diagnostic(
                    source,
                    curr_line,
                    curr_col,
                    "Each parameter in a multi-line method definition must start on a separate line."
                        .to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineMethodParameterLineBreaks,
        "cops/layout/multiline_method_parameter_line_breaks"
    );
}
