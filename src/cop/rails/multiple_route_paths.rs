use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MultipleRoutePaths;

const HTTP_METHODS: &[&[u8]] = &[b"get", b"post", b"put", b"patch", b"delete"];

impl Cop for MultipleRoutePaths {
    fn name(&self) -> &'static str {
        "Rails/MultipleRoutePaths"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/config/routes.rb", "**/config/routes/**/*.rb"]
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be receiverless HTTP method
        if call.receiver().is_some() {
            return;
        }

        let name = call.name().as_slice();
        if !HTTP_METHODS.contains(&name) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Count non-hash arguments (route paths)
        let mut path_count = 0;
        for arg in args.arguments().iter() {
            if arg.as_hash_node().is_none()
                && arg.as_keyword_hash_node().is_none()
                && arg.as_array_node().is_none()
            {
                path_count += 1;
            }
        }

        if path_count < 2 {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(
            self.diagnostic(
                source,
                line,
                column,
                "Use separate routes instead of combining multiple route paths in a single route."
                    .to_string(),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleRoutePaths, "cops/rails/multiple_route_paths");
}
