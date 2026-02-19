use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, DEF_NODE, FORWARDING_PARAMETER_NODE, LOCAL_VARIABLE_READ_NODE, REST_PARAMETER_NODE, SPLAT_NODE};

pub struct ArgumentsForwarding;

impl Cop for ArgumentsForwarding {
    fn name(&self) -> &'static str {
        "Style/ArgumentsForwarding"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, DEF_NODE, FORWARDING_PARAMETER_NODE, LOCAL_VARIABLE_READ_NODE, REST_PARAMETER_NODE, SPLAT_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_only_rest = config.get_bool("AllowOnlyRestArgument", true);
        let _use_anonymous = config.get_bool("UseAnonymousForwarding", true);
        let redundant_rest = config.get_string_array("RedundantRestArgumentNames")
            .unwrap_or_else(|| vec!["args".to_string(), "arguments".to_string()]);
        let _redundant_kw_rest = config.get_string_array("RedundantKeywordRestArgumentNames")
            .unwrap_or_else(|| vec!["kwargs".to_string(), "options".to_string(), "opts".to_string()]);
        let redundant_block = config.get_string_array("RedundantBlockArgumentNames")
            .unwrap_or_else(|| vec!["blk".to_string(), "block".to_string(), "proc".to_string()]);

        // `...` forwarding requires Ruby >= 2.7
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(3.4);
        if ruby_version < 2.7 {
            return Vec::new();
        }

        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let params = match def_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Check for ... forwarding parameter already being used
        if params.keyword_rest().is_some() {
            if let Some(kw_rest) = params.keyword_rest() {
                if kw_rest.as_forwarding_parameter_node().is_some() {
                    return Vec::new(); // Already using ...
                }
            }
        }

        // Check if the method has *args, **kwargs, &block pattern
        let has_rest = params.rest().is_some();
        let has_block = params.block().is_some();

        if !has_rest || !has_block {
            return Vec::new();
        }

        // Must not have regular positional params, optional params, or keyword params
        if !params.requireds().is_empty()
            || !params.optionals().is_empty()
            || !params.keywords().is_empty()
            || params.posts().iter().next().is_some()
        {
            return Vec::new();
        }

        // Get the rest and block parameter names
        if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                if rest_param.name().is_none() {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        }

        let block_name = if let Some(block_param) = params.block() {
            match block_param.name() {
                Some(n) => n.as_slice().to_vec(),
                None => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        let rest_name = if let Some(rest) = params.rest() {
            if let Some(rest_param) = rest.as_rest_parameter_node() {
                rest_param.name().map(|n| n.as_slice().to_vec()).unwrap_or_default()
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        };

        // RuboCop checks if parameter names are "redundant" (i.e., in the
        // configurable lists of meaningless names). If AllowOnlyRestArgument is
        // true and the block argument name is NOT redundant, RuboCop won't
        // suggest `...` forwarding because the block arg has a meaningful name
        // and might be used for readability.
        if allow_only_rest {
            let block_name_str = String::from_utf8_lossy(&block_name).to_string();
            if !redundant_block.iter().any(|n| n == &block_name_str) {
                return Vec::new();
            }
        }

        // Similarly, check if the rest arg name is redundant when AllowOnlyRestArgument
        // is true. If it's not in the RedundantRestArgumentNames list, don't flag.
        if allow_only_rest {
            let rest_name_str = String::from_utf8_lossy(&rest_name).to_string();
            if !redundant_rest.iter().any(|n| n == &rest_name_str) {
                return Vec::new();
            }
        }

        // Check that the method body contains at least one call that forwards
        // *rest and &block to the SAME call. Without this, cases like
        // `new(*args).tap(&block)` would be incorrectly flagged.
        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        if !has_forwarding_call(&body, &rest_name, &block_name) {
            return Vec::new();
        }

        let loc = params.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use shorthand syntax `...` for arguments forwarding.".to_string(),
        )]
    }
}

/// Check if any call node in the tree forwards both *rest_name and &block_name
/// in the same argument list.
fn has_forwarding_call(node: &ruby_prism::Node<'_>, rest_name: &[u8], block_name: &[u8]) -> bool {
    let mut finder = ForwardingCallFinder {
        rest_name: rest_name.to_vec(),
        block_name: block_name.to_vec(),
        found: false,
    };
    finder.visit(node);
    finder.found
}

struct ForwardingCallFinder {
    rest_name: Vec<u8>,
    block_name: Vec<u8>,
    found: bool,
}

impl<'pr> Visit<'pr> for ForwardingCallFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let mut has_splat = false;
        let mut has_block = false;

        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                if let Some(splat) = arg.as_splat_node() {
                    if let Some(expr) = splat.expression() {
                        if let Some(lvar) = expr.as_local_variable_read_node() {
                            if lvar.name().as_slice() == self.rest_name.as_slice() {
                                has_splat = true;
                            }
                        }
                    }
                }
            }
        }

        if let Some(block) = node.block() {
            if let Some(block_arg) = block.as_block_argument_node() {
                if let Some(expr) = block_arg.expression() {
                    if let Some(lvar) = expr.as_local_variable_read_node() {
                        if lvar.name().as_slice() == self.block_name.as_slice() {
                            has_block = true;
                        }
                    }
                }
            }
        }

        if has_splat && has_block {
            self.found = true;
            return;
        }

        // Continue recursing
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArgumentsForwarding, "cops/style/arguments_forwarding");
}
