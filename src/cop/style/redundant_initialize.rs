use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantInitialize;

impl Cop for RedundantInitialize {
    fn name(&self) -> &'static str {
        "Style/RedundantInitialize"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_comments = config.get_bool("AllowComments", true);

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Must be named `initialize`
        if def_node.name().as_slice() != b"initialize" {
            return Vec::new();
        }

        // Must not have a receiver (not def self.initialize)
        if def_node.receiver().is_some() {
            return Vec::new();
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => {
                // Empty initialize method
                if allow_comments {
                    // Check for comments inside the method
                    let def_start = def_node.location().start_offset();
                    let def_end = def_node.location().end_offset();
                    let body_bytes = &source.as_bytes()[def_start..def_end];
                    if has_comment_in_body(body_bytes) {
                        return Vec::new();
                    }
                }
                let loc = def_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Remove unnecessary empty `initialize` method.".to_string(),
                )];
            }
        };

        // Check if the body is just a single `super` or `super(...)` call
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        // Check for super call
        // ForwardingSuperNode = bare `super` (forwards all args)
        // SuperNode = super with explicit args `super(...)` or `super(a, b)`
        let is_forwarding_super = body_nodes[0].as_forwarding_super_node().is_some();
        let is_explicit_super = body_nodes[0].as_super_node().is_some();

        if !is_forwarding_super && !is_explicit_super {
            return Vec::new();
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
                    return Vec::new();
                }
            }
        }

        // For explicit `super(...)`: only redundant if both the def and super have 0 args
        if is_explicit_super {
            if let Some(super_node) = body_nodes[0].as_super_node() {
                let super_has_args = super_node.arguments().is_some()
                    && super_node.arguments().unwrap().arguments().iter().next().is_some();
                let def_has_params = def_node.parameters().is_some()
                    && def_node.parameters().unwrap().requireds().iter().next().is_some();
                // super() is only redundant if the def also has no params
                if super_has_args || def_has_params {
                    return Vec::new();
                }
            }
        }

        if allow_comments {
            let def_start = def_node.location().start_offset();
            let def_end = def_node.location().end_offset();
            let body_bytes = &source.as_bytes()[def_start..def_end];
            if has_comment_in_body(body_bytes) {
                return Vec::new();
            }
        }

        let loc = def_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Remove unnecessary `initialize` method.".to_string(),
        )]
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
