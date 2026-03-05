use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/PendingWithoutReason - detects pending/skipped specs without a reason string.
///
/// **Root causes of corpus divergence (FP=186, FN=35):**
///
/// 1. **FP (~170): `skip if condition` / `pending unless condition`**
///    When `skip`/`pending` is inside a conditional (if/unless), RuboCop's parent_node()
///    traversal finds the IfNode as parent, which is neither spec_group? nor example?,
///    so no offense. Nitrocop was missing this parent context check entirely.
///
/// 2. **FP (~16): `pending`/`skip` in non-RSpec contexts**
///    RuboCop only flags inside spec groups/examples. Nitrocop was flagging everywhere,
///    including FactoryBot blocks and other non-RSpec contexts.
///
/// 3. **FN (35): x-prefixed methods required block**
///    Nitrocop required `call.block().is_some()` for x-prefixed methods (xit, xdescribe).
///    RuboCop flags them regardless of block presence.
///
/// **Fix:** Rewrote to use `check_source` with a visitor that tracks RSpec context
/// (example group / example nesting). Only flags pending/skip/x-prefixed inside
/// RSpec context, and skips calls inside conditionals (matching RuboCop behavior).
pub struct PendingWithoutReason;

/// x-prefixed methods that skip specs (example group level).
const XGROUP_METHODS: &[&[u8]] = &[b"xcontext", b"xdescribe", b"xfeature"];

/// x-prefixed methods that skip specs (example level).
const XEXAMPLE_METHODS: &[&[u8]] = &[b"xexample", b"xit", b"xscenario", b"xspecify"];

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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = PendingVisitor {
            source,
            cop: self,
            // Track nesting: 0 = not in RSpec, 1+ = inside RSpec group/example
            rspec_depth: 0,
            // Whether we're directly inside an example block (for bare pending/skip)
            in_example: false,
            // Whether we're directly inside a spec group block (for bare pending/skip)
            in_spec_group: false,
            // Whether we're inside a conditional (if/unless) — suppresses bare pending/skip
            in_conditional: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct PendingVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a PendingWithoutReason,
    rspec_depth: usize,
    in_example: bool,
    in_spec_group: bool,
    in_conditional: bool,
    diagnostics: Vec<Diagnostic>,
}

