use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// RSpec/Pending - detects pending specs via x-prefixed methods, `pending`/`skip` calls,
/// examples without blocks, and `:skip`/`:pending` metadata symbols or keyword args.
///
/// **Root cause of original 103 FPs:** Standalone `skip` (no args, no block) was detected
/// without verifying the call was receiverless in all code paths. Also metadata checks
/// could fire on non-RSpec calls.
///
/// **Root cause of original 1,407 FNs:**
/// - XMETHODS (xdescribe, xcontext, etc.) incorrectly required a block — vendor fires
///   with or without block.
/// - `pending` standalone (no args, no block) was not detected — only `skip` was.
/// - `skip`/`pending` with string arg but no block (e.g., `skip 'not ready'`) not detected.
/// - Metadata (:skip/:pending symbols, skip:/pending: keywords) on example groups
///   (describe, context, feature, example_group) was not fully detected.
/// - `its` was missing from the example methods list for "no body" detection.
/// - `xdescribe`/`xcontext`/`xfeature` with RSpec receiver not detected.
///
/// **Detection patterns (matches vendor rubocop-rspec):**
/// 1. X-prefixed example groups: xdescribe, xcontext, xfeature (nil or RSpec receiver)
/// 2. X-prefixed examples: xit, xspecify, xexample, xscenario (nil receiver)
/// 3. `skip`/`pending` as example-level calls (nil receiver, any args, with or without block)
/// 4. Examples without bodies: it, specify, example, scenario, its (nil receiver, has args, no block)
/// 5. Symbol metadata :skip/:pending on example groups or examples
/// 6. Keyword metadata skip:/pending: with true/string value (NOT skip: false) on groups or examples
pub struct Pending;

/// X-prefixed example group methods (skipped groups).
const XGROUP_METHODS: &[&[u8]] = &[b"xcontext", b"xdescribe", b"xfeature"];

/// X-prefixed example methods (skipped examples).
const XEXAMPLE_METHODS: &[&[u8]] = &[b"xexample", b"xit", b"xscenario", b"xspecify"];

/// Regular example group methods that can have :skip/:pending metadata.
const REGULAR_GROUPS: &[&[u8]] = &[b"context", b"describe", b"example_group", b"feature"];

/// Regular example methods that can have :skip/:pending metadata or be body-less.
const REGULAR_EXAMPLES: &[&[u8]] = &[b"example", b"it", b"its", b"scenario", b"specify"];

/// Returns true if the receiver is nil or `RSpec`/`::RSpec`.
fn has_rspec_or_nil_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    match call.receiver() {
        None => true,
        Some(recv) => util::constant_name(&recv).is_some_and(|n| n == b"RSpec"),
    }
}

/// Check if a call's arguments contain :skip or :pending symbol metadata,
/// or skip:/pending: keyword metadata with a truthy value (not false).
fn has_skip_or_pending_metadata(call: &ruby_prism::CallNode<'_>) -> bool {
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };

    for arg in args.arguments().iter() {
        // Check for :skip or :pending symbol metadata
        if let Some(sym) = arg.as_symbol_node() {
            let val = sym.unescaped();
            if val == b"skip" || val == b"pending" {
                return true;
            }
        }

        // Check for skip: / pending: keyword args
        if let Some(kw) = arg.as_keyword_hash_node() {
            for elem in kw.elements().iter() {
                if let Some(assoc) = elem.as_assoc_node() {
                    if let Some(key_sym) = assoc.key().as_symbol_node() {
                        let key = key_sym.unescaped();
                        if (key == b"skip" || key == b"pending")
                            && assoc.value().as_false_node().is_none()
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        use ruby_prism::Visit;

        struct Visitor<'a> {
            cop: &'a Pending,
            source: &'a SourceFile,
            diagnostics: &'a mut Vec<Diagnostic>,
        }

        impl Visitor<'_> {
            fn flag(&mut self, call: &ruby_prism::CallNode<'_>) {
                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Pending spec found.".to_string(),
                ));
            }

            fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
                let method_name = call.name().as_slice();

                // 1. X-prefixed example groups (xdescribe, xcontext, xfeature)
                //    Matches with nil or RSpec receiver, with or without block.
                if XGROUP_METHODS.contains(&method_name) && has_rspec_or_nil_receiver(call) {
                    self.flag(call);
                    return;
                }

                // 2. X-prefixed examples (xit, xspecify, xexample, xscenario)
                //    Nil receiver only, with or without block.
                if XEXAMPLE_METHODS.contains(&method_name) && call.receiver().is_none() {
                    self.flag(call);
                    return;
                }

                // 3. `skip`/`pending` as example-defining or standalone calls.
                //    Nil receiver, any args (or none), with or without block.
                if (method_name == b"skip" || method_name == b"pending")
                    && call.receiver().is_none()
                {
                    self.flag(call);
                    return;
                }

                // 4. Regular example groups with :skip/:pending metadata.
                if REGULAR_GROUPS.contains(&method_name)
                    && has_rspec_or_nil_receiver(call)
                    && has_skip_or_pending_metadata(call)
                {
                    self.flag(call);
                    return;
                }

                // 5. Regular examples with :skip/:pending metadata.
                if REGULAR_EXAMPLES.contains(&method_name)
                    && call.receiver().is_none()
                    && has_skip_or_pending_metadata(call)
                {
                    self.flag(call);
                    return;
                }

                // 6. Examples without bodies (e.g., `it 'test'` with no block).
                //    Must have at least one argument (to avoid matching `it` as block param).
                if REGULAR_EXAMPLES.contains(&method_name)
                    && call.receiver().is_none()
                    && call.block().is_none()
                    && call.arguments().is_some()
                {
                    self.flag(call);
                }
            }
        }

        impl Visit<'_> for Visitor<'_> {
            fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
                self.check_call(node);
                ruby_prism::visit_call_node(self, node);
            }
        }

        let mut visitor = Visitor {
            cop: self,
            source,
            diagnostics,
        };
        let root = parse_result.node();
        visitor.visit(&root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Pending, "cops/rspec/pending");
}
