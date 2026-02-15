use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ExpectActual;

impl Cop for ExpectActual {
    fn name(&self) -> &'static str {
        "RSpec/ExpectActual"
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
        // Look for expect(literal).to/to_not/not_to matcher(args) chains
        // RuboCop only flags when the full chain has a matcher with arguments.
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        // Must be a runner method (.to, .to_not, .not_to)
        if method_name != b"to" && method_name != b"to_not" && method_name != b"not_to" {
            return Vec::new();
        }

        // Receiver must be expect(literal)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let expect_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if expect_call.name().as_slice() != b"expect" || expect_call.receiver().is_some() {
            return Vec::new();
        }

        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let expect_arg_list: Vec<ruby_prism::Node<'_>> = expect_args.arguments().iter().collect();
        if expect_arg_list.len() != 1 {
            return Vec::new();
        }

        let literal_arg = &expect_arg_list[0];
        if !is_literal_value(literal_arg) {
            return Vec::new();
        }

        // Check that the matcher has arguments (RuboCop requires this).
        // `expect(5).to eq(price)` → matcher `eq` has arg `price` → flagged
        // `expect(".foo").to be_present` → `be_present` has no args → NOT flagged
        let matcher_args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let matcher_list: Vec<ruby_prism::Node<'_>> = matcher_args.arguments().iter().collect();
        if matcher_list.is_empty() {
            return Vec::new();
        }

        // The matcher call itself must have arguments
        let matcher = &matcher_list[0];
        if let Some(matcher_call) = matcher.as_call_node() {
            let matcher_name = matcher_call.name().as_slice();
            // Skip route_to and be_routable matchers
            if matcher_name == b"route_to" || matcher_name == b"be_routable" {
                return Vec::new();
            }
            // Matcher must have arguments (eq(something), be(something), etc.)
            if matcher_call.arguments().is_none() {
                // Also check for `be == something` pattern
                return Vec::new();
            }
        }

        let loc = literal_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Provide the actual value you are testing to `expect(...)`.".to_string(),
        )]
    }
}

fn is_literal_value(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_regular_expression_node().is_some()
    {
        return true;
    }

    // String without interpolation
    if let Some(s) = node.as_string_node() {
        // Check it's not an interpolated string
        let _ = s;
        return true;
    }

    // Symbol without interpolation
    if node.as_symbol_node().is_some() {
        return true;
    }

    // Array with all literal elements
    if let Some(arr) = node.as_array_node() {
        let elements: Vec<ruby_prism::Node<'_>> = arr.elements().iter().collect();
        if elements.iter().all(|e| is_literal_value(e)) {
            return true;
        }
    }

    // Hash with all literal keys and values
    if let Some(hash) = node.as_hash_node() {
        let pairs: Vec<ruby_prism::Node<'_>> = hash.elements().iter().collect();
        if pairs.iter().all(|p| {
            if let Some(assoc) = p.as_assoc_node() {
                is_literal_value(&assoc.key()) && is_literal_value(&assoc.value())
            } else {
                false
            }
        }) {
            return true;
        }
    }

    // Range with literal endpoints
    if let Some(range) = node.as_range_node() {
        let left_ok = range.left().is_none() || range.left().is_some_and(|l| is_literal_value(&l));
        let right_ok =
            range.right().is_none() || range.right().is_some_and(|r| is_literal_value(&r));
        if left_ok && right_ok {
            return true;
        }
    }

    // Keyword hash (bare key-value pairs used in method args)
    if let Some(kh) = node.as_keyword_hash_node() {
        let elems: Vec<ruby_prism::Node<'_>> = kh.elements().iter().collect();
        if elems.iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_literal_value(&assoc.key()) && is_literal_value(&assoc.value())
            } else {
                false
            }
        }) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExpectActual, "cops/rspec/expect_actual");
}
