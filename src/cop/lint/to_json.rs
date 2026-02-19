use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct ToJSON;

impl Cop for ToJSON {
    fn name(&self) -> &'static str {
        "Lint/ToJSON"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        if def_node.name().as_slice() != b"to_json" {
            return Vec::new();
        }

        // Check if the method has no parameters
        let params = def_node.parameters();
        let has_params = match params {
            Some(p) => {
                // Check if there are any parameters at all
                let has_requireds = !p.requireds().is_empty();
                let has_optionals = !p.optionals().is_empty();
                let has_rest = p.rest().is_some();
                let has_keywords = !p.keywords().is_empty();
                let has_keyword_rest = p.keyword_rest().is_some();
                let has_block = p.block().is_some();
                let has_posts = !p.posts().is_empty();

                has_requireds
                    || has_optionals
                    || has_rest
                    || has_keywords
                    || has_keyword_rest
                    || has_block
                    || has_posts
            }
            None => false,
        };

        if has_params {
            return Vec::new();
        }

        let loc = def_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "`#to_json` requires an optional argument to be parsable via JSON.generate(obj)."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ToJSON, "cops/lint/to_json");
}
