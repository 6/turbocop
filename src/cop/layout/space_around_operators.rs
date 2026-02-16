use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct SpaceAroundOperators;

/// Collect byte offsets of `=` signs that are part of parameter defaults,
/// and byte ranges of operator method names in `def` statements.
struct ExclusionCollector {
    /// Byte offsets of `=` in default parameter positions.
    default_param_offsets: Vec<usize>,
    /// Byte ranges (start..end) of operator method names in `def` statements.
    /// e.g., `def ==(other)` — the `==` is a method name, not an operator.
    def_method_name_ranges: Vec<std::ops::Range<usize>>,
}

impl<'pr> Visit<'pr> for ExclusionCollector {
    fn visit_optional_parameter_node(&mut self, node: &ruby_prism::OptionalParameterNode<'pr>) {
        let op_loc = node.operator_loc();
        self.default_param_offsets.push(op_loc.start_offset());
    }

    fn visit_optional_keyword_parameter_node(
        &mut self,
        _node: &ruby_prism::OptionalKeywordParameterNode<'pr>,
    ) {
        // Keyword params use `:` not `=`, so nothing to exclude.
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let name = node.name().as_slice();
        // Check if the method name contains operator characters that this cop checks
        let is_operator_name = name.contains(&b'=') || name.contains(&b'!') || name.contains(&b'>');
        if is_operator_name {
            let loc = node.name_loc();
            self.def_method_name_ranges
                .push(loc.start_offset()..loc.end_offset());
        }
        // Recurse into the body to find nested defs and default params
        ruby_prism::visit_def_node(self, node);
    }
}

impl Cop for SpaceAroundOperators {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundOperators"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_for_alignment = config.get_bool("AllowForAlignment", true);
        let _enforced_style_exponent =
            config.get_str("EnforcedStyleForExponentOperator", "no_space");
        let _enforced_style_rational =
            config.get_str("EnforcedStyleForRationalLiterals", "no_space");

        // Collect default parameter `=` offsets and operator method name ranges
        let mut collector = ExclusionCollector {
            default_param_offsets: Vec::new(),
            def_method_name_ranges: Vec::new(),
        };
        collector.visit(&parse_result.node());
        let default_param_offsets = collector.default_param_offsets;
        let def_name_ranges = collector.def_method_name_ranges;

        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut diagnostics = Vec::new();
        let mut i = 0;

        // Suppress unused variable warning when alignment is not used
        let _ = allow_for_alignment;

        // Helper closure: check if offset `pos` falls within any operator method name range
        let in_def_name = |pos: usize| -> bool {
            def_name_ranges.iter().any(|r| r.contains(&pos))
        };

        while i < len {
            if !code_map.is_code(i) {
                i += 1;
                continue;
            }

            // Check for multi-char operators first: ==, !=, =>
            if i + 1 < len && code_map.is_code(i + 1) {
                let two = &bytes[i..i + 2];
                if two == b"==" || two == b"!=" || two == b"=>" {
                    // Skip ===
                    if two == b"==" && i + 2 < len && bytes[i + 2] == b'=' {
                        i += 3;
                        continue;
                    }

                    // Skip `=>` that is part of `<=>` (spaceship operator):
                    // if byte at i is `=` and i-1 is `<`, this is `<=>` not `=>`
                    if two == b"=>" && i > 0 && bytes[i - 1] == b'<' {
                        i += 2;
                        continue;
                    }

                    // Skip operator method names: `def ==(other)`, `def !=(other)`
                    if in_def_name(i) {
                        i += 2;
                        continue;
                    }

                    let op_str = std::str::from_utf8(two).unwrap_or("??");
                    let space_before = i > 0 && bytes[i - 1] == b' ';
                    let space_after = i + 2 < len && bytes[i + 2] == b' ';
                    let newline_after =
                        i + 2 >= len || bytes[i + 2] == b'\n' || bytes[i + 2] == b'\r';
                    if !space_before || (!space_after && !newline_after) {
                        let (line, column) = source.offset_to_line_col(i);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Surrounding space missing for operator `{op_str}`."),
                        ));
                    }
                    i += 2;
                    continue;
                }
            }

            // Single = (not ==, !=, =>, =~, <=, >=, or part of +=/-=/etc.)
            if bytes[i] == b'=' {
                // Skip =~ and =>
                if i + 1 < len && (bytes[i + 1] == b'~' || bytes[i + 1] == b'>') {
                    i += 2;
                    continue;
                }
                // Skip ==
                if i + 1 < len && bytes[i + 1] == b'=' {
                    i += 2;
                    continue;
                }
                // Skip if preceded by !, <, >, =, +, -, *, /, %, &, |, ^, ~
                if i > 0 {
                    let prev = bytes[i - 1];
                    if matches!(
                        prev,
                        b'!' | b'<'
                            | b'>'
                            | b'='
                            | b'+'
                            | b'-'
                            | b'*'
                            | b'/'
                            | b'%'
                            | b'&'
                            | b'|'
                            | b'^'
                            | b'~'
                    ) {
                        i += 1;
                        continue;
                    }
                }

                // Skip default parameter `=` signs (handled by SpaceAroundEqualsInParameterDefault)
                if default_param_offsets.contains(&i) {
                    i += 1;
                    continue;
                }

                // Skip `=` that is part of an operator method name: `def []=`, `def ===`
                if in_def_name(i) {
                    i += 1;
                    continue;
                }

                let space_before = i > 0 && bytes[i - 1] == b' ';
                let space_after = i + 1 < len && bytes[i + 1] == b' ';
                let newline_after =
                    i + 1 >= len || bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r';
                if !space_before || (!space_after && !newline_after) {
                    let (line, column) = source.offset_to_line_col(i);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Surrounding space missing for operator `=`.".to_string(),
                    ));
                }
                i += 1;
                continue;
            }

            i += 1;
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAroundOperators, "cops/layout/space_around_operators");
}
