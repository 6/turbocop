use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct RelativeDateConstant;

const RELATIVE_TIME_METHODS: &[&[u8]] = &[b"today", b"now", b"current", b"yesterday", b"tomorrow"];
const TIME_CONSTANTS: &[&[u8]] = &[b"Date", b"Time", b"DateTime"];

impl Cop for RelativeDateConstant {
    fn name(&self) -> &'static str {
        "Rails/RelativeDateConstant"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let value = if let Some(cw) = node.as_constant_write_node() {
            cw.value()
        } else if let Some(cpw) = node.as_constant_path_write_node() {
            cpw.value()
        } else {
            return Vec::new();
        };

        // Check if the value contains a relative date/time call
        let mut finder = RelativeDateFinder { found: false };
        finder.visit(&value);

        if finder.found {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not assign relative dates to constants.".to_string(),
            )];
        }

        Vec::new()
    }
}

struct RelativeDateFinder {
    found: bool,
}

impl<'a> Visit<'a> for RelativeDateFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'a>) {
        if self.found {
            return;
        }

        let method_name = node.name().as_slice();
        if RELATIVE_TIME_METHODS.contains(&method_name) {
            if let Some(recv) = node.receiver() {
                if let Some(cr) = recv.as_constant_read_node() {
                    if TIME_CONSTANTS.contains(&cr.name().as_slice()) {
                        self.found = true;
                        return;
                    }
                }
            }
        }

        // Continue visiting children
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RelativeDateConstant, "cops/rails/relative_date_constant");
}
