use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UniqBeforePluck;

impl Cop for UniqBeforePluck {
    fn name(&self) -> &'static str {
        "Rails/UniqBeforePluck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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
        let chain = match util::as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        // outer is `uniq`, inner is `pluck`
        if chain.outer_method != b"uniq" || chain.inner_method != b"pluck" {
            return;
        }

        let style = config.get_str("EnforcedStyle", "conservative");

        // In conservative mode, only flag if pluck's receiver is a constant (model class)
        if style == "conservative" {
            let pluck_receiver = chain.inner_call.receiver();
            let is_const = match pluck_receiver {
                Some(ref r) => {
                    r.as_constant_read_node().is_some() || r.as_constant_path_node().is_some()
                }
                None => false,
            };
            if !is_const {
                return;
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `distinct` before `pluck`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UniqBeforePluck, "cops/rails/uniq_before_pluck");
}
