use ruby_prism::Visit;

use crate::cop::node_type::{
    BLOCK_ARGUMENT_NODE, DEF_NODE, FORWARDING_PARAMETER_NODE, LOCAL_VARIABLE_READ_NODE,
    REST_PARAMETER_NODE, SPLAT_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArgumentsForwarding;

impl Cop for ArgumentsForwarding {
    fn name(&self) -> &'static str {
        "Style/ArgumentsForwarding"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            DEF_NODE,
            FORWARDING_PARAMETER_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REST_PARAMETER_NODE,
            SPLAT_NODE,
        ]
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
        let allow_only_rest = config.get_bool("AllowOnlyRestArgument", true);
        let _use_anonymous = config.get_bool("UseAnonymousForwarding", false);
        let redundant_rest = config
            .get_string_array("RedundantRestArgumentNames")
            .unwrap_or_else(|| vec!["args".to_string(), "arguments".to_string()]);
        let redundant_kw_rest = config
            .get_string_array("RedundantKeywordRestArgumentNames")
            .unwrap_or_else(|| {
                vec![
                    "kwargs".to_string(),
                    "options".to_string(),
                    "opts".to_string(),
                ]
            });
        let redundant_block = config
            .get_string_array("RedundantBlockArgumentNames")
            .unwrap_or_else(|| vec!["blk".to_string(), "block".to_string(), "proc".to_string()]);

        // `...` forwarding requires Ruby >= 2.7
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(3.4);
        if ruby_version < 2.7 {
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

        // Check for ... forwarding parameter already being used
        if params.keyword_rest().is_some() {
            if let Some(kw_rest) = params.keyword_rest() {
                if kw_rest.as_forwarding_parameter_node().is_some() {
                    return; // Already using ...
                }
            }
        }

        // Check if the method has *args, **kwargs, &block pattern
        let has_rest = params.rest().is_some();
        let has_block = params.block().is_some();

        if !has_rest || !has_block {
            return;
        }

        // Must not have regular positional params, optional params, or keyword params
        if !params.requireds().is_empty()
            || !params.optionals().is_empty()
            || !params.keywords().is_empty()
            || params.posts().iter().next().is_some()
        {
            return;
        }

        // Get the rest and block parameter names
        if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                if rest_param.name().is_none() {
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        }

        let block_name = if let Some(block_param) = params.block() {
            match block_param.name() {
                Some(n) => n.as_slice().to_vec(),
                None => return,
            }
        } else {
            return;
        };

        let rest_name = if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                rest_param
                    .name()
                    .map(|n| n.as_slice().to_vec())
                    .unwrap_or_default()
            } else {
                return;
            }
        } else {
            return;
        };

        // RuboCop checks if parameter names are "redundant" (i.e., in the
        // configurable lists of meaningless names). If AllowOnlyRestArgument is
        // true and the block argument name is NOT redundant, RuboCop won't
        // suggest `...` forwarding because the block arg has a meaningful name
        // and might be used for readability.
        if allow_only_rest {
            let block_name_str = String::from_utf8_lossy(&block_name).to_string();
            if !redundant_block.iter().any(|n| n == &block_name_str) {
                return;
            }
        }

        // Similarly, check if the rest arg name is redundant when AllowOnlyRestArgument
        // is true. If it's not in the RedundantRestArgumentNames list, don't flag.
        if allow_only_rest {
            let rest_name_str = String::from_utf8_lossy(&rest_name).to_string();
            if !redundant_rest.iter().any(|n| n == &rest_name_str) {
                return;
            }
        }

        // If there's a keyword_rest parameter (**opts), check that the name is
        // redundant too. Also collect the name for the reference check.
        let kwrest_name = if let Some(kw_rest) = params.keyword_rest() {
            if let Some(kw_rest_param) = kw_rest.as_keyword_rest_parameter_node() {
                let name = kw_rest_param
                    .name()
                    .map(|n| n.as_slice().to_vec())
                    .unwrap_or_default();
                if allow_only_rest && !name.is_empty() {
                    let name_str = String::from_utf8_lossy(&name).to_string();
                    if !redundant_kw_rest.iter().any(|n| n == &name_str) {
                        return;
                    }
                }
                Some(name)
            } else {
                None
            }
        } else {
            None
        };

        // Check that the method body contains at least one call that forwards
        // *rest and &block to the SAME call. Without this, cases like
        // `new(*args).tap(&block)` would be incorrectly flagged.
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Check that args/block are not referenced outside of forwarding context.
        // If `args` is used as a local variable (e.g., `args.first`), we can't
        // replace with `...`.
        let referenced = non_forwarding_references(&body);
        let rest_name_str = String::from_utf8_lossy(&rest_name).to_string();
        let block_name_str = String::from_utf8_lossy(&block_name).to_string();
        if referenced.contains(&rest_name_str) || referenced.contains(&block_name_str) {
            return;
        }
        if let Some(ref kw_name) = kwrest_name {
            if !kw_name.is_empty() {
                let kw_name_str = String::from_utf8_lossy(kw_name).to_string();
                if referenced.contains(&kw_name_str) {
                    return;
                }
            }
        }

        let forwarding_calls =
            find_forwarding_calls(&body, &rest_name, &block_name, kwrest_name.as_deref());
        if forwarding_calls.is_empty() {
            return;
        }

        let loc = params.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use shorthand syntax `...` for arguments forwarding.".to_string(),
        ));

        // Also report on each forwarding call site (RuboCop reports both)
        for (start, _end_off) in &forwarding_calls {
            let (call_line, call_col) = source.offset_to_line_col(*start);
            diagnostics.push(self.diagnostic(
                source,
                call_line,
                call_col,
                "Use shorthand syntax `...` for arguments forwarding.".to_string(),
            ));
        }
    }
}

