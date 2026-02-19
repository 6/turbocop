use crate::cop::rspec_rails::RSPEC_RAILS_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, SYMBOL_NODE};

pub struct MinitestAssertions;

/// All Minitest assertion method names we detect.
const ASSERTION_METHODS: &[&[u8]] = &[
    b"assert_equal",
    b"assert_not_equal",
    b"refute_equal",
    b"assert_kind_of",
    b"assert_not_kind_of",
    b"refute_kind_of",
    b"assert_instance_of",
    b"assert_not_instance_of",
    b"refute_instance_of",
    b"assert_includes",
    b"assert_not_includes",
    b"refute_includes",
    b"assert_in_delta",
    b"assert_not_in_delta",
    b"refute_in_delta",
    b"assert_match",
    b"refute_match",
    b"assert_nil",
    b"assert_not_nil",
    b"refute_nil",
    b"assert_empty",
    b"assert_not_empty",
    b"refute_empty",
    b"assert_true",
    b"assert_false",
    b"assert_predicate",
    b"assert_not_predicate",
    b"refute_predicate",
    b"assert_response",
];

fn is_negated(method: &[u8]) -> bool {
    method.starts_with(b"assert_not_") || method.starts_with(b"refute_")
}

fn source_text<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a str {
    let loc = node.location();
    let end = loc.start_offset() + loc.as_slice().len();
    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..end])
        .unwrap_or("?")
}

impl Cop for MinitestAssertions {
    fn name(&self) -> &'static str {
        "RSpecRails/MinitestAssertions"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_RAILS_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        if !ASSERTION_METHODS.iter().any(|m| *m == method) {
            return;
        }

        // Must be a bare call (no receiver)
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        let negated = is_negated(method);
        let runner = if negated { "not_to" } else { "to" };

        let preferred = match method {
            // Two-arg assertions: assert_equal(expected, actual [, msg])
            b"assert_equal" | b"assert_not_equal" | b"refute_equal" => {
                if arg_list.len() < 2 {
                    return;
                }
                let expected = source_text(source, &arg_list[0]);
                let actual = source_text(source, &arg_list[1]);
                format!("expect({actual}).{runner} eq({expected})")
            }

            // Two-arg: assert_kind_of(klass, actual [, msg])
            b"assert_kind_of" | b"assert_not_kind_of" | b"refute_kind_of" => {
                if arg_list.len() < 2 {
                    return;
                }
                let expected = source_text(source, &arg_list[0]);
                let actual = source_text(source, &arg_list[1]);
                format!("expect({actual}).{runner} be_a_kind_of({expected})")
            }

            // Two-arg: assert_instance_of(klass, actual [, msg])
            b"assert_instance_of" | b"assert_not_instance_of" | b"refute_instance_of" => {
                if arg_list.len() < 2 {
                    return;
                }
                let expected = source_text(source, &arg_list[0]);
                let actual = source_text(source, &arg_list[1]);
                format!("expect({actual}).{runner} be_an_instance_of({expected})")
            }

            // Two-arg: assert_includes(collection, member [, msg])
            b"assert_includes" | b"assert_not_includes" | b"refute_includes" => {
                if arg_list.len() < 2 {
                    return;
                }
                let collection = source_text(source, &arg_list[0]);
                let member = source_text(source, &arg_list[1]);
                format!("expect({collection}).{runner} include({member})")
            }

            // assert_in_delta(expected, actual [, delta [, msg]])
            b"assert_in_delta" | b"assert_not_in_delta" | b"refute_in_delta" => {
                if arg_list.len() < 2 {
                    return;
                }
                let expected = source_text(source, &arg_list[0]);
                let actual = source_text(source, &arg_list[1]);
                let delta = if arg_list.len() >= 3 {
                    source_text(source, &arg_list[2]).to_string()
                } else {
                    "0.001".to_string()
                };
                format!("expect({actual}).{runner} be_within({delta}).of({expected})")
            }

            // Two-arg: assert_match(pattern, actual [, msg])
            b"assert_match" | b"refute_match" => {
                if arg_list.len() < 2 {
                    return;
                }
                let pattern = source_text(source, &arg_list[0]);
                let actual = source_text(source, &arg_list[1]);
                format!("expect({actual}).{runner} match({pattern})")
            }

            // One-arg: assert_nil(actual [, msg])
            b"assert_nil" | b"assert_not_nil" | b"refute_nil" => {
                if arg_list.is_empty() {
                    return;
                }
                let actual = source_text(source, &arg_list[0]);
                format!("expect({actual}).{runner} eq(nil)")
            }

            // One-arg: assert_empty(actual [, msg])
            b"assert_empty" | b"assert_not_empty" | b"refute_empty" => {
                if arg_list.is_empty() {
                    return;
                }
                let actual = source_text(source, &arg_list[0]);
                format!("expect({actual}).{runner} be_empty")
            }

            // One-arg: assert_true(actual [, msg])
            b"assert_true" => {
                if arg_list.is_empty() {
                    return;
                }
                let actual = source_text(source, &arg_list[0]);
                format!("expect({actual}).to be(true)")
            }

            // One-arg: assert_false(actual [, msg])
            b"assert_false" => {
                if arg_list.is_empty() {
                    return;
                }
                let actual = source_text(source, &arg_list[0]);
                format!("expect({actual}).to be(false)")
            }

            // Two-arg: assert_predicate(subject, predicate [, msg])
            b"assert_predicate" | b"assert_not_predicate" | b"refute_predicate" => {
                if arg_list.len() < 2 {
                    return;
                }
                // The predicate must be a symbol ending in ?
                let pred_sym = match arg_list[1].as_symbol_node() {
                    Some(s) => s,
                    None => return,
                };
                let pred_name = pred_sym.unescaped();
                let pred_str = std::str::from_utf8(pred_name.as_ref()).unwrap_or("");
                if !pred_str.ends_with('?') {
                    return;
                }
                let actual = source_text(source, &arg_list[0]);
                let be_method = &pred_str[..pred_str.len() - 1]; // strip trailing ?
                format!("expect({actual}).{runner} be_{be_method}")
            }

            // One-arg: assert_response(expected [, msg])
            b"assert_response" => {
                if arg_list.is_empty() {
                    return;
                }
                let expected = source_text(source, &arg_list[0]);
                format!("expect(response).to have_http_status({expected})")
            }

            _ => return,
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{preferred}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MinitestAssertions, "cops/rspecrails/minitest_assertions");
}
