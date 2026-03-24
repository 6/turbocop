use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks direct manipulation of ActiveModel#errors as hash.
///
/// ## Investigation findings (2026-03-10)
///
/// **FP root cause (43 FPs):** The cop was not implementing RuboCop's
/// `receiver_matcher` logic, which distinguishes between model files
/// (path contains `/models/`) and non-model files. In RuboCop:
/// - Outside model files: `errors` must have an explicit receiver
///   (`user.errors`, `@record.errors`, `record.errors`) — bare `errors`
///   (implicit self, no receiver) is NOT flagged.
/// - Inside model files: bare `errors` (implicit self) IS flagged.
///
/// The original implementation treated all `errors` calls the same
/// regardless of whether `errors` had an explicit receiver or the
/// file path, causing false positives on bare `errors.keys`, `errors.values`,
/// `errors.to_h`, `errors.to_xml`, `errors[:field] << 'msg'`, etc. in
/// non-model files (controllers, services, specs, etc.).
///
/// **FN root cause (8 FNs):** Two issues:
/// 1. MANIPULATIVE_METHODS only included `<<` and `clear`. RuboCop includes
///    ~30 methods (`append`, `push`, `pop`, `shift`, `unshift`, `concat`,
///    `delete`, `reject!`, `select!`, `map!`, `sort!`, etc.).
/// 2. Missing `root_assignment?` pattern: `errors[:name] = []` (Prism
///    represents as `[]=` on `errors` directly, not on `errors[...]`).
///    Also missing `messages_details_assignment?` pattern for
///    `errors.messages[:name] = []`.
///
/// ## Investigation findings (2026-03-14)
///
/// **FP root cause (28 FPs):** Pattern 4 (`DEPRECATED_ERRORS_METHODS`: `keys`,
/// `values`, `to_h`, `to_xml`) was using `is_errors_receiver` which matches
/// BOTH `errors` AND `errors.messages`/`errors.details`. So `errors.messages.keys`
/// was incorrectly flagged — but `errors.messages` returns a plain Hash and
/// calling `.keys` on it is perfectly valid. Fixed by using `is_errors_call`
/// (direct `errors` only) in Pattern 4.
///
/// ## Investigation findings (2026-03-14, second pass)
///
/// **FP root cause (8 FPs):** `is_errors_call` matched `errors(args)` calls
/// (e.g., `result.errors(locale: :de).to_h`, `result.errors(full: true).to_h`
/// in dry-rb and OpenProject code). ActiveModel's `errors` method takes no
/// arguments; calls with arguments are custom methods on non-ActiveModel objects.
/// Fixed by adding `call.arguments().is_none()` check in `is_errors_call`.
///
/// ## Investigation findings (2026-03-23)
///
/// **FP root cause (3 FPs):** Pattern 4 (`DEPRECATED_ERRORS_METHODS`) was not
/// checking whether the deprecated method itself was called with arguments.
/// RuboCop's `errors_deprecated?` pattern has no trailing `...`, so it only
/// matches argument-less calls. Calls like `errors.to_xml(:skip_instruct => true)`
/// are NOT deprecated hash manipulation — `to_xml` with options is a legitimate
/// serialization method. Fixed by adding `call.arguments().is_none()` check
/// in Pattern 4.
pub struct DeprecatedActiveModelErrorsMethods;

const MSG: &str = "Avoid manipulating ActiveModel errors as hash directly.";

/// Manipulative methods that indicate direct hash manipulation.
/// Matches RuboCop's MANIPULATIVE_METHODS set.
const MANIPULATIVE_METHODS: &[&[u8]] = &[
    b"<<",
    b"append",
    b"clear",
    b"collect!",
    b"compact!",
    b"concat",
    b"delete",
    b"delete_at",
    b"delete_if",
    b"drop",
    b"drop_while",
    b"fill",
    b"filter!",
    b"flatten!",
    b"insert",
    b"keep_if",
    b"map!",
    b"pop",
    b"prepend",
    b"push",
    b"reject!",
    b"replace",
    b"reverse!",
    b"rotate!",
    b"select!",
    b"shift",
    b"shuffle!",
    b"slice!",
    b"sort!",
    b"sort_by!",
    b"uniq!",
    b"unshift",
];

/// Deprecated methods called directly on errors (e.g., errors.keys, errors.values).
const DEPRECATED_ERRORS_METHODS: &[&[u8]] = &[b"keys", b"values", b"to_h", b"to_xml"];

