use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_READ_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE,
    GLOBAL_VARIABLE_READ_NODE, INSTANCE_VARIABLE_READ_NODE, LOCAL_VARIABLE_READ_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DeprecatedOpenSSLConstant;

impl Cop for DeprecatedOpenSSLConstant {
    fn name(&self) -> &'static str {
        "Lint/DeprecatedOpenSSLConstant"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_READ_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_READ_NODE,
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"new" && method_name != b"digest" {
            return;
        }

        // RuboCop skips when arguments contain variables, method calls, or constants
        // because autocorrection can't handle dynamic values
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if arg.as_local_variable_read_node().is_some()
                    || arg.as_instance_variable_read_node().is_some()
                    || arg.as_class_variable_read_node().is_some()
                    || arg.as_global_variable_read_node().is_some()
                    || arg.as_call_node().is_some()
                    || arg.as_constant_read_node().is_some()
                    || arg.as_constant_path_node().is_some()
                {
                    return;
                }
            }
        }

        // Check for pattern: OpenSSL::Cipher::XXX.new or OpenSSL::Digest::XXX.new/digest
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver should be a ConstantPathNode like OpenSSL::Cipher::AES
        let recv_path = match recv.as_constant_path_node() {
            Some(p) => p,
            None => return,
        };

        // The algorithm name is the rightmost constant
        let _algo_name = match recv_path.name() {
            Some(n) => n,
            None => return,
        };

        // Parent should be OpenSSL::Cipher or OpenSSL::Digest
        let parent = match recv_path.parent() {
            Some(p) => p,
            None => return,
        };

        let parent_path = match parent.as_constant_path_node() {
            Some(p) => p,
            None => return,
        };

        let parent_name = match parent_path.name() {
            Some(n) => n,
            None => return,
        };

        let parent_name_str = parent_name.as_slice();
        if parent_name_str != b"Cipher" && parent_name_str != b"Digest" {
            return;
        }

        // Grandparent should be OpenSSL
        let grandparent = match parent_path.parent() {
            Some(p) => p,
            None => return,
        };

        let is_openssl = if let Some(const_read) = grandparent.as_constant_read_node() {
            const_read.name().as_slice() == b"OpenSSL"
        } else if let Some(const_path) = grandparent.as_constant_path_node() {
            const_path
                .name()
                .is_some_and(|n| n.as_slice() == b"OpenSSL")
        } else {
            false
        };

        if !is_openssl {
            return;
        }

        let parent_class =
            std::str::from_utf8(parent_path.location().as_slice()).unwrap_or("OpenSSL::Cipher");

        let recv_src =
            std::str::from_utf8(recv.location().as_slice()).unwrap_or("OpenSSL::Cipher::AES");

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{parent_class}` instead of `{recv_src}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DeprecatedOpenSSLConstant,
        "cops/lint/deprecated_open_ssl_constant"
    );
}
