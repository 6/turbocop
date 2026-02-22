use crate::cop::node_type::{DEF_NODE, FORWARDING_SUPER_NODE, STATEMENTS_NODE, SUPER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantInitialize;

impl Cop for RedundantInitialize {
    fn name(&self) -> &'static str {
        "Style/RedundantInitialize"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, FORWARDING_SUPER_NODE, STATEMENTS_NODE, SUPER_NODE]
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
        let allow_comments = config.get_bool("AllowComments", true);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must be named `initialize`
        if def_node.name().as_slice() != b"initialize" {
            return;
        }

        // Must not have a receiver (not def self.initialize)
        if def_node.receiver().is_some() {
            return;
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => {
                // Empty initialize method — only redundant if no parameters
                if def_node.parameters().is_some() {
                    return;
                }
                if allow_comments {
                    // Check for comments inside the method
                    let def_start = def_node.location().start_offset();
                    let def_end = def_node.location().end_offset();
                    let body_bytes = &source.as_bytes()[def_start..def_end];
                    if has_comment_in_body(body_bytes) {
                        return;
                    }
                }
                let loc = def_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Remove unnecessary empty `initialize` method.".to_string(),
                ));
                return;
            }
        };

        // Check if the body is just a single `super` or `super(...)` call
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // Check for super call
        // ForwardingSuperNode = bare `super` (forwards all args)
        // SuperNode = super with explicit args `super(...)` or `super(a, b)`
        let is_forwarding_super = body_nodes[0].as_forwarding_super_node().is_some();
        let is_explicit_super = body_nodes[0].as_super_node().is_some();

        if !is_forwarding_super && !is_explicit_super {
            return;
        }

        // For bare `super`: only redundant if the method has no default args,
        // rest args, keyword args, or block args (simple required params only)
        if is_forwarding_super {
            if let Some(params) = def_node.parameters() {
                // Has optionals, rest, keywords, keyword_rest, or block
                if !params.optionals().is_empty()
                    || params.rest().is_some()
                    || !params.keywords().is_empty()
                    || params.keyword_rest().is_some()
                    || params.block().is_some()
                    || params.posts().iter().next().is_some()
                {
                    return;
                }
            }
        }

        // For explicit `super(...)`: only redundant if both the def and super have 0 args
        if is_explicit_super {
            if let Some(super_node) = body_nodes[0].as_super_node() {
                let super_has_args = super_node.arguments().is_some()
                    && super_node
                        .arguments()
                        .unwrap()
                        .arguments()
                        .iter()
                        .next()
                        .is_some();
                let def_has_params = def_node.parameters().is_some()
                    && def_node
                        .parameters()
                        .unwrap()
                        .requireds()
                        .iter()
                        .next()
                        .is_some();
                // super() is only redundant if the def also has no params
                if super_has_args || def_has_params {
                    return;
                }
            }
        }

        if allow_comments {
            let def_start = def_node.location().start_offset();
            let def_end = def_node.location().end_offset();
            let body_bytes = &source.as_bytes()[def_start..def_end];
            if has_comment_in_body(body_bytes) {
                return;
            }
        }

        let loc = def_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Remove unnecessary `initialize` method.".to_string(),
        ));
    }
}

fn has_comment_in_body(body_bytes: &[u8]) -> bool {
    // Skip the first line (def line) and check for comments
    let mut in_string = false;
    let mut first_line = true;
    for &b in body_bytes {
        if b == b'\n' {
            first_line = false;
            in_string = false;
            continue;
        }
        if first_line {
            continue;
        }
        if b == b'#' && !in_string {
            return true;
        }
        if b == b'"' || b == b'\'' {
            in_string = !in_string;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantInitialize, "cops/style/redundant_initialize");
}