/// Check if the file path contains `/models/`, matching RuboCop's `model_file?`.
fn is_model_file(source: &SourceFile) -> bool {
    source.path_str().contains("/models/")
}

impl Cop for DeprecatedActiveModelErrorsMethods {
    fn name(&self) -> &'static str {
        "Rails/DeprecatedActiveModelErrorsMethods"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let model_file = is_model_file(source);

        // Pattern 4: errors.keys / errors.values / errors.to_h / errors.to_xml
        // Only flag when receiver is `errors` directly — NOT `errors.messages` or
        // `errors.details` (those return a plain Hash, so .keys/.values etc. are valid).
        // RuboCop's `errors_deprecated?` pattern has no trailing `...`, so it only
        // matches calls WITHOUT arguments. e.g. `errors.to_xml(:skip_instruct => true)`
        // is NOT flagged.
        if DEPRECATED_ERRORS_METHODS.contains(&method_name) && call.arguments().is_none() {
            if let Some(recv) = call.receiver() {
                if is_errors_call(&recv, model_file) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
            }
        }

        // Pattern 1-3: errors[:name] << 'msg' / errors[:name].clear / etc.
        // Also: errors.messages[:name] << / errors.details[:name] <<
        if MANIPULATIVE_METHODS.contains(&method_name) {
            if let Some(recv) = call.receiver() {
                if is_errors_bracket_access(&recv, model_file) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
            }
        }

        // Root assignment: errors[:name] = [] (Prism: `[]=` on `errors` directly)
        // Also: errors.messages[:name] = [] / errors.details[:name] = []
        if method_name == b"[]=" {
            if let Some(recv) = call.receiver() {
                // Check for errors[:name] = ... (bracket access on errors)
                if is_errors_bracket_access(&recv, model_file) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                    return;
                }
                // Check for errors[:]= ... (root assignment: `[]=` directly on `errors`)
                // and errors.messages[:]= / errors.details[:]= (messages/details assignment)
                if is_errors_receiver(&recv, model_file) {
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                }
            }
        }
    }
}

/// Check if a node is `errors`, `errors.messages`, or `errors.details`,
/// with receiver validation matching RuboCop's `receiver_matcher`.
///
/// Outside model files, `errors` must have an explicit receiver (send/ivar/lvar).
/// Inside model files, bare `errors` (implicit self, no receiver) is also accepted.
fn is_errors_receiver(node: &ruby_prism::Node<'_>, model_file: bool) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if name == b"errors" {
            return is_valid_errors_call(&call, model_file);
        }
        // errors.messages or errors.details
        if (name == b"messages" || name == b"details") && call.arguments().is_none() {
            if let Some(recv) = call.receiver() {
                return is_errors_call(&recv, model_file);
            }
        }
    }
    false
}

/// Check if an `errors` CallNode has a valid receiver.
///
/// RuboCop's `receiver_matcher`:
/// - Outside models: `{send ivar lvar}` — requires explicit receiver
/// - Inside models: `{nil? send ivar lvar}` — also allows bare `errors` (nil? = no receiver)
fn is_valid_errors_call(call: &ruby_prism::CallNode<'_>, model_file: bool) -> bool {
    match call.receiver() {
        Some(recv) => {
            // Explicit receiver: must be send (CallNode), ivar (InstanceVariableReadNode),
            // or lvar (LocalVariableReadNode)
            recv.as_call_node().is_some()
                || recv.as_instance_variable_read_node().is_some()
                || recv.as_local_variable_read_node().is_some()
        }
        None => {
            // Bare `errors` (implicit self) — only valid in model files
            model_file
        }
    }
}

/// Check if a node is `x.errors` or bare `errors` (with receiver validation).
///
/// ActiveModel's `errors` method takes no arguments. Calls with arguments
/// (e.g., `errors(locale: :de)`, `errors(full: true)`) are NOT ActiveModel errors
/// and should NOT be flagged.
fn is_errors_call(node: &ruby_prism::Node<'_>, model_file: bool) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"errors" && call.arguments().is_none() {
            return is_valid_errors_call(&call, model_file);
        }
    }
    false
}

/// Check if a node is `errors[:field]`, `errors.messages[:field]`, or `errors.details[:field]`.
fn is_errors_bracket_access(node: &ruby_prism::Node<'_>, model_file: bool) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"[]" {
            if let Some(recv) = call.receiver() {
                return is_errors_receiver(&recv, model_file);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DeprecatedActiveModelErrorsMethods,
        "cops/rails/deprecated_active_model_errors_methods"
    );
}
