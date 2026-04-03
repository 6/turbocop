use crate::cop::shared::constant_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashMap;

/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=408, FN=914.
///
/// Root causes in the previous implementation:
/// - FP: `its(:x)` signatures were keyed only by args, so different block expectations
///   (e.g. different `include(...)` checks) were incorrectly grouped together.
/// - FN: only direct statements were checked, missing examples nested under iterator/if
///   wrappers within the same example-group scope.
///
/// Fix:
/// - Replaced direct-statement scan with a scope-aware recursive collector that mirrors
///   RuboCop's `ExampleGroup#find_all_in_scope`: recurse through the current group, but
///   stop at nested example groups/shared groups/include blocks and at example bodies.
/// - Non-`its` signatures use example args (docstring + metadata).
/// - `its` signatures use docstring + implementation body so block differences are respected.
///
/// Validation (`scripts/check-cop.py RSpec/RepeatedDescription --verbose --rerun`):
/// - Expected (RuboCop): 2,875
/// - Actual (nitrocop): 2,866
/// - Potential FP: 0
/// - Potential FN: 9
///
/// Note: check-cop reports FAIL against the CI nitrocop baseline because this fix restores
/// many previously-missed detections (large FN reduction), increasing total offense count.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=0, FN=7.
///
/// FN=7: Descriptions with same content but different quote styles (single vs double
/// quotes) were not matched. The signature used raw source bytes including quotes.
/// Fix: use `unescaped()` content for StringNode args to normalize quote style,
/// matching RuboCop's `doc_string` which returns the string value.
///
/// ## Corpus investigation (2026-03-30)
///
/// FP=0, FN=2.
///
/// FN=2: repeated `specify ""` examples were skipped because explicit empty-string
/// docstrings normalize to an empty byte signature, and the grouping loop treated any
/// empty signature as "no description". Fix: only skip examples when
/// `example_signature` returns `None` (true one-liners with no docstring); keep empty
/// string signatures so blank descriptions are grouped like RuboCop.
pub struct RepeatedDescription;

#[derive(Clone)]
struct ExampleEntry {
    is_its: bool,
    signature: Vec<u8>,
    line: usize,
    column: usize,
}

struct ExampleCollector<'a> {
    source: &'a SourceFile,
    examples: Vec<ExampleEntry>,
}

impl<'a> ExampleCollector<'a> {
    fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            examples: Vec::new(),
        }
    }
}

impl<'pr> Visit<'pr> for ExampleCollector<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        let block = node.block().and_then(|b| b.as_block_node());
        let has_block = block.is_some();

        // Scope boundaries: nested example groups/shared groups/include blocks
        // should not be searched from the current example-group scope.
        if has_block && is_scope_change_call(node) {
            return;
        }

        // Examples are collected but their bodies are not traversed.
        if has_block && is_example_call(node) {
            let is_its = name == b"its";
            let signature = if is_its {
                its_signature(self.source, node)
            } else {
                example_signature(self.source, node)
            };

            if let Some(signature) = signature {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.examples.push(ExampleEntry {
                    is_its,
                    signature,
                    line,
                    column,
                });
            }
            return;
        }

        ruby_prism::visit_call_node(self, node);
    }
}

impl Cop for RepeatedDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedDescription"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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

        if !is_example_group_call(&call) {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let mut collector = ExampleCollector::new(source);
        collector.visit(&body);

        #[allow(clippy::type_complexity)] // internal collection used only in this function
        let mut repeated_desc: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();
        #[allow(clippy::type_complexity)] // internal collection used only in this function
        let mut repeated_its: HashMap<Vec<u8>, Vec<(usize, usize)>> = HashMap::new();

        for example in &collector.examples {
            if example.is_its {
                repeated_its
                    .entry(example.signature.clone())
                    .or_default()
                    .push((example.line, example.column));
            } else {
                repeated_desc
                    .entry(example.signature.clone())
                    .or_default()
                    .push((example.line, example.column));
            }
        }

        for locs in repeated_desc.values().chain(repeated_its.values()) {
            if locs.len() <= 1 {
                continue;
            }
            for &(line, column) in locs {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Don't repeat descriptions within an example group.".to_string(),
                ));
            }
        }
    }
}

/// Build a signature for an example call based on the content of its arguments
/// (description + metadata), excluding the block body.
///
/// Uses unescaped string content for description strings so that `'foo'` and
/// `"foo"` produce the same signature (matching RuboCop's `doc_string` which
/// returns the string value, not the raw source with quotes).
fn example_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None; // No description = one-liner, skip
    }

    let mut sig = Vec::new();
    for (i, arg) in arg_list.iter().enumerate() {
        if i > 0 {
            sig.push(0); // separator
        }
        // For string nodes, use the unescaped content to normalize quote style
        if let Some(s) = arg.as_string_node() {
            sig.extend_from_slice(s.unescaped().as_ref());
        } else {
            // For non-string args (symbols, hashes, etc.), use raw source
            let loc = arg.location();
            sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
        }
    }
    Some(sig)
}

/// Build a signature for `its` examples from docstring + implementation.
fn its_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let doc = args.arguments().iter().next()?;

    let block = call.block().and_then(|b| b.as_block_node())?;
    let body = block.body()?;

    let doc_loc = doc.location();
    let body_loc = body.location();
    let mut sig = Vec::new();
    sig.extend_from_slice(&source.as_bytes()[doc_loc.start_offset()..doc_loc.end_offset()]);
    sig.push(0);
    sig.extend_from_slice(&source.as_bytes()[body_loc.start_offset()..body_loc.end_offset()]);
    Some(sig)
}

fn is_example_group_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if !is_rspec_example_group(name) || is_rspec_shared_group(name) {
        return false;
    }

    match call.receiver() {
        None => true,
        Some(recv) => {
            constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
        }
    }
}

fn is_example_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver().is_none() && is_rspec_example(call.name().as_slice())
}

fn is_scope_change_call(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();

    if call.receiver().is_none() {
        return is_rspec_example_group(name)
            || is_rspec_shared_group(name)
            || is_include_scope_method(name);
    }

    let is_rspec_receiver = constant_predicates::constant_short_name(&call.receiver().unwrap())
        .is_some_and(|n| n == b"RSpec");
    is_rspec_receiver && (is_rspec_example_group(name) || is_rspec_shared_group(name))
}

fn is_include_scope_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"include_examples" | b"it_behaves_like" | b"it_should_behave_like" | b"include_context"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RepeatedDescription, "cops/rspec/repeated_description");
}
