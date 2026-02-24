use ruby_prism::Visit;

use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockForwarding;

impl Cop for BlockForwarding {
    fn name(&self) -> &'static str {
        "Naming/BlockForwarding"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        // Anonymous block forwarding requires Ruby 3.1+
        // Default TargetRubyVersion is 3.4 (matching RuboCop's behavior when unset)
        let target_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_u64().map(|u| u as f64))
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(3.4);
        if target_version < 3.1 {
            return;
        }

        let enforced_style = config.get_str("EnforcedStyle", "anonymous");
        let _block_forwarding_name = config.get_str("BlockForwardingName", "block");

        if enforced_style != "anonymous" {
            return;
        }

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Check if there's a &block parameter
        let block_param = match params.block() {
            Some(b) => b,
            None => return,
        };

        // If the block param has no name (already anonymous &), skip
        let param_name = match block_param.name() {
            Some(n) => n,
            None => return,
        };

        // Ruby has a syntax error when using anonymous block forwarding with keyword params:
        //   def foo(k:, &); end  => "no anonymous block parameter"
        // This applies to all Ruby versions that support anonymous block forwarding.
        if !params.keywords().is_empty() {
            return;
        }

        let param_name_bytes = param_name.as_slice();

        // Visit the body to check block param usage
        let mut checker = BlockUsageChecker {
            block_name: param_name_bytes,
            only_forwarded: true,
            has_forwarding: false,
            has_any_reference: false,
            used_in_nested_block: false,
            forwarding_locations: Vec::new(),
        };

        if let Some(body) = def_node.body() {
            checker.visit(&body);
        }
        // If body is None, the block param is unused — still an offense

        // If the block param is assigned (e.g., block ||= ...), it's not pure forwarding
        if !checker.only_forwarded {
            return;
        }

        // Ruby 3.1-3.3: anonymous block forwarding inside nested blocks is a syntax error
        if target_version < 3.4 && checker.used_in_nested_block {
            return;
        }

        // Offense if:
        // - Block is forwarded (has_forwarding) and only forwarded (only_forwarded), OR
        // - Block is never referenced at all (unused param should be anonymous &)
        if checker.has_forwarding || !checker.has_any_reference {
            // Offense on the parameter
            let loc = block_param.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use anonymous block forwarding.".to_string(),
            ));
            // Offense on each &block forwarding usage in the body
            for (start, _end) in &checker.forwarding_locations {
                let (line, column) = source.offset_to_line_col(*start);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use anonymous block forwarding.".to_string(),
                ));
            }
        }
    }
}

struct BlockUsageChecker<'a> {
    block_name: &'a [u8],
    only_forwarded: bool,
    has_forwarding: bool,
    has_any_reference: bool,
    used_in_nested_block: bool,
    /// (start_offset, end_offset) for each `&block` forwarding usage in the body
    forwarding_locations: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for BlockUsageChecker<'_> {
    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        // &block in a call argument — this is forwarding
        if let Some(expr) = node.expression() {
            if let Some(local_var) = expr.as_local_variable_read_node() {
                if local_var.name().as_slice() == self.block_name {
                    self.has_forwarding = true;
                    self.has_any_reference = true;
                    // Collect the location of the &block argument for body-level offense
                    let loc = node.location();
                    self.forwarding_locations
                        .push((loc.start_offset(), loc.end_offset()));
                }
            } else {
                // Complex expression like &(block || fallback) — descend into children
                // to detect local variable reads of the block param, which make it
                // a non-forwarding use (can't use anonymous &).
                self.visit(&expr);
            }
        }
    }

    fn visit_yield_node(&mut self, node: &ruby_prism::YieldNode<'pr>) {
        // `yield` forwards the block — this counts as forwarding usage
        self.has_forwarding = true;
        // Continue visiting children (yield arguments)
        ruby_prism::visit_yield_node(self, node);
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.block_name {
            // Block variable used in non-forwarding context (e.g., block.call, if block, block)
            // Note: reads inside block_argument_node are handled there, not here,
            // because visit_call_node visits block args before visiting regular args/receiver.
            self.only_forwarded = false;
            self.has_any_reference = true;
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.block_name {
            // Block param is reassigned — not pure forwarding
            self.only_forwarded = false;
            self.has_any_reference = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.block_name {
            // block ||= ... — not pure forwarding
            self.only_forwarded = false;
            self.has_any_reference = true;
        }
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.block_name {
            self.only_forwarded = false;
            self.has_any_reference = true;
        }
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.block_name {
            self.only_forwarded = false;
            self.has_any_reference = true;
        }
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        if node.name().as_slice() == self.block_name {
            // Multi-assignment target (e.g., `a, block = ary`) — not pure forwarding
            self.only_forwarded = false;
            self.has_any_reference = true;
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check the call's block argument first (so block arg reads are counted as forwarding)
        if let Some(block_arg) = node.block() {
            self.visit(&block_arg);
        }
        // Visit arguments
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        // Visit receiver
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Track whether the block param is used inside a nested block (do..end / {})
        // On Ruby 3.1-3.3, anonymous block forwarding inside nested blocks is a syntax error.
        let saved_forwarding = self.has_forwarding;
        let saved_reference = self.has_any_reference;
        ruby_prism::visit_block_node(self, node);
        // If any forwarding or reference was added during the nested block visit,
        // mark used_in_nested_block
        if self.has_forwarding != saved_forwarding || self.has_any_reference != saved_reference {
            self.used_in_nested_block = true;
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Same treatment as block_node for nested scope tracking
        let saved_forwarding = self.has_forwarding;
        let saved_reference = self.has_any_reference;
        ruby_prism::visit_lambda_node(self, node);
        if self.has_forwarding != saved_forwarding || self.has_any_reference != saved_reference {
            self.used_in_nested_block = true;
        }
    }

    // Don't descend into nested def nodes — they have their own scope
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BlockForwarding, "cops/naming/block_forwarding");
}
