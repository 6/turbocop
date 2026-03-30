use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation: 63 FPs caused by flagging argument-less matchers like
/// `be_exist`, `be_exists`, `be_match` etc. RuboCop's cop has an
/// `arguments.empty?` guard that skips matchers without arguments — these are
/// dynamic predicate matchers (calling `object.exist?`) not redundant wrappers
/// around built-in matchers. Fixed by adding the same argument-presence check.
///
/// FN=21 from old-style `.should`/`.should_not` syntax (e.g.,
/// `lines[1].should be_include("value")`). Fixed by adding `should` and
/// `should_not` to the method name check alongside `to`/`not_to`/`to_not`.
///
/// Corpus investigation (2026-03-30): FN=1 in cookpad/mixed_gauge.
/// `expect(repository.all).to all(be_respond_to(:connection))` was missed.
/// RuboCop runs this cop on the redundant matcher send node itself
/// (`RESTRICT_ON_SEND`), so nested matcher calls inside `all(...)` still get
/// visited. nitrocop only inspected the top-level matcher passed to
/// `.to`/`.should`. Fixed by recursively searching the matcher subtree while
/// keeping detection scoped to expectation runners.
pub struct RedundantPredicateMatcher;

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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to"
            && method_name != b"not_to"
            && method_name != b"to_not"
            && method_name != b"should"
            && method_name != b"should_not"
        {
            return;
        }

        // Get the matcher argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let mut redundant_matchers = Vec::new();
        collect_redundant_matchers(&arg_list[0], &mut redundant_matchers);

        for (offset, redundant, builtin) in redundant_matchers {
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use `{builtin}` instead of `{redundant}`."),
            ));
        }
    }
}

fn collect_redundant_matchers<'a>(
    node: &ruby_prism::Node<'a>,
    out: &mut Vec<(usize, &'static str, &'static str)>,
) {
    let Some(call) = node.as_call_node() else {
        return;
    };

    if let Some((redundant, builtin)) = redundant_matcher(&call) {
        out.push((call.location().start_offset(), redundant, builtin));
    }

    if let Some(recv) = call.receiver() {
        collect_redundant_matchers(&recv, out);
    }

    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            collect_redundant_matchers(&arg, out);
        }
    }
}

fn redundant_matcher(call: &ruby_prism::CallNode<'_>) -> Option<(&'static str, &'static str)> {
    let (redundant, builtin) = match call.name().as_slice() {
        b"be_all" => ("be_all", "all"),
        b"be_cover" => ("be_cover", "cover"),
        b"be_end_with" => ("be_end_with", "end_with"),
        b"be_eql" => ("be_eql", "eql"),
        b"be_equal" => ("be_equal", "equal"),
        b"be_exist" => ("be_exist", "exist"),
        b"be_exists" => ("be_exists", "exist"),
        b"be_include" => ("be_include", "include"),
        b"be_match" => ("be_match", "match"),
        b"be_respond_to" => ("be_respond_to", "respond_to"),
        b"be_start_with" => ("be_start_with", "start_with"),
        _ => return None,
    };

    if call.receiver().is_some() || call.block().is_some() {
        return None;
    }

    let args = call.arguments()?;
    let first_arg = args.arguments().iter().next()?;

    // RuboCop's `replaceable_arguments?` guard keeps `be_all` narrow: only
    // matcher-like arguments are replaceable, while literal forms such as
    // `be_all(false)` are not.
    if redundant == "be_all" && first_arg.as_call_node().is_none() {
        return None;
    }

    Some((redundant, builtin))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantPredicateMatcher,
        "cops/rspec/redundant_predicate_matcher"
    );
}
