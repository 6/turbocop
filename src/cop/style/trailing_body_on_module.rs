use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::MODULE_NODE;

pub struct TrailingBodyOnModule;

impl Cop for TrailingBodyOnModule {
    fn name(&self) -> &'static str {
        "Style/TrailingBodyOnModule"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[MODULE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return Vec::new(),
        };

        let body = match module_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Check if module keyword line equals body start line
        let mod_loc = module_node.constant_path().location();
        let (mod_line, _) = source.offset_to_line_col(mod_loc.start_offset());
        let body_loc = body.location();
        let (body_line, body_column) = source.offset_to_line_col(body_loc.start_offset());

        if mod_line == body_line {
            return vec![self.diagnostic(
                source,
                body_line,
                body_column,
                "Place the first line of module body on its own line.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingBodyOnModule, "cops/style/trailing_body_on_module");
}
