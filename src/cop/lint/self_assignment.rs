use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_VARIABLE_READ_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_PATH_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_READ_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_READ_NODE, GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE};

pub struct SelfAssignment;

impl Cop for SelfAssignment {
    fn name(&self) -> &'static str {
        "Lint/SelfAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_VARIABLE_READ_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_PATH_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_READ_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_READ_NODE, GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let _allow_rbs = config.get_bool("AllowRBSInlineAnnotation", false);

        // Local variable: x = x
        if let Some(write) = node.as_local_variable_write_node() {
            if let Some(read) = write.value().as_local_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    let loc = write.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Self-assignment detected.".to_string(),
                    ));
                }
            }
        }

        // Instance variable: @x = @x
        if let Some(write) = node.as_instance_variable_write_node() {
            if let Some(read) = write.value().as_instance_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    let loc = write.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Self-assignment detected.".to_string(),
                    ));
                }
            }
        }

        // Class variable: @@x = @@x
        if let Some(write) = node.as_class_variable_write_node() {
            if let Some(read) = write.value().as_class_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    let loc = write.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Self-assignment detected.".to_string(),
                    ));
                }
            }
        }

        // Global variable: $x = $x
        if let Some(write) = node.as_global_variable_write_node() {
            if let Some(read) = write.value().as_global_variable_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    let loc = write.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Self-assignment detected.".to_string(),
                    ));
                }
            }
        }

        // Constant: FOO = FOO
        if let Some(write) = node.as_constant_write_node() {
            if let Some(read) = write.value().as_constant_read_node() {
                if write.name().as_slice() == read.name().as_slice() {
                    let loc = write.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Self-assignment detected.".to_string(),
                    ));
                }
            }
        }

        // Constant path: Mod::FOO = Mod::FOO
        if let Some(write) = node.as_constant_path_write_node() {
            let target = write.target();
            let value = write.value();
            if let Some(val_path) = value.as_constant_path_node() {
                let target_src = target.location().as_slice();
                let val_src = val_path.location().as_slice();
                if target_src == val_src {
                    let loc = write.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Self-assignment detected.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelfAssignment, "cops/lint/self_assignment");
}