impl PendingVisitor<'_> {
    fn add_offense(&mut self, node: &ruby_prism::CallNode<'_>, label: &str) {
        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Give the reason for {label}."),
        ));
    }

    fn is_rspec_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
        if call.receiver().is_none() {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return util::constant_name(&recv).is_some_and(|n| n == b"RSpec");
        }
        false
    }

    /// Check if a call is an RSpec example group method (describe, context, etc.)
    fn is_example_group_call(call: &ruby_prism::CallNode<'_>) -> bool {
        Self::is_rspec_receiver(call) && is_rspec_example_group(call.name().as_slice())
    }

    /// Check if a call is an RSpec example method (it, specify, etc.)
    fn is_example_call(call: &ruby_prism::CallNode<'_>) -> bool {
        Self::is_rspec_receiver(call) && is_rspec_example(call.name().as_slice())
    }

    /// Check metadata for :skip, :pending, skip: true, pending: true
    fn check_metadata(&mut self, call: &ruby_prism::CallNode<'_>) {
        if call.block().is_none() {
            return;
        }
        let Some(args) = call.arguments() else {
            return;
        };
        for arg in args.arguments().iter() {
            // :skip or :pending symbol metadata
            if let Some(sym) = arg.as_symbol_node() {
                let val = sym.unescaped();
                if val == b"skip" || val == b"pending" {
                    let label = std::str::from_utf8(val).unwrap_or("skip");
                    self.add_offense(call, label);
                }
            }
            // skip: true or pending: true (not a string reason)
            if let Some(kw) = arg.as_keyword_hash_node() {
                for elem in kw.elements().iter() {
                    if let Some(assoc) = elem.as_assoc_node() {
                        if let Some(key_sym) = assoc.key().as_symbol_node() {
                            let key = key_sym.unescaped();
                            if (key == b"skip" || key == b"pending")
                                && assoc.value().as_true_node().is_some()
                            {
                                let label = std::str::from_utf8(key).unwrap_or("skip");
                                self.add_offense(call, label);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for PendingVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check metadata on example/example-group calls (:skip, :pending, skip: true, etc.)
        if (self.rspec_depth > 0 || Self::is_rspec_receiver(node))
            && (Self::is_example_group_call(node) || Self::is_example_call(node))
        {
            self.check_metadata(node);
        }

        // x-prefixed group methods (xdescribe, xcontext, xfeature) — flag when RSpec receiver
        if XGROUP_METHODS.contains(&method_name) && Self::is_rspec_receiver(node) {
            // RuboCop uses "skip" as the label for xdescribe/xcontext
            self.add_offense(node, "skip");
            // Still visit children (the block body may have more offenses)
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    let old_depth = self.rspec_depth;
                    let old_group = self.in_spec_group;
                    self.rspec_depth += 1;
                    self.in_spec_group = true;
                    if let Some(body) = bn.body() {
                        self.visit(&body);
                    }
                    self.rspec_depth = old_depth;
                    self.in_spec_group = old_group;
                    return;
                }
            }
            return;
        }

        // x-prefixed example methods (xit, xspecify, etc.) — flag inside RSpec context
        if XEXAMPLE_METHODS.contains(&method_name)
            && Self::is_rspec_receiver(node)
            && self.rspec_depth > 0
        {
            let label = std::str::from_utf8(method_name).unwrap_or("skip");
            self.add_offense(node, label);
            // Don't visit block body for example-level skips
            return;
        }

        // `pending` or `skip` as example-level methods (with block, used as example definition)
        if (method_name == b"pending" || method_name == b"skip")
            && node.receiver().is_none()
            && node.block().is_some()
            && self.in_spec_group
            && !self.in_example
        {
            let label = std::str::from_utf8(method_name).unwrap_or("skip");
            self.add_offense(node, label);
            // Don't visit block body
            return;
        }

        // Bare `pending` or `skip` without arguments (inside example or spec group)
        if (method_name == b"pending" || method_name == b"skip")
            && node.receiver().is_none()
            && node.arguments().is_none()
            && node.block().is_none()
            && !self.in_conditional
            && (self.in_example || self.in_spec_group)
        {
            let label = std::str::from_utf8(method_name).unwrap_or("skip");
            self.add_offense(node, label);
            return;
        }

        // Enter example group blocks (describe, context, shared_examples, etc.)
        if Self::is_example_group_call(node) {
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    let old_depth = self.rspec_depth;
                    let old_group = self.in_spec_group;
                    let old_example = self.in_example;
                    self.rspec_depth += 1;
                    self.in_spec_group = true;
                    self.in_example = false;
                    if let Some(body) = bn.body() {
                        self.visit(&body);
                    }
                    self.rspec_depth = old_depth;
                    self.in_spec_group = old_group;
                    self.in_example = old_example;
                    return;
                }
            }
        }

        // Enter example blocks (it, specify, etc.)
        if Self::is_example_call(node) && self.rspec_depth > 0 {
            if let Some(block) = node.block() {
                if let Some(bn) = block.as_block_node() {
                    let old_example = self.in_example;
                    let old_group = self.in_spec_group;
                    self.in_example = true;
                    self.in_spec_group = false;
                    if let Some(body) = bn.body() {
                        self.visit(&body);
                    }
                    self.in_example = old_example;
                    self.in_spec_group = old_group;
                    return;
                }
            }
        }

        // Default: visit children
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let old = self.in_conditional;
        self.in_conditional = true;
        ruby_prism::visit_if_node(self, node);
        self.in_conditional = old;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let old = self.in_conditional;
        self.in_conditional = true;
        ruby_prism::visit_unless_node(self, node);
        self.in_conditional = old;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PendingWithoutReason, "cops/rspec/pending_without_reason");
}
