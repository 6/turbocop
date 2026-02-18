use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FormatString;

impl Cop for FormatString {
    fn name(&self) -> &'static str {
        "Style/FormatString"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_bytes = call.name().as_slice();
        let style = config.get_str("EnforcedStyle", "format");

        match method_bytes {
            b"%" => {
                // String#% - only flag when style prefers format or sprintf
                if style == "percent" {
                    return Vec::new();
                }
                // Must have a string receiver
                let receiver = match call.receiver() {
                    Some(r) => r,
                    None => return Vec::new(),
                };
                if receiver.as_string_node().is_none()
                    && receiver.as_interpolated_string_node().is_none()
                {
                    return Vec::new();
                }

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "format" { "format" } else { "sprintf" };
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `String#%`.", preferred),
                )];
            }
            b"format" => {
                if style == "format" {
                    return Vec::new();
                }
                // Only flag top-level or Kernel.format
                if let Some(recv) = call.receiver() {
                    if recv.as_constant_read_node().map_or(true, |c| c.name().as_slice() != b"Kernel") {
                        return Vec::new();
                    }
                }

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "sprintf" { "sprintf" } else { "String#%" };
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `format`.", preferred),
                )];
            }
            b"sprintf" => {
                if style == "sprintf" {
                    return Vec::new();
                }
                // Only flag top-level or Kernel.sprintf
                if let Some(recv) = call.receiver() {
                    if recv.as_constant_read_node().map_or(true, |c| c.name().as_slice() != b"Kernel") {
                        return Vec::new();
                    }
                }

                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let preferred = if style == "format" { "format" } else { "String#%" };
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Favor `{}` over `sprintf`.", preferred),
                )];
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FormatString, "cops/style/format_string");
}
