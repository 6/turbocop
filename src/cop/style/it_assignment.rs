use crate::cop::shared::node_type::{
    BLOCK_NODE, BLOCK_PARAMETER_NODE, DEF_NODE, KEYWORD_REST_PARAMETER_NODE, LAMBDA_NODE,
    LOCAL_VARIABLE_AND_WRITE_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
    LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_TARGET_NODE, LOCAL_VARIABLE_WRITE_NODE,
    OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE,
    REQUIRED_PARAMETER_NODE, REST_PARAMETER_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const ASSIGNMENT_MSG: &str = "Avoid assigning to local variable `it`, since `it` will be the default block parameter in Ruby 3.4+. Consider using a different variable name.";
const PARAMETER_MSG: &str = "`it` is the default block parameter; consider another name.";

/// Matches RuboCop's `Style/ItAssignment` checks for explicit `it` locals and
/// parameters.
///
/// The original port only listened to `LocalVariableWriteNode`, which preserved
/// existing assignment hits but missed every explicit parameter shape Prism
/// represents separately, including block params like `{ |it| ... }` and method
/// params like `def foo(it)`, `def foo(*it)`, `def foo(it:)`, `def foo(**it)`,
/// and `def foo(&it)`. Fix: dispatch on the corresponding Prism parameter node
/// types and report when the parameter name is exactly `it`.
///
/// Additionally, compound assignment forms (`it ||= 0`, `it &&= 0`, `it += 1`)
/// and multi-write targets (`it ,= expr`) were missed because they use
/// `LocalVariableOrWriteNode`, `LocalVariableAndWriteNode`,
/// `LocalVariableOperatorWriteNode`, and `LocalVariableTargetNode` respectively,
/// not `LocalVariableWriteNode`. Fix: dispatch on those node types as well.
pub struct ItAssignment;

impl Cop for ItAssignment {
    fn name(&self) -> &'static str {
        "Style/ItAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETER_NODE,
            DEF_NODE,
            KEYWORD_REST_PARAMETER_NODE,
            LAMBDA_NODE,
            LOCAL_VARIABLE_AND_WRITE_NODE,
            LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_TARGET_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            OPTIONAL_PARAMETER_NODE,
            REQUIRED_KEYWORD_PARAMETER_NODE,
            REQUIRED_PARAMETER_NODE,
            REST_PARAMETER_NODE,
        ]
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
        if let Some(def_node) = node.as_def_node() {
            check_block_param(self, source, def_node.parameters(), diagnostics);
            return;
        }

        if let Some(block_node) = node.as_block_node() {
            let params = block_node
                .parameters()
                .and_then(|params| params.as_block_parameters_node())
                .and_then(|params| params.parameters());
            check_block_param(self, source, params, diagnostics);
            return;
        }

        if let Some(lambda_node) = node.as_lambda_node() {
            let params = lambda_node
                .parameters()
                .and_then(|params| params.as_block_parameters_node())
                .and_then(|params| params.parameters());
            check_block_param(self, source, params, diagnostics);
            return;
        }

        if let Some(write_node) = node.as_local_variable_write_node() {
            if write_node.name().as_slice() == b"it" {
                add_offense(
                    self,
                    source,
                    &write_node.name_loc(),
                    ASSIGNMENT_MSG,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(write_node) = node.as_local_variable_or_write_node() {
            if write_node.name().as_slice() == b"it" {
                add_offense(
                    self,
                    source,
                    &write_node.name_loc(),
                    PARAMETER_MSG,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(write_node) = node.as_local_variable_and_write_node() {
            if write_node.name().as_slice() == b"it" {
                add_offense(
                    self,
                    source,
                    &write_node.name_loc(),
                    PARAMETER_MSG,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(write_node) = node.as_local_variable_operator_write_node() {
            if write_node.name().as_slice() == b"it" {
                add_offense(
                    self,
                    source,
                    &write_node.name_loc(),
                    PARAMETER_MSG,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(target_node) = node.as_local_variable_target_node() {
            if target_node.name().as_slice() == b"it" {
                add_offense(
                    self,
                    source,
                    &target_node.location(),
                    PARAMETER_MSG,
                    diagnostics,
                );
            }
            return;
        }

        if let Some(param) = node.as_required_parameter_node() {
            if param.name().as_slice() == b"it" {
                add_offense(self, source, &param.location(), PARAMETER_MSG, diagnostics);
            }
            return;
        }

        if let Some(param) = node.as_optional_parameter_node() {
            if param.name().as_slice() == b"it" {
                add_offense(self, source, &param.location(), PARAMETER_MSG, diagnostics);
            }
            return;
        }

        if let Some(param) = node.as_required_keyword_parameter_node() {
            if strip_keyword_suffix(param.name().as_slice()) == b"it" {
                add_offense(self, source, &param.name_loc(), PARAMETER_MSG, diagnostics);
            }
            return;
        }

        if let Some(param) = node.as_optional_keyword_parameter_node() {
            if strip_keyword_suffix(param.name().as_slice()) == b"it" {
                add_offense(self, source, &param.name_loc(), PARAMETER_MSG, diagnostics);
            }
            return;
        }

        if let Some(param) = node.as_rest_parameter_node() {
            if let (Some(name), Some(name_loc)) = (param.name(), param.name_loc()) {
                if name.as_slice() == b"it" {
                    add_offense(self, source, &name_loc, PARAMETER_MSG, diagnostics);
                }
            }
            return;
        }

        if let Some(param) = node.as_keyword_rest_parameter_node() {
            if let (Some(name), Some(name_loc)) = (param.name(), param.name_loc()) {
                if name.as_slice() == b"it" {
                    add_offense(self, source, &name_loc, PARAMETER_MSG, diagnostics);
                }
            }
            return;
        }

        if let Some(param) = node.as_block_parameter_node() {
            if let (Some(name), Some(name_loc)) = (param.name(), param.name_loc()) {
                if name.as_slice() == b"it" {
                    add_offense(self, source, &name_loc, PARAMETER_MSG, diagnostics);
                }
            }
        }
    }
}

fn strip_keyword_suffix(name: &[u8]) -> &[u8] {
    if name.ends_with(b":") {
        &name[..name.len() - 1]
    } else {
        name
    }
}

fn add_offense(
    cop: &ItAssignment,
    source: &SourceFile,
    loc: &ruby_prism::Location<'_>,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(source, line, column, message.to_string()));
}

fn check_block_param(
    cop: &ItAssignment,
    source: &SourceFile,
    params: Option<ruby_prism::ParametersNode<'_>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(block_param) = params.and_then(|params| params.block()) else {
        return;
    };

    if let (Some(name), Some(name_loc)) = (block_param.name(), block_param.name_loc()) {
        if name.as_slice() == b"it" {
            add_offense(cop, source, &name_loc, PARAMETER_MSG, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ItAssignment, "cops/style/it_assignment");
}
