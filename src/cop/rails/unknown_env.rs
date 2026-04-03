use crate::cop::shared::util;
use crate::cop::shared::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks that environments called with `Rails.env` predicates exist.
///
/// ## Corpus findings
///
/// FP root cause: The cop was flagging `Rails.env.starts_with?("production")` and
/// `Rails.env.exclude?("development")` — these are String methods with arguments,
/// not environment predicate checks. RuboCop's NodePattern only matches zero-argument
/// sends like `Rails.env.staging?`, not methods with arguments. Fixed by checking
/// that the outer call has no arguments.
///
/// FN root cause (89 `local?`): The cop hardcoded `local` as always known, but
/// RuboCop only treats `local` as known when `target_rails_version >= 7.1`. The
/// corpus repos have `target_rails_version < 7.1`, so RuboCop flags `local?` but
/// nitrocop didn't. Fixed by removing the hardcoded `local` and only treating it
/// as known when `target_rails_version >= 7.1` or explicitly in Environments config.
///
/// FN root cause (1 `== "profile"`): The cop didn't detect `Rails.env == "string"`
/// or `"string" == Rails.env` comparison patterns. RuboCop matches both `==` and
/// `===` operators with string operands. Fixed by adding comparison detection.
pub struct UnknownEnv;

const KNOWN_ENVS: &[&str] = &["development", "test", "production"];

impl UnknownEnv {
    /// Check if an environment name is known (in configured or default list).
    /// `local` is only known when target_rails_version >= 7.1.
    fn is_known_env(&self, env_name: &str, config: &CopConfig) -> bool {
        let configured_envs = config.get_string_array("Environments");
        if let Some(ref envs) = configured_envs {
            if envs.iter().any(|e| e == env_name) {
                return true;
            }
        } else if KNOWN_ENVS.contains(&env_name) {
            return true;
        }

        // RuboCop adds "local" when target_rails_version >= 7.1
        if env_name == "local" && config.rails_version_at_least(7.1) {
            return true;
        }

        false
    }

    /// Check for `Rails.env.staging?` predicate pattern (no arguments).
    fn check_predicate(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        // outer_method should end with ?
        if !chain.outer_method.ends_with(b"?") {
            return;
        }

        // The outer call must have no arguments — methods like starts_with?("x")
        // or exclude?("y") are String methods, not env predicates.
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if outer_call.arguments().is_some() {
            return;
        }

        // inner should be `env` called on `Rails`
        if chain.inner_method != b"env" {
            return;
        }

        let inner_recv = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Handle both ConstantReadNode (Rails) and ConstantPathNode (::Rails)
        if util::constant_name(&inner_recv) != Some(b"Rails") {
            return;
        }

        // Extract env name (strip trailing ?)
        let env_name = &chain.outer_method[..chain.outer_method.len() - 1];
        let env_str = std::str::from_utf8(env_name).unwrap_or("");

        if self.is_known_env(env_str, config) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Unknown environment `{env_str}`."),
        ));
    }

    /// Check for `Rails.env == "staging"` or `"staging" == Rails.env` patterns.
    /// Also handles `===` operator.
    fn check_equality(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name();
        let method_name = method.as_slice();
        if method_name != b"==" && method_name != b"===" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }
        let arg = &arg_list[0];

        // Two patterns:
        // 1. Rails.env == "string"  (receiver is Rails.env, arg is string)
        // 2. "string" == Rails.env  (receiver is string, arg is Rails.env)

        if self.is_rails_env(&receiver) {
            // Pattern 1: Rails.env == "string"
            if let Some(str_node) = arg.as_string_node() {
                let unescaped = str_node.unescaped();
                let env_name = String::from_utf8_lossy(unescaped);
                if !self.is_known_env(&env_name, config) {
                    let loc = arg.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Unknown environment `{env_name}`."),
                    ));
                }
            }
        } else if self.is_rails_env(arg) {
            // Pattern 2: "string" == Rails.env
            if let Some(str_node) = receiver.as_string_node() {
                let unescaped = str_node.unescaped();
                let env_name = String::from_utf8_lossy(unescaped);
                if !self.is_known_env(&env_name, config) {
                    let loc = receiver.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Unknown environment `{env_name}`."),
                    ));
                }
            }
        }
    }

    /// Check if a node represents `Rails.env` or `::Rails.env`.
    fn is_rails_env(&self, node: &ruby_prism::Node<'_>) -> bool {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return false,
        };
        if call.name().as_slice() != b"env" {
            return false;
        }
        let recv = match call.receiver() {
            Some(r) => r,
            None => return false,
        };
        util::constant_name(&recv) == Some(b"Rails")
    }
}

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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        self.check_predicate(source, node, config, diagnostics);
        self.check_equality(source, node, config, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnknownEnv, "cops/rails/unknown_env");
}
