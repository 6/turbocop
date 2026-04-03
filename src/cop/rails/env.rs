use crate::cop::shared::constant_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/Env cop - flags `Rails.env.production?`, `Rails.env.development?`, etc.
///
/// **Complete rewrite (2026-03-08):** The original implementation was completely wrong.
/// It was flagging `ENV['RAILS_ENV']` and `ENV['RACK_ENV']`, suggesting `Rails.env` instead.
/// The actual vendor cop does the opposite: it flags predicate calls on `Rails.env`
/// (like `Rails.env.production?`) and suggests using Feature Flags instead.
///
/// The vendor cop (`vendor/rubocop-rails/lib/rubocop/cop/rails/env.rb`):
/// 1. Matches `send` calls to `:env` where receiver is `Rails` constant
/// 2. Checks if the parent is a predicate method call (method name ends with `?`)
/// 3. Has an ALLOWED_LIST of string-utility predicates that should NOT be flagged
/// 4. Does NOT flag bare `Rails.env` (only when an env-checking predicate is called on it)
///
/// **FN fix (2026-03-15):** The vendor cop checks if the *parent* AST node of `Rails.env`
/// is a predicate, regardless of whether `Rails.env` is the receiver or an argument.
/// This means patterns like `%w[test development].member?(Rails.env)` and
/// `@envs.any?(Rails.env)` are also flagged, because the parent of the `env` send
/// is the predicate call `member?`/`any?`. The previous implementation only checked
/// `Rails.env` as the receiver of a predicate (e.g., `Rails.env.production?`).
///
/// **FN fix (2026-03-16):** `defined?(Rails.env)` was not being flagged. In RuboCop's
/// AST, `defined?` responds to `predicate_method?` (name ends with `?`), so the vendor
/// cop flags it. In Prism, `defined?` is a `DefinedNode` (keyword), not a `CallNode`,
/// so the `visit_call_node` handler never sees it. Added `visit_defined_node` to handle
/// this case. All 13 corpus FNs were `defined?(Rails.env)` patterns.
pub struct Env;

/// Methods ending in `?` that are allowed on `Rails.env` (string utility methods,
/// not environment-checking predicates).
const ALLOWED_PREDICATES: &[&[u8]] = &[
    b"unicode_normalized?",
    b"exclude?",
    b"empty?",
    b"acts_like_string?",
    b"include?",
    b"is_utf8?",
    b"casecmp?",
    b"match?",
    b"starts_with?",
    b"ends_with?",
    b"start_with?",
    b"end_with?",
    b"valid_encoding?",
    b"ascii_only?",
    b"between?",
];

impl Cop for Env {
    fn name(&self) -> &'static str {
        "Rails/Env"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::cop::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = EnvVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct EnvVisitor<'a> {
    cop: &'a Env,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl EnvVisitor<'_> {
    /// Check if a node is a `Rails.env` or `::Rails.env` call.
    fn is_rails_env_call(&self, node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"env" {
                if let Some(recv) = call.receiver() {
                    return constant_predicates::constant_short_name(&recv) == Some(b"Rails");
                }
            }
        }
        false
    }

    /// Report an offense for a `Rails.env` usage within a predicate call.
    /// The `predicate_node` is the outer predicate call; `env_node` is the `Rails.env` call.
    fn report_offense(&mut self, predicate_node: &ruby_prism::CallNode<'_>) {
        let start = predicate_node.location().start_offset();
        let (line, column) = self.source.offset_to_line_col(start);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use Feature Flags or config instead of `Rails.env`.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for EnvVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // We're looking for a predicate method call (ends with '?') where
        // `Rails.env` appears as either the receiver or an argument.
        // This matches the vendor cop which hooks `on_send(:env)` and checks
        // if the parent AST node is a predicate call.
        if method_name.ends_with(b"?") && !ALLOWED_PREDICATES.contains(&method_name) {
            // Case 1: Rails.env is the receiver of the predicate
            // e.g., Rails.env.production?
            if let Some(recv) = node.receiver() {
                if self.is_rails_env_call(&recv) {
                    self.report_offense(node);
                    // Don't fall through to argument check
                    ruby_prism::visit_call_node(self, node);
                    return;
                }
            }

            // Case 2: Rails.env is an argument to the predicate
            // e.g., %w[test development].member?(Rails.env)
            //       @envs.any?(Rails.env)
            if let Some(args) = node.arguments() {
                for arg in args.arguments().iter() {
                    if self.is_rails_env_call(&arg) {
                        self.report_offense(node);
                        break;
                    }
                }
            }
        }

        // Continue visiting child nodes
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_defined_node(&mut self, node: &ruby_prism::DefinedNode<'pr>) {
        // Case 3: Rails.env inside `defined?()` — `defined?` is a keyword
        // (DefinedNode in Prism), not a method call, but RuboCop's AST treats it
        // as a predicate (name ends with `?`). The vendor cop flags
        // `defined?(Rails.env)` because the parent of the `env` send node is the
        // `defined?` node, which responds to `predicate_method?`.
        // e.g., `if defined?(Rails.env)`, `defined? Rails.env`
        let value = node.value();
        if self.is_rails_env_call(&value) {
            let start = node.location().start_offset();
            let (line, column) = self.source.offset_to_line_col(start);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use Feature Flags or config instead of `Rails.env`.".to_string(),
            ));
        }

        // Continue visiting child nodes
        ruby_prism::visit_defined_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Env, "cops/rails/env");
}
