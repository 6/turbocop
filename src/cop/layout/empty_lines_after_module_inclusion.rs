use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAfterModuleInclusion;

const MODULE_INCLUSION_METHODS: &[&[u8]] = &[b"include", b"extend", b"prepend"];

impl Cop for EmptyLinesAfterModuleInclusion {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAfterModuleInclusion"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a bare call (no receiver)
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if !MODULE_INCLUSION_METHODS.iter().any(|&m| m == method_name) {
            return Vec::new();
        }

        // Must have arguments (e.g., `include Foo`)
        if call.arguments().is_none() {
            return Vec::new();
        }

        let loc = call.location();
        let (last_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
        let lines: Vec<&[u8]> = source.lines().collect();

        // Check if the next line exists
        if last_line >= lines.len() {
            return Vec::new(); // End of file
        }

        let next_line = lines[last_line]; // next line (0-indexed)

        // If next line is blank, no offense
        if is_blank_line(next_line) {
            return Vec::new();
        }

        // If next line is end of class/module, no offense
        let next_trimmed: Vec<u8> = next_line
            .iter()
            .copied()
            .skip_while(|&b| b == b' ' || b == b'\t')
            .collect();
        if next_trimmed.starts_with(b"end") {
            let after_end = next_trimmed.get(3);
            if after_end.is_none()
                || matches!(
                    after_end,
                    Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'#')
                )
            {
                return Vec::new();
            }
        }

        // If next line is another module inclusion, no offense
        for &method in MODULE_INCLUSION_METHODS {
            if next_trimmed.starts_with(method) {
                let after = next_trimmed.get(method.len());
                if after.is_none() || matches!(after, Some(b' ') | Some(b'(')) {
                    return Vec::new();
                }
            }
        }

        let (line, col) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            col,
            "Add an empty line after module inclusion.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAfterModuleInclusion,
        "cops/layout/empty_lines_after_module_inclusion"
    );
}
