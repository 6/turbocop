use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, MODULE_NODE, STATEMENTS_NODE,
};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/DescribedClassModuleWrapping: Avoid opening modules and defining specs within them.
pub struct DescribedClassModuleWrapping;

impl Cop for DescribedClassModuleWrapping {
    fn name(&self) -> &'static str {
        "RSpec/DescribedClassModuleWrapping"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            MODULE_NODE,
            STATEMENTS_NODE,
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
        let module_node = match node.as_module_node() {
            Some(m) => m,
            None => return,
        };

        let loc = module_node.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());

        // Check if this module contains an RSpec.describe block (anywhere nested)
        if contains_rspec_describe(module_node) {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Avoid opening modules and defining specs within them.".to_string(),
            ));
        }
    }
}

fn contains_rspec_describe(module_node: ruby_prism::ModuleNode<'_>) -> bool {
    let body = match module_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    for stmt in stmts.body().iter() {
        if is_rspec_describe(&stmt) {
            return true;
        }
        // Check nested modules
        if let Some(nested_module) = stmt.as_module_node() {
            if contains_rspec_describe(nested_module) {
                return true;
            }
        }
    }
    false
}

fn is_rspec_describe(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    if name != b"describe" {
        return false;
    }
    // Check for RSpec receiver
    if let Some(recv) = call.receiver() {
        if let Some(cr) = recv.as_constant_read_node() {
            return cr.name().as_slice() == b"RSpec";
        }
        if let Some(cp) = recv.as_constant_path_node() {
            return cp.name().is_some_and(|n| n.as_slice() == b"RSpec") && cp.parent().is_none();
        }
    }
    // Receiverless describe is also an example group
    call.receiver().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        DescribedClassModuleWrapping,
        "cops/rspec/described_class_module_wrapping"
    );
}
