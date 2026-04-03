use crate::cop::shared::constant_predicates;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// RSpec/Pending - detects pending specs via x-prefixed methods, `pending`/`skip` calls,
/// examples without blocks, and `:skip`/`:pending` metadata symbols or keyword args.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=103, FN=5.
///
/// FP=103 root cause: keyword metadata matching was too broad. We treated
/// `skip:`/`pending:` as pending for any value except `false`, but RuboCop only
/// matches literal `true`, `str`, and `dstr` metadata values. Dynamic boolean or
/// expression values (for example version checks) must not be flagged.
///
/// FN=5 root cause: examples with a block-pass arg (`&(proc do ... end)`) were
/// treated as having a body because Prism exposes block-pass via `call.block()`.
/// RuboCop's `!node.block_node` still treats these as body-less pending examples.
///
/// Fixes applied:
/// - Restrict keyword metadata values to RuboCop's matcher shape: `true|str|dstr`.
/// - Treat only `BlockNode` as a real example body; block-pass nodes still count
///   as body-less for this cop.
///
/// ## Corpus investigation (2026-03-29)
///
/// FN=5 root cause: body-less examples whose only effective argument is a block-pass
/// (`it(&example)`, `super { it(&block) }`) were missed. RuboCop's Parser AST stores
/// `(block_pass ...)` as a send argument, so `skippable_example?` still matches.
/// Prism stores the same syntax in `call.block()` as a `BlockArgumentNode`, so
/// `call.arguments().is_some()` incorrectly returned false and suppressed offenses.
///
/// Fix: treat `BlockArgumentNode` as satisfying the "example has arguments but no
/// real body" requirement, while still requiring the absence of a real `BlockNode`.
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
        Some(recv) => {
            constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
        }
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
                            && is_pending_metadata_value(&assoc.value())
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

fn is_pending_metadata_value(value: &ruby_prism::Node<'_>) -> bool {
    value.as_true_node().is_some()
        || value.as_string_node().is_some()
        || value.as_interpolated_string_node().is_some()
}

fn has_example_arguments(call: &ruby_prism::CallNode<'_>) -> bool {
    call.arguments().is_some()
        || call
            .block()
            .is_some_and(|block| block.as_block_argument_node().is_some())
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
                    && call.block().and_then(|b| b.as_block_node()).is_none()
                    && has_example_arguments(call)
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
