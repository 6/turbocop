use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CLASS_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, CONSTANT_WRITE_NODE, SELF_NODE, STATEMENTS_NODE};

pub struct EmptyClassDefinition;

impl Cop for EmptyClassDefinition {
    fn name(&self) -> &'static str {
        "Style/EmptyClassDefinition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CLASS_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, CONSTANT_WRITE_NODE, SELF_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "class_definition");

        match enforced_style {
            "class_definition" => diagnostics.extend(check_class_definition_style(self, source, node)),
            "class_new" => diagnostics.extend(check_class_new_style(self, source, node)),
            _ => {}
        }
    }
}

fn check_class_definition_style(
    cop: &EmptyClassDefinition,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Vec<Diagnostic> {
    // Check for FooError = Class.new(StandardError)
    if let Some(const_write) = node.as_constant_write_node() {
        let value = const_write.value();
        if let Some(call) = value.as_call_node() {
            let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            if method_name == "new" {
                if let Some(receiver) = call.receiver() {
                    if is_class_const(&receiver) {
                        // Skip if it has a block
                        if call.block().is_some() {
                            return Vec::new();
                        }
                        // Skip if chained with another method
                        // (can't easily detect from Prism AST alone)

                        // Check parent class arg
                        if let Some(args) = call.arguments() {
                            let arg_list: Vec<_> = args.arguments().iter().collect();
                            if arg_list.len() <= 1 {
                                // Verify the parent is a constant, not a variable
                                if arg_list.len() == 1 {
                                    let arg = &arg_list[0];
                                    if arg.as_constant_read_node().is_none()
                                        && arg.as_constant_path_node().is_none()
                                        && arg.as_self_node().is_none()
                                    {
                                        return Vec::new();
                                    }
                                    // Skip if parent is self
                                    if arg.as_self_node().is_some() {
                                        return Vec::new();
                                    }
                                }

                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                return vec![cop.diagnostic(
                                    source,
                                    line,
                                    column,
                                    "Prefer a two-line class definition over `Class.new` for classes with no body.".to_string(),
                                )];
                            }
                        } else {
                            // Class.new with no args
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            return vec![cop.diagnostic(
                                source,
                                line,
                                column,
                                "Prefer a two-line class definition over `Class.new` for classes with no body.".to_string(),
                            )];
                        }
                    }
                }
            }
        }
    }

    Vec::new()
}

fn check_class_new_style(
    cop: &EmptyClassDefinition,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Vec<Diagnostic> {
    // Check for empty class definitions
    if let Some(class_node) = node.as_class_node() {
        // Must have no body
        if class_node.body().is_some() {
            // Check if body has actual statements
            if let Some(body) = class_node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    if stmts.body().iter().next().is_some() {
                        return Vec::new();
                    }
                }
            }
        }

        let loc = class_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        return vec![cop.diagnostic(
            source,
            line,
            column,
            "Prefer `Class.new` over class definition for classes with no body.".to_string(),
        )];
    }

    Vec::new()
}

fn is_class_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(read) = node.as_constant_read_node() {
        return std::str::from_utf8(read.name().as_slice()).unwrap_or("") == "Class";
    }
    if let Some(path) = node.as_constant_path_node() {
        let name = std::str::from_utf8(path.name_loc().as_slice()).unwrap_or("");
        return name == "Class" && path.parent().is_none();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyClassDefinition, "cops/style/empty_class_definition");
}
