use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE};

pub struct ClassMethodsDefinitions;

impl Cop for ClassMethodsDefinitions {
    fn name(&self) -> &'static str {
        "Style/ClassMethodsDefinitions"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "def_self");

        if enforced_style == "def_self" {
            // Check for `class << self` with public methods
            if let Some(sclass) = node.as_singleton_class_node() {
                let expr = sclass.expression();
                if expr.as_self_node().is_some() {
                    // Check if body has public def nodes
                    if let Some(body) = sclass.body() {
                        if has_public_defs(&body) {
                            let loc = sclass.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Do not define public methods within class << self.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

    }
}

fn has_public_defs(body: &ruby_prism::Node<'_>) -> bool {
    if let Some(stmts) = body.as_statements_node() {
        for stmt in stmts.body().iter() {
            if stmt.as_def_node().is_some() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassMethodsDefinitions, "cops/style/class_methods_definitions");
}
