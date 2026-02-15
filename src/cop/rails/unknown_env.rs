use crate::cop::util;
use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnknownEnv;

const KNOWN_ENVS: &[&[u8]] = &[
    b"development?",
    b"test?",
    b"production?",
    b"local?",
];

impl Cop for UnknownEnv {
    fn name(&self) -> &'static str {
        "Rails/UnknownEnv"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let configured_envs = config.get_string_array("Environments");

        // Looking for Rails.env.staging? pattern (3-method chain or 2-method chain)
        // Rails.env is a method chain: ConstantReadNode(Rails).env
        // Then the predicate call on it: Rails.env.staging?
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        // outer_method should end with ?
        if !chain.outer_method.ends_with(b"?") {
            return Vec::new();
        }

        // inner should be `env` called on `Rails`
        if chain.inner_method != b"env" {
            return Vec::new();
        }

        let inner_recv = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
        if util::constant_name(&inner_recv) != Some(b"Rails") {
            return Vec::new();
        }

        // Check if the method is a known env (configured or default)
        // RuboCop adds "local" when target_rails_version >= 7.1; we always include it
        // since Rails 7.1+ is the norm for modern projects
        if let Some(ref envs) = configured_envs {
            let env_name = &chain.outer_method[..chain.outer_method.len() - 1];
            let env_str = std::str::from_utf8(env_name).unwrap_or("");
            if envs.iter().any(|e| e == env_str) || env_str == "local" {
                return Vec::new();
            }
        } else if KNOWN_ENVS.contains(&chain.outer_method) {
            return Vec::new();
        }

        // Extract env name (strip trailing ?)
        let env_name = &chain.outer_method[..chain.outer_method.len() - 1];
        let env_str = String::from_utf8_lossy(env_name);

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Unknown environment `{env_str}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnknownEnv, "cops/rails/unknown_env");
}
