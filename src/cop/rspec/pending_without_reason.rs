use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE};

pub struct PendingWithoutReason;

/// x-prefixed methods that skip specs.
const XMETHODS: &[&[u8]] = &[
    b"xcontext", b"xdescribe", b"xexample", b"xfeature",
    b"xit", b"xscenario", b"xspecify",
];

impl Cop for PendingWithoutReason {
    fn name(&self) -> &'static str {
        "RSpec/PendingWithoutReason"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ASSOC_NODE, CALL_NODE, KEYWORD_HASH_NODE, SYMBOL_NODE, TRUE_NODE]
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

        // x-prefixed methods with blocks: xdescribe, xit, etc.
        if XMETHODS.contains(&method_name) {
            let is_rspec = if call.receiver().is_none() {
                true
            } else if let Some(recv) = call.receiver() {
                util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
            } else {
                false
            };

            if is_rspec && call.block().is_some() {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let label = std::str::from_utf8(method_name).unwrap_or("skip");
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Give the reason for {label}."),
                ));
            }
        }

        // `pending` or `skip` used as example method with block (no reason)
        if (method_name == b"pending" || method_name == b"skip")
            && call.receiver().is_none()
            && call.block().is_some()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let label = std::str::from_utf8(method_name).unwrap_or("skip");
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Give the reason for {label}."),
            ));
        }

        // `pending` or `skip` without arguments (no reason string)
        if (method_name == b"pending" || method_name == b"skip")
            && call.receiver().is_none()
            && call.arguments().is_none()
            && call.block().is_none()
        {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let label = std::str::from_utf8(method_name).unwrap_or("skip");
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Give the reason for {label}."),
            ));
        }

        // Check metadata: :skip, :pending, skip: true, pending: true (without reason string)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                // :skip or :pending symbol metadata
                if let Some(sym) = arg.as_symbol_node() {
                    let val = sym.unescaped();
                    if (val == b"skip" || val == b"pending") && call.block().is_some() {
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let label = std::str::from_utf8(val).unwrap_or("skip");
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Give the reason for {label}."),
                        ));
                    }
                }
                // skip: true or pending: true (not a string reason)
                if let Some(kw) = arg.as_keyword_hash_node() {
                    for elem in kw.elements().iter() {
                        if let Some(assoc) = elem.as_assoc_node() {
                            if let Some(key_sym) = assoc.key().as_symbol_node() {
                                let key = key_sym.unescaped();
                                if key == b"skip" || key == b"pending" {
                                    let value = assoc.value();
                                    // Only flag if value is `true` (not a string reason)
                                    if value.as_true_node().is_some() && call.block().is_some() {
                                        let loc = call.location();
                                        let (line, column) =
                                            source.offset_to_line_col(loc.start_offset());
                                        let label = std::str::from_utf8(key).unwrap_or("skip");
                                        diagnostics.push(self.diagnostic(
                                            source,
                                            line,
                                            column,
                                            format!("Give the reason for {label}."),
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
    crate::cop_fixture_tests!(PendingWithoutReason, "cops/rspec/pending_without_reason");
}
