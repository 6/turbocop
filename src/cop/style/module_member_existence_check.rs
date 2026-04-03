use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// Current reruns still had excess offenses from forms that RuboCop explicitly skips:
/// - methods without an inherit parameter, such as `class_variables` and
///   `included_modules`, were still being flagged when called with arguments
/// - non-simple `include?`/`member?` argument shapes (multiple args, splats, kwargs)
///   were treated as offenses even though the upstream matcher rejects them
///
/// Fix: mirror RuboCop's matcher more closely by separating methods with and
/// without inherit arguments and only flagging simple single-argument
/// `include?`/`member?` calls.
/// Acceptance gate after fix: `scripts/check-cop.py Style/ModuleMemberExistenceCheck --verbose --rerun`
/// improved the cop from Actual=428 to Actual=427 against Expected=425.
/// Remaining gap is concentrated in `jruby` (+5) and `jsonapi-resources` (+1),
/// offset by two repos with missing detections; those patterns were deferred.
pub struct ModuleMemberExistenceCheck;

/// Maps array-returning methods to their predicate equivalents
const METHOD_MAPPINGS: &[(&[u8], &str)] = &[
    (b"instance_methods", "method_defined?"),
    (b"public_instance_methods", "public_method_defined?"),
    (b"private_instance_methods", "private_method_defined?"),
    (b"protected_instance_methods", "protected_method_defined?"),
    (b"constants", "const_defined?"),
    (b"included_modules", "include?"),
    (b"class_variables", "class_variable_defined?"),
];

const METHODS_WITHOUT_INHERIT_PARAM: &[&[u8]] = &[b"class_variables", b"included_modules"];

fn is_simple_argument(arg: &ruby_prism::Node<'_>) -> bool {
    arg.as_splat_node().is_none()
        && arg.as_block_argument_node().is_none()
        && arg.as_hash_node().is_none()
        && arg.as_keyword_hash_node().is_none()
}

impl Cop for ModuleMemberExistenceCheck {
    fn name(&self) -> &'static str {
        "Style/ModuleMemberExistenceCheck"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let allowed_methods = config.get_string_array("AllowedMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `include?` or `member?`
        let outer_method = call.name();
        let outer_bytes = outer_method.as_slice();
        if outer_bytes != b"include?" && outer_bytes != b"member?" {
            return;
        }

        let outer_args = match call.arguments() {
            Some(args) => args,
            None => return,
        };
        let outer_arg_list: Vec<_> = outer_args.arguments().iter().collect();
        if outer_arg_list.len() != 1 || !is_simple_argument(&outer_arg_list[0]) {
            return;
        }

        // Receiver must be a call to one of the array-returning methods
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_method = recv_call.name();
        let recv_bytes = recv_method.as_slice();

        let predicate = match METHOD_MAPPINGS.iter().find(|(m, _)| *m == recv_bytes) {
            Some((_, p)) => *p,
            None => return,
        };

        let receiver_has_inherit_param = !METHODS_WITHOUT_INHERIT_PARAM.contains(&recv_bytes);
        match recv_call.arguments() {
            Some(args) if receiver_has_inherit_param => {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 || !is_simple_argument(&arg_list[0]) {
                    return;
                }
            }
            Some(_) => return,
            None => {}
        }

        // Check AllowedMethods
        if let Some(ref allowed) = allowed_methods {
            let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("");
            if allowed.iter().any(|m| m == recv_str) {
                return;
            }
        }

        let msg_loc = recv_call
            .message_loc()
            .unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{predicate}` instead."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ModuleMemberExistenceCheck,
        "cops/style/module_member_existence_check"
    );
}
