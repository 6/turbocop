use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, DEF_NODE, STATEMENTS_NODE};

pub struct RedundantBegin;

impl Cop for RedundantBegin {
    fn name(&self) -> &'static str {
        "Style/RedundantBegin"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, DEF_NODE, STATEMENTS_NODE]
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

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // The body might be a BeginNode directly or a StatementsNode containing
        // a single BeginNode
        let begin_node = if let Some(b) = body.as_begin_node() {
            b
        } else if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                return Vec::new();
            }
            match body_nodes[0].as_begin_node() {
                Some(b) => b,
                None => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        // The begin is redundant if it's the only statement in the method body
        let begin_kw_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let (line, column) = source.offset_to_line_col(begin_kw_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBegin, "cops/style/redundant_begin");
}
