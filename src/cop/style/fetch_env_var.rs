use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FetchEnvVar;

impl FetchEnvVar {
    fn is_env_receiver(node: &ruby_prism::Node<'_>) -> bool {
        // Simple constant: ENV
        if node.as_constant_read_node()
            .map_or(false, |c| c.name().as_slice() == b"ENV")
        {
            return true;
        }
        // Qualified constant: ::ENV (constant_path_node with no parent)
        if let Some(cp) = node.as_constant_path_node() {
            if cp.parent().is_none() && cp.name().map_or(false, |n| n.as_slice() == b"ENV") {
                return true;
            }
        }
        false
    }
}

impl Cop for FetchEnvVar {
    fn name(&self) -> &'static str {
        "Style/FetchEnvVar"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be [] method
        if call.name().as_slice() != b"[]" {
            return Vec::new();
        }

        // Receiver must be ENV
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if !Self::is_env_receiver(&receiver) {
            return Vec::new();
        }

        // Get the argument source for the message
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        // Check AllowedVars
        let _allowed_vars = config.get_string_array("AllowedVars");
        let _default_to_nil = config.get_bool("DefaultToNil", true);

        let arg_loc = arg_list[0].location();
        let arg_src = &source.as_bytes()[arg_loc.start_offset()..arg_loc.end_offset()];
        let arg_str = String::from_utf8_lossy(arg_src);

        // Check if the var is in AllowedVars
        if let Some(ref allowed) = _allowed_vars {
            // Extract the string value from quotes
            let var_name = arg_str.trim_matches('\'').trim_matches('"');
            if allowed.iter().any(|v| v == var_name) {
                return Vec::new();
            }
        }

        let loc = call.location();
        let call_src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        let call_str = String::from_utf8_lossy(call_src);

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let replacement = if _default_to_nil {
            format!("ENV.fetch({}, nil)", arg_str)
        } else {
            format!("ENV.fetch({})", arg_str)
        };

        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}` instead of `{}`.", replacement, call_str),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FetchEnvVar, "cops/style/fetch_env_var");
}
