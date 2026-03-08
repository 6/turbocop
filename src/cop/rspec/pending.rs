use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, FALSE_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/Pending - detects pending specs via x-prefixed methods, `pending`/`skip` calls,
/// examples without blocks, and `:skip`/`:pending` metadata symbols or keyword args.
///
/// **Fixed FP (302):** The metadata check (`:skip`/`:pending` symbols and `skip:`/`pending:`
/// keyword args) was running on ALL call nodes, causing false positives on matcher arguments
/// like `eq(:skip)` and factory calls like `create(:record, skip: true)`. Fix: restrict
/// metadata checks to only RSpec example, example group, shared group, and hook methods.
///
/// ## Known false positives / reverted attempt (2026-03-07)
///
/// Attempted fix: rewrote the cop to mirror RuboCop `Pending` node-pattern logic
/// (skipped groups/examples + metadata true/str/dstr + regular examples without body).
///
/// Acceptance gate before (CI baseline): expected=2,996, actual=1,692, excess=0, missing=1,304.
/// Acceptance gate after rewrite (`check-cop --rerun`): expected=2,996, actual=3,223,
/// excess=227, missing=0 (adjusted excess vs CI baseline: +1,106).
///
/// Effect: removed many historical FN but introduced massive new FP clusters
/// (`skip` in helper/control-flow contexts across multiple repos).
///
/// A correct future fix needs parent/context-aware handling for `skip`/`pending`
/// send nodes rather than a pure method-name match.
pub struct Pending;

/// x-prefixed methods that indicate pending specs.
const XMETHODS: &[&[u8]] = &[
    b"xcontext",
    b"xdescribe",
    b"xexample",
    b"xfeature",
    b"xit",
    b"xscenario",
    b"xspecify",
];

impl Cop for Pending {
    fn name(&self) -> &'static str {
        "RSpec/Pending"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ASSOC_NODE,
            CALL_NODE,
            FALSE_NODE,
            KEYWORD_HASH_NODE,
            SYMBOL_NODE,
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

        // Check for x-prefixed methods with blocks
        if XMETHODS.contains(&method_name) && call.receiver().is_none() && call.block().is_some() {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Pending spec found.".to_string(),
            ));
        }

        // Check for `pending 'test' do` and `skip 'test' do` as example methods
        if (method_name == b"pending" || method_name == b"skip")
            && call.receiver().is_none()
            && call.block().is_some()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Pending spec found.".to_string(),
            ));
        }

        // Check for `skip` without arguments inside an example (standalone call)
        if method_name == b"skip"
            && call.receiver().is_none()
            && call.arguments().is_none()
            && call.block().is_none()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Pending spec found.".to_string(),
            ));
        }

        // Check for `it 'test'` without a block (pending example)
        let example_methods: &[&[u8]] = &[b"it", b"specify", b"example", b"scenario"];
        if example_methods.contains(&method_name)
            && call.receiver().is_none()
            && call.block().is_none()
            && call.arguments().is_some()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Pending spec found.".to_string(),
            ));
        }

        // Check for :skip or :pending metadata, or skip: true/string, pending: true/string
        // Only check metadata on RSpec example, example group, shared group, and hook methods
        // to avoid false positives on arbitrary method calls like `eq(:skip)`.
        let is_rspec_method = util::is_rspec_example(method_name)
            || util::is_rspec_example_group(method_name)
            || util::is_rspec_shared_group(method_name)
            || util::is_rspec_hook(method_name);
        let has_rspec_receiver = call.receiver().is_none()
            || call
                .receiver()
                .is_some_and(|recv| util::constant_name(&recv).is_some_and(|n| n == b"RSpec"));
        if is_rspec_method && has_rspec_receiver {
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    // Check for :skip or :pending symbol metadata
                    if let Some(sym) = arg.as_symbol_node() {
                        let val = sym.unescaped();
                        if val == b"skip" || val == b"pending" {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                "Pending spec found.".to_string(),
                            ));
                        }
                    }
                    // Check for skip: true/string, pending: true/string in keyword args
                    if let Some(kw) = arg.as_keyword_hash_node() {
                        for elem in kw.elements().iter() {
                            if let Some(assoc) = elem.as_assoc_node() {
                                if let Some(key_sym) = assoc.key().as_symbol_node() {
                                    let key = key_sym.unescaped();
                                    if key == b"skip" || key == b"pending" {
                                        // skip: false is not pending
                                        if assoc.value().as_false_node().is_some() {
                                            continue;
                                        }
                                        let loc = call.location();
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());
                                        diagnostics.push(self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            "Pending spec found.".to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Pending, "cops/rspec/pending");
}