/// Find local variable names that are referenced outside of forwarding contexts
/// (i.e., not inside splat, kwsplat, or block_pass nodes).
fn non_forwarding_references(node: &ruby_prism::Node<'_>) -> std::collections::HashSet<String> {
    let mut finder = NonForwardingRefFinder {
        in_forwarding_context: false,
        referenced: std::collections::HashSet::new(),
    };
    finder.visit(node);
    finder.referenced
}

struct NonForwardingRefFinder {
    in_forwarding_context: bool,
    referenced: std::collections::HashSet<String>,
}

impl<'pr> Visit<'pr> for NonForwardingRefFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if !self.in_forwarding_context {
            let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
            self.referenced.insert(name);
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = String::from_utf8_lossy(node.name().as_slice()).to_string();
        self.referenced.insert(name);
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_splat_node(&mut self, node: &ruby_prism::SplatNode<'pr>) {
        let was = self.in_forwarding_context;
        self.in_forwarding_context = true;
        ruby_prism::visit_splat_node(self, node);
        self.in_forwarding_context = was;
    }

    fn visit_assoc_splat_node(&mut self, node: &ruby_prism::AssocSplatNode<'pr>) {
        let was = self.in_forwarding_context;
        self.in_forwarding_context = true;
        ruby_prism::visit_assoc_splat_node(self, node);
        self.in_forwarding_context = was;
    }

    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        let was = self.in_forwarding_context;
        self.in_forwarding_context = true;
        ruby_prism::visit_block_argument_node(self, node);
        self.in_forwarding_context = was;
    }

    // Don't recurse into nested defs
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
}

/// Find all call nodes in the tree that forward both *rest_name and &block_name
/// in the same argument list. Returns (start_offset, end_offset) of the forwarded args range.
fn find_forwarding_calls(
    node: &ruby_prism::Node<'_>,
    rest_name: &[u8],
    block_name: &[u8],
    _kwrest_name: Option<&[u8]>,
) -> Vec<(usize, usize)> {
    let mut finder = ForwardingCallFinder {
        rest_name: rest_name.to_vec(),
        block_name: block_name.to_vec(),
        locations: Vec::new(),
    };
    finder.visit(node);
    finder.locations
}

struct ForwardingCallFinder {
    rest_name: Vec<u8>,
    block_name: Vec<u8>,
    locations: Vec<(usize, usize)>,
}

