use ruby_prism::Visit;

use crate::cop::node_type::{DEF_NODE, LOCAL_VARIABLE_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct BlockForwarding;

impl Cop for BlockForwarding {
    fn name(&self) -> &'static str {
        "Naming/BlockForwarding"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE, LOCAL_VARIABLE_READ_NODE]
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

        let param_name_bytes = param_name.as_slice();

        // Check if the block is only used for forwarding (passed as &name to other calls)
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Check if the block parameter is only used as &name in call arguments
        let mut checker = BlockUsageChecker {
            block_name: param_name_bytes,
            only_forwarded: true,
            has_forwarding: false,
        };
        checker.visit(&body);

        if checker.only_forwarded && checker.has_forwarding {
            let loc = block_param.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use anonymous block forwarding.".to_string(),
            ));
        }
    }
}

struct BlockUsageChecker<'a> {
    block_name: &'a [u8],
    only_forwarded: bool,
    has_forwarding: bool,
}

impl<'pr> Visit<'pr> for BlockUsageChecker<'_> {
    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        // &block in a call argument — this is forwarding
        if let Some(expr) = node.expression() {
            if let Some(local_var) = expr.as_local_variable_read_node() {
                if local_var.name().as_slice() == self.block_name {
                    self.has_forwarding = true;
                    return;
                }
            }
        }
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if node.name().as_slice() == self.block_name {
            // block variable used in non-forwarding context
            self.only_forwarded = false;
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check the call's block argument
        if let Some(block_arg) = node.block() {
            self.visit(&ruby_prism::Node::from(block_arg));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BlockForwarding, "cops/naming/block_forwarding");
}
