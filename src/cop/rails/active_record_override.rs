use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use crate::cop::node_type::DEF_NODE;

pub struct ActiveRecordOverride;

const BAD_METHODS: &[&[u8]] = &[b"create", b"destroy", b"save", b"update"];

/// Visitor to check if a node contains `super` (ForwardingSuperNode or SuperNode)
struct HasSuperVisitor {
    found: bool,
}

impl<'pr> Visit<'pr> for HasSuperVisitor {
    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.found = true;
    }

    fn visit_super_node(&mut self, _node: &ruby_prism::SuperNode<'pr>) {
        self.found = true;
    }
}

impl Cop for ActiveRecordOverride {
    fn name(&self) -> &'static str {
        "Rails/ActiveRecordOverride"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let method_name = def_node.name().as_slice();
        if !BAD_METHODS.iter().any(|m| *m == method_name) {
            return;
        }

        // Must not be a class method (def self.save)
        if def_node.receiver().is_some() {
            return;
        }

        // Check for `super` call in body
        let mut visitor = HasSuperVisitor { found: false };
        if let Some(body) = def_node.body() {
            visitor.visit(&body);
        }
        if !visitor.found {
            return;
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("?");
        let callbacks = format!(
            "`before_{method_str}`, `around_{method_str}`, or `after_{method_str}`"
        );

        let loc = def_node.name_loc();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use {callbacks} callbacks instead of overriding the Active Record method `{method_str}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveRecordOverride, "cops/rails/active_record_override");
}
