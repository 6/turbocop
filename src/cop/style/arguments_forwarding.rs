use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArgumentsForwarding;

impl Cop for ArgumentsForwarding {
    fn name(&self) -> &'static str {
        "Style/ArgumentsForwarding"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allow_only_rest = config.get_bool("AllowOnlyRestArgument", true);
        let _use_anonymous = config.get_bool("UseAnonymousForwarding", true);
        let _redundant_rest = config.get_string_array("RedundantRestArgumentNames").unwrap_or_default();
        let _redundant_kw_rest = config.get_string_array("RedundantKeywordRestArgumentNames").unwrap_or_default();
        let _redundant_block = config.get_string_array("RedundantBlockArgumentNames").unwrap_or_default();

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

        if let Some(block_param) = params.block() {
            if block_param.name().is_none() {
                return Vec::new();
            }
        } else {
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArgumentsForwarding, "cops/style/arguments_forwarding");
}
