use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct RedundantPredicateMatcher;

/// Maps redundant `be_X` matchers to their built-in equivalents.
const REDUNDANT_MATCHERS: &[(&str, &str)] = &[
    ("be_all", "all"),
    ("be_cover", "cover"),
    ("be_end_with", "end_with"),
    ("be_eql", "eql"),
    ("be_equal", "equal"),
    ("be_exist", "exist"),
    ("be_exists", "exist"),
    ("be_include", "include"),
    ("be_match", "match"),
    ("be_respond_to", "respond_to"),
    ("be_start_with", "start_with"),
];

/// Flags redundant predicate matchers like `be_include(x)` when `include(x)` exists.
impl Cop for RedundantPredicateMatcher {
    fn name(&self) -> &'static str {
        "RSpec/RedundantPredicateMatcher"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return Vec::new();
        }

        // Get the matcher argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let matcher = &arg_list[0];
        let matcher_call = match matcher.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if matcher_call.receiver().is_some() {
            return Vec::new();
        }

        let matcher_name = matcher_call.name().as_slice();
        let matcher_str = match std::str::from_utf8(matcher_name) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Check if this is a redundant matcher
        for &(redundant, builtin) in REDUNDANT_MATCHERS {
            if matcher_str == redundant {
                // Special case: be_all with a block is not redundant
                if redundant == "be_all" {
                    if matcher_call.block().is_some() {
                        return Vec::new();
                    }
                    // Also check if the argument is a non-send node (literal)
                    // be_all(false) or be_all(1) are not redundant
                    if let Some(args) = matcher_call.arguments() {
                        let inner_args: Vec<_> = args.arguments().iter().collect();
                        if !inner_args.is_empty() {
                            let arg = &inner_args[0];
                            // If the arg is a simple call (a matcher), it's redundant
                            // If it's a literal, it's not
                            if arg.as_call_node().is_none() {
                                return Vec::new();
                            }
                        } else {
                            return Vec::new();
                        }
                    } else {
                        return Vec::new();
                    }
                }

                // Special case: be_match without argument is not redundant
                if redundant == "be_match" && matcher_call.arguments().is_none() {
                    return Vec::new();
                }

                let loc = matcher_call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `{builtin}` instead of `{redundant}`."),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantPredicateMatcher,
        "cops/rspec/redundant_predicate_matcher"
    );
}
