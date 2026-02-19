use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE};

pub struct RedundantRequireStatement;

/// Features that are always redundant (Ruby 2.0+, well below any supported version).
const ALWAYS_REDUNDANT: &[&[u8]] = &[b"enumerator"];

/// Features redundant since Ruby 2.1+.
const RUBY_21_REDUNDANT: &[&[u8]] = &[b"thread"];

/// Features redundant since Ruby 2.2+.
const RUBY_22_REDUNDANT: &[&[u8]] = &[b"rational", b"complex"];

/// Features redundant since Ruby 2.7+.
const RUBY_27_REDUNDANT: &[&[u8]] = &[b"ruby2_keywords"];

/// Features redundant since Ruby 3.1+.
const RUBY_31_REDUNDANT: &[&[u8]] = &[b"fiber"];

/// Features redundant since Ruby 3.2+.
const RUBY_32_REDUNDANT: &[&[u8]] = &[b"set"];

/// Get the target Ruby version from cop config, defaulting to 2.7
/// (matching RuboCop's default when no version is specified).
fn target_ruby_version(config: &CopConfig) -> f64 {
    config
        .options
        .get("TargetRubyVersion")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
        .unwrap_or(2.7)
}

/// Check if a feature is redundant given the target Ruby version.
fn is_redundant_feature(feature: &[u8], ruby_version: f64) -> bool {
    if ALWAYS_REDUNDANT.iter().any(|f| *f == feature) {
        return true;
    }
    if ruby_version >= 2.1 && RUBY_21_REDUNDANT.iter().any(|f| *f == feature) {
        return true;
    }
    if ruby_version >= 2.2 && RUBY_22_REDUNDANT.iter().any(|f| *f == feature) {
        return true;
    }
    if ruby_version >= 2.7 && RUBY_27_REDUNDANT.iter().any(|f| *f == feature) {
        return true;
    }
    if ruby_version >= 3.1 && RUBY_31_REDUNDANT.iter().any(|f| *f == feature) {
        return true;
    }
    if ruby_version >= 3.2 && RUBY_32_REDUNDANT.iter().any(|f| *f == feature) {
        return true;
    }
    false
}

impl Cop for RedundantRequireStatement {
    fn name(&self) -> &'static str {
        "Lint/RedundantRequireStatement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"require" || call.receiver().is_some() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();
        let string_node = match first_arg.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let feature = string_node.unescaped();
        let ruby_ver = target_ruby_version(config);

        if is_redundant_feature(feature, ruby_ver) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Remove unnecessary `require` statement.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRequireStatement, "cops/lint/redundant_require_statement");
}