impl ForwardingCallFinder {
    /// Check if the given arguments and block forward *rest_name and &block_name.
    /// Returns the (start_offset, end_offset) of the forwarded args range if found.
    fn check_args_and_block(
        &self,
        arguments: Option<ruby_prism::ArgumentsNode<'_>>,
        block: Option<ruby_prism::Node<'_>>,
    ) -> Option<(usize, usize)> {
        let mut splat_start: Option<usize> = None;
        let mut _splat_end: Option<usize> = None;
        let mut block_end: Option<usize> = None;

        if let Some(args) = &arguments {
            for arg in args.arguments().iter() {
                if let Some(splat) = arg.as_splat_node() {
                    if let Some(expr) = splat.expression() {
                        if let Some(lvar) = expr.as_local_variable_read_node() {
                            if lvar.name().as_slice() == self.rest_name.as_slice() {
                                let loc = splat.location();
                                if splat_start.is_none()
                                    || loc.start_offset() < splat_start.unwrap()
                                {
                                    splat_start = Some(loc.start_offset());
                                }
                                _splat_end = Some(loc.end_offset());
                            }
                        }
                    }
                }
            }
        }

        if let Some(block_node) = block {
            if let Some(block_arg) = block_node.as_block_argument_node() {
                if let Some(expr) = block_arg.expression() {
                    if let Some(lvar) = expr.as_local_variable_read_node() {
                        if lvar.name().as_slice() == self.block_name.as_slice() {
                            block_end = Some(block_arg.location().end_offset());
                        }
                    }
                }
            }
        }

        if let (Some(start), Some(end)) = (splat_start, block_end) {
            Some((start, end))
        } else {
            None
        }
    }
}

impl<'pr> Visit<'pr> for ForwardingCallFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(range) = self.check_args_and_block(node.arguments(), node.block()) {
            self.locations.push(range);
        }

        // Continue recursing to find all forwarding calls
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        if let Some(range) = self.check_args_and_block(node.arguments(), node.block()) {
            self.locations.push(range);
        }

        // Continue recursing
        ruby_prism::visit_super_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArgumentsForwarding, "cops/style/arguments_forwarding");

    #[test]
    fn detects_triple_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args, **opts, &block)\n  bar(*args, **opts, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect triple forwarding (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn detects_super_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args, &block)\n  super(*args, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect super forwarding (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn no_false_positive_different_calls() {
        use crate::testutil::run_cop_full;
        // *args and &block used in different calls — cannot use ...
        let source = b"def foo(*args, &block)\n  bar(*args)\n  baz(&block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            0,
            "should not detect when args forwarded to different calls: {:?}",
            diags
        );
    }

    #[test]
    fn detects_self_class_method_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def self.foo(*args, &block)\n  bar(*args, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect singleton method forwarding (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn detects_forwarding_without_kwargs() {
        use crate::testutil::run_cop_full;
        // Method has **opts but they're not part of redundant names check
        let source = b"def foo(*args, **options, &block)\n  bar(*args, **options, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect forwarding with options (def + call): {:?}",
            diags
        );
    }

    #[test]
    fn no_false_positive_args_referenced_directly() {
        use crate::testutil::run_cop_full;
        // args is used as a local variable (args.first), not just in *args
        let source = b"def foo(*args, &block)\n  bar(*args, &block)\n  args.first\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            0,
            "should not flag when args is referenced directly: {:?}",
            diags
        );
    }

    #[test]
    fn no_false_positive_block_referenced_directly() {
        use crate::testutil::run_cop_full;
        // block is called directly
        let source = b"def foo(*args, &block)\n  bar(*args, &block)\n  block.call\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            0,
            "should not flag when block is referenced directly: {:?}",
            diags
        );
    }

    #[test]
    fn detects_super_with_triple_forwarding() {
        use crate::testutil::run_cop_full;
        let source = b"def foo(*args, **opts, &block)\n  super(*args, **opts, &block)\nend\n";
        let diags = run_cop_full(&ArgumentsForwarding, source);
        assert_eq!(
            diags.len(),
            2,
            "should detect super with triple forwarding (def + call): {:?}",
            diags
        );
    }
}
