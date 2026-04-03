use crate::cop::shared::node_type::{CALL_NODE, CLASS_NODE};
use crate::cop::shared::util::{full_constant_path, parent_class_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/ApplicationController — checks that controllers subclass `ApplicationController`.
///
/// Root cause of 0% match rate (FN=472): the cop had `default_include: app/controllers/**/*.rb`
/// restricting it to only files under `app/controllers/`, but RuboCop has NO Include restriction
/// in its vendor config — the cop runs on all files. Many corpus offenses are in test files
/// (e.g., `actionpack/test/`) and non-standard paths.
///
/// Additional FN sources:
/// - `::ActionController::Base` (leading `::`) was not handled — the raw byte comparison
///   failed because `parent_class_name` returns the full source text including `::`.
/// - `Class.new(ActionController::Base)` pattern was not handled — RuboCop's
///   `EnforceSuperclass` mixin has a separate `on_send` matcher for this.
///
/// **FN root cause (2 FN, 2026-03-18):**
/// - `stub_const("Trestle::ApplicationController", Class.new(ActionController::Base))` was
///   incorrectly skipped because the prefix check scanned for `ApplicationController` as raw text.
///   The string argument `"Trestle::ApplicationController"` contains `ApplicationController` as a
///   substring, so the check falsely returned (skipped) the offense.
///   Fix: changed prefix check to look for `ApplicationController` followed by `=` (constant
///   assignment syntax), not just any occurrence of the name. This correctly distinguishes
///   `ApplicationController = Class.new(...)` (skip) from `stub_const("...ApplicationController",
///   Class.new(...))` (fire).
pub struct ApplicationController;

impl Cop for ApplicationController {
    fn name(&self) -> &'static str {
        "Rails/ApplicationController"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_NODE, CALL_NODE]
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
        if let Some(class) = node.as_class_node() {
            self.check_class(source, &class, diagnostics);
        } else if let Some(call) = node.as_call_node() {
            self.check_class_new(source, &call, diagnostics);
        }
    }
}

/// Check if the line prefix contains `ApplicationController` as a constant assignment LHS.
/// Returns true if the prefix contains `ApplicationController` followed by `=` (with optional
/// whitespace), indicating a constant assignment like `ApplicationController = Class.new(...)`.
/// This avoids false matches for string literals like:
///   stub_const("SomeMod::ApplicationController", Class.new(ActionController::Base))
fn is_application_controller_assignment(prefix: &[u8]) -> bool {
    let needle = b"ApplicationController";
    let mut pos = 0;
    while pos + needle.len() <= prefix.len() {
        if &prefix[pos..pos + needle.len()] == needle {
            // Check if what follows (skipping whitespace) is `=` (but not `==`)
            let after = pos + needle.len();
            let rest = &prefix[after..];
            let rest_trimmed = rest.iter().position(|&b| b != b' ' && b != b'\t');
            let after_ws = match rest_trimmed {
                Some(i) => after + i,
                None => prefix.len(),
            };
            if after_ws < prefix.len() {
                let ch = prefix[after_ws];
                // `=` but not `==`
                if ch == b'=' {
                    let next = if after_ws + 1 < prefix.len() {
                        prefix[after_ws + 1]
                    } else {
                        0
                    };
                    if next != b'=' {
                        return true;
                    }
                }
            }
        }
        pos += 1;
    }
    false
}

impl ApplicationController {
    /// Check `class Foo < ActionController::Base` pattern.
    fn check_class(
        &self,
        source: &SourceFile,
        class: &ruby_prism::ClassNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Skip if the class IS ApplicationController itself
        let class_name = full_constant_path(source, &class.constant_path());
        if class_name == b"ApplicationController"
            || class_name.ends_with(b"::ApplicationController")
        {
            return;
        }

        let parent = match parent_class_name(source, class) {
            Some(p) => p,
            None => return,
        };

        // Handle both ActionController::Base and ::ActionController::Base
        let parent_trimmed = if parent.starts_with(b"::") {
            &parent[2..]
        } else {
            parent
        };

        if parent_trimmed == b"ActionController::Base" {
            // Report offense on the superclass node (matches RuboCop's behavior)
            let superclass = class.superclass().unwrap();
            let loc = superclass.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Controllers should subclass `ApplicationController`.".to_string(),
            ));
        }
    }

    /// Check `Class.new(ActionController::Base)` pattern.
    ///
    /// RuboCop's EnforceSuperclass mixin uses:
    ///   `(send (const {nil? cbase} :Class) :new BASE_PATTERN)`
    /// with the additional constraint that it must NOT be:
    ///   `ApplicationController = Class.new(ActionController::Base)`
    fn check_class_new(
        &self,
        source: &SourceFile,
        call: &ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Must be `Class.new(...)` — receiver is `Class`, method is `new`
        if call.name().as_slice() != b"new" {
            return;
        }
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver must be `Class` or `::Class`
        let is_class_const = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice() == b"Class"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            // ::Class — parent is None (cbase), child is "Class"
            cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Class")
        } else {
            false
        };
        if !is_class_const {
            return;
        }

        // Must have exactly one argument that is ActionController::Base
        let args = call.arguments();
        let arg_list = match args.as_ref() {
            Some(a) => a,
            None => return,
        };
        let arguments: Vec<_> = arg_list.arguments().iter().collect();
        if arguments.len() != 1 {
            return;
        }

        let arg = &arguments[0];
        let arg_bytes =
            &source.as_bytes()[arg.location().start_offset()..arg.location().end_offset()];
        let arg_trimmed = if arg_bytes.starts_with(b"::") {
            &arg_bytes[2..]
        } else {
            arg_bytes
        };
        if arg_trimmed != b"ActionController::Base" {
            return;
        }

        // Check if this is `ApplicationController = Class.new(...)` — skip it.
        // RuboCop's EnforceSuperclass checks that the parent casgn is NOT named
        // ApplicationController. We approximate by checking the source bytes
        // preceding `Class.new` on the same line for `ApplicationController`
        // followed by `=` (assignment syntax). We must check for `=` after the
        // name to avoid false matches in string literals like:
        //   stub_const("Trestle::ApplicationController", Class.new(...))
        // where `ApplicationController` appears inside a string argument, not as
        // a constant assignment LHS.
        let call_start = call.location().start_offset();
        // Find start of current line by scanning backwards for '\n'
        let line_start = source.as_bytes()[..call_start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(0);
        let prefix = &source.as_bytes()[line_start..call_start];
        if is_application_controller_assignment(prefix) {
            return;
        }

        // Report offense on the argument (ActionController::Base)
        let loc = arg.location();
        let (arg_line, arg_col) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            arg_line,
            arg_col,
            "Controllers should subclass `ApplicationController`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ApplicationController, "cops/rails/application_controller");
}
