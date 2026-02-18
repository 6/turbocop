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

        // Check that the method body contains at least one call that forwards the args
        // with *rest_name and &block_name
        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body_src = body.location().as_slice();
        let rest_splat = format!("*{}", String::from_utf8_lossy(&rest_name));
        let block_pass = format!("&{}", String::from_utf8_lossy(&block_name));

        // Simple check: body source must contain both `*args` and `&block` patterns
        let body_str = String::from_utf8_lossy(body_src);
        if !body_str.contains(&rest_splat) || !body_str.contains(&block_pass) {
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
