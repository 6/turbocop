use crate::cop::shared::node_type::{
    AND_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, DEFINED_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for multiple `defined?` calls joined by `&&`/`and` that can be
/// combined into a single `defined?`.
///
/// ## Corpus investigation (2026-03-20)
///
/// **FP root cause (6 total):** nitrocop used string prefix matching
/// (`starts_with("Foo::")`) to detect nested constants, which incorrectly
/// flagged cases that skip nesting levels (e.g., `defined?(Foo) &&
/// defined?(Foo::Bar::Baz)`). RuboCop checks that one argument is the
/// **direct** namespace/receiver of the other — not just any ancestor.
///
/// **FN root cause (10 total):** `get_defined_subject` only handled constant
/// nodes (ConstantReadNode, ConstantPathNode), not method call nodes. Patterns
/// like `defined?(Rails) && defined?(Rails.backtrace_cleaner)` were missed
/// because `Rails.backtrace_cleaner` is a CallNode. Fixed by also extracting
/// the receiver from CallNode subjects.
///
/// **Fix:** replaced string prefix matching with structured parent extraction.
/// For each `defined?` subject, extract its direct parent (namespace for
/// const paths, receiver for calls). Then check if any subject matches
/// another's parent via source text comparison. This matches RuboCop's
/// `namespaces` / `defined_calls` logic.
///
/// RuboCop also requires ALL terms in the `&&` chain to be `defined?` calls.
/// We replicate this by collecting all terms and checking the `all defined?`
/// condition. This prevents false positives on mixed expressions like
/// `defined?(Foo) && bar && defined?(Foo::Bar)`.
pub struct CombinableDefined;

impl Cop for CombinableDefined {
    fn name(&self) -> &'static str {
        "Style/CombinableDefined"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            DEFINED_NODE,
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
        if let Some(and_node) = node.as_and_node() {
            check_and(
                self,
                source,
                node,
                &and_node.left(),
                &and_node.right(),
                diagnostics,
            );
            return;
        }

        // `and` keyword is sometimes parsed as a CallNode
        if let Some(call) = node.as_call_node() {
            let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            if method_name == "and" {
                if let Some(receiver) = call.receiver() {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            check_and(self, source, node, &receiver, &arg_list[0], diagnostics);
                        }
                    }
                }
            }
        }
    }
}

/// Subject info extracted from a `defined?` argument.
struct SubjectInfo {
    /// Source text of the subject (e.g., "Foo::Bar", "Rails.backtrace_cleaner")
    text: String,
    /// Source text of the direct parent (namespace for const, receiver for call)
    parent_text: Option<String>,
}

/// Recursively collect subject info from all `defined?` terms in an `&&`/`and` chain.
/// Returns `None` if any term is not a `defined?` call (matching RuboCop's
/// `terms.all?(&:defined_type?)` check).
fn collect_subjects(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    out: &mut Vec<SubjectInfo>,
) -> bool {
    if let Some(and_node) = node.as_and_node() {
        let ok_left = collect_subjects(source, &and_node.left(), out);
        let ok_right = collect_subjects(source, &and_node.right(), out);
        return ok_left && ok_right;
    }

    if let Some(call) = node.as_call_node() {
        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name == "and" {
            if let Some(receiver) = call.receiver() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        let ok_left = collect_subjects(source, &receiver, out);
                        let ok_right = collect_subjects(source, &arg_list[0], out);
                        return ok_left && ok_right;
                    }
                }
            }
            return false;
        }
    }

    // Leaf term — must be a defined? call
    if let Some(defined) = node.as_defined_node() {
        let value = defined.value();
        // Only const and call subjects (matching RuboCop's filter)
        if value.as_constant_read_node().is_some()
            || value.as_constant_path_node().is_some()
            || value.as_call_node().is_some()
        {
            let text = node_source_text(source, &value);
            let parent_text = extract_direct_parent(source, &value);
            out.push(SubjectInfo { text, parent_text });
        }
        return true;
    }

    // Non-defined? term (e.g., `bar`, `ObjectSpace.memsize_of(value)`)
    false
}

fn check_and(
    cop: &CombinableDefined,
    source: &SourceFile,
    whole_node: &ruby_prism::Node<'_>,
    left: &ruby_prism::Node<'_>,
    right: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Collect subject info from all terms in the chain
    let mut subjects = Vec::new();
    let all_left = collect_subjects(source, left, &mut subjects);
    // Reset and collect from the full chain — we need all terms together
    if !all_left {
        return;
    }
    let all_right = collect_subjects(source, right, &mut subjects);
    if !all_right {
        return;
    }

    // Check if any subject's direct parent matches another subject's text
    let subject_texts: Vec<&str> = subjects.iter().map(|s| s.text.as_str()).collect();

    for subject in &subjects {
        if let Some(ref parent_text) = subject.parent_text {
            if subject_texts.contains(&parent_text.as_str()) {
                let loc = whole_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Combine nested `defined?` calls.".to_string(),
                ));
                return;
            }
        }
    }
}

/// Extract the direct parent/namespace of a defined? subject as source text:
/// - `Foo::Bar` → `"Foo"`
/// - `::Foo::Bar` → `"::Foo"`
/// - `foo.bar` → `"foo"`
/// - `Foo` (top-level constant) → None
fn extract_direct_parent(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<String> {
    if let Some(path) = node.as_constant_path_node() {
        let parent = path.parent()?;
        return Some(node_source_text(source, &parent));
    }
    if let Some(call) = node.as_call_node() {
        let receiver = call.receiver()?;
        return Some(node_source_text(source, &receiver));
    }
    None
}

/// Get the source text of a node.
fn node_source_text(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CombinableDefined, "cops/style/combinable_defined");
}
