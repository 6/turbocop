use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct PredicateMatcher;

/// Default style `inflected`: flags `expect(foo.bar?).to be_truthy` →
/// prefer `expect(foo).to be_bar`.
impl Cop for PredicateMatcher {
    fn name(&self) -> &'static str {
        "RSpec/PredicateMatcher"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Inflected style: flag `expect(foo.predicate?).to be_truthy/be_falsey/be(true)/be(false)`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return Vec::new();
        }

        // The matcher argument should be be_truthy, be_falsey, a_truthy_value, etc.
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let matcher = &arg_list[0];
        if !is_boolean_matcher(matcher) {
            return Vec::new();
        }

        // The receiver should be `expect(foo.predicate?)`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let expect_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if expect_call.name().as_slice() != b"expect" || expect_call.receiver().is_some() {
            return Vec::new();
        }

        // Get the argument to expect — should be a predicate call (ends with ?)
        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let expect_arg_list: Vec<_> = expect_args.arguments().iter().collect();
        if expect_arg_list.is_empty() {
            return Vec::new();
        }

        let actual = &expect_arg_list[0];
        let predicate_call = match actual.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let pred_name = predicate_call.name().as_slice();
        if !pred_name.ends_with(b"?") {
            return Vec::new();
        }

        // Build the suggested matcher name
        let pred_str = std::str::from_utf8(pred_name).unwrap_or("");
        let suggested = predicate_to_matcher(pred_str);

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Prefer using `{suggested}` matcher over `{pred_str}`."),
        )]
    }
}

fn predicate_to_matcher(pred: &str) -> String {
    let base = &pred[..pred.len() - 1]; // strip trailing ?
    if base == "exist" || base == "exists" {
        "exist".to_string()
    } else if let Some(stripped) = base.strip_prefix("has_") {
        format!("have_{stripped}")
    } else if base == "include" {
        "include".to_string()
    } else if base == "respond_to" {
        "respond_to".to_string()
    } else if base == "is_a" {
        "be_a".to_string()
    } else if base == "instance_of" {
        "be_an_instance_of".to_string()
    } else {
        format!("be_{base}")
    }
}

fn is_boolean_matcher(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.receiver().is_some() {
        return false;
    }

    let name = call.name().as_slice();
    matches!(
        name,
        b"be_truthy"
            | b"be_falsey"
            | b"be_falsy"
            | b"a_truthy_value"
            | b"a_falsey_value"
            | b"a_falsy_value"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PredicateMatcher, "cops/rspec/predicate_matcher");
}
