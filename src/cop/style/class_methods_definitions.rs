use crate::cop::node_type::{DEF_NODE, SELF_NODE, SINGLETON_CLASS_NODE, STATEMENTS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
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
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    let mut in_private = false;
    for stmt in stmts.body().iter() {
        // Check for access modifier calls (private, protected, public)
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if call.receiver().is_none() {
                if call.arguments().is_none() {
                    // Standalone modifier: `private` / `protected` / `public`
                    if name == b"private" || name == b"protected" {
                        in_private = true;
                        continue;
                    }
                    if name == b"public" {
                        in_private = false;
                        continue;
                    }
                } else if name == b"private" || name == b"protected" {
                    // Inline modifier: `private def foo` / `protected def bar`
                    // The def is an argument to the call, so it's not a public def.
                    continue;
                } else if name == b"public" {
                    // `public def foo` — the def is public, but it's inside the
                    // arguments, not a standalone def_node. We need to check if
                    // the argument is a def.
                    if let Some(args) = call.arguments() {
                        for arg in args.arguments().iter() {
                            if arg.as_def_node().is_some() {
                                return true;
                            }
                        }
                    }
                    continue;
                }
            }
        }

        if stmt.as_def_node().is_some() && !in_private {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ClassMethodsDefinitions,
        "cops/style/class_methods_definitions"
    );
}
