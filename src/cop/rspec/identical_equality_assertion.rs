use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for equality assertions with identical expressions on both sides.
///
/// ## Investigation notes (2026-03)
///
/// - RuboCop only matches `.to` when it receives a single matcher argument.
///   `expect(x).to eq(x), message` is therefore a no-offense shape, so this cop
///   must reject `.to` calls with extra message arguments.
/// - Prism represents `expect(x).to be == x` as a `==` call whose receiver is a
///   bare `be` matcher. Support that operator form in addition to `eq/eql/be(...)`.
/// - The fingerprint still normalizes Parser-vs-Prism surface differences for
///   `Obj.method` vs `Obj::method`, empty arrays, and equivalent regex escapes.
pub struct IdenticalEqualityAssertion;

impl Cop for IdenticalEqualityAssertion {
    fn name(&self) -> &'static str {
        "RSpec/IdenticalEqualityAssertion"
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
        // Look for expect(X).to eq(X) / eql(X) / be(X)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Only flag `.to` (not `.not_to`)
        if method_name != b"to" {
            return;
        }

        // Receiver must be expect(X)
        let expect_call = match call.receiver() {
            Some(recv) => match recv.as_call_node() {
                Some(c) => c,
                None => return,
            },
            None => return,
        };

        if expect_call.name().as_slice() != b"expect" {
            return;
        }

        // Get the expect argument
        let expect_args = match expect_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let expect_arg_list: Vec<_> = expect_args.arguments().iter().collect();
        if expect_arg_list.len() != 1 {
            return;
        }

        let expect_arg = &expect_arg_list[0];

        // Get the matcher call (eq/eql/be)
        let matcher_args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let matcher_arg_list: Vec<_> = matcher_args.arguments().iter().collect();
        if matcher_arg_list.len() != 1 {
            return;
        }

        let matcher_call = match matcher_arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let matcher_arg = match matcher_argument(&matcher_call) {
            Some(arg) => arg,
            None => return,
        };

        // Compare AST structure of both expressions (not source text).
        // RuboCop uses `left == right` on Parser gem AST nodes, which compares
        // structure recursively. We build fingerprints that normalize surface
        // syntax differences (`.` vs `::`, `%i{}` vs `[]`, regex escapes).
        let mut expect_fp = Vec::new();
        let mut matcher_fp = Vec::new();
        ast_fingerprint(source.as_bytes(), expect_arg, &mut expect_fp);
        ast_fingerprint(source.as_bytes(), &matcher_arg, &mut matcher_fp);

        if expect_fp == matcher_fp {
            let loc = expect_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Identical expressions on both sides of the equality may indicate a flawed test.".to_string(),
            ));
        }
    }
}

fn matcher_argument<'pr>(
    matcher_call: &ruby_prism::CallNode<'pr>,
) -> Option<ruby_prism::Node<'pr>> {
    let matcher_args = matcher_call.arguments()?;
    let mut matcher_arg_list = matcher_args.arguments().iter();
    let matcher_arg = matcher_arg_list.next()?;
    if matcher_arg_list.next().is_some() {
        return None;
    }

    match matcher_call.name().as_slice() {
        b"eq" | b"eql" | b"be" => {
            if matcher_call.receiver().is_some() {
                return None;
            }
        }
        b"==" => {
            let be_call = matcher_call.receiver()?.as_call_node()?;
            if !is_bare_matcher_call(&be_call, b"be") {
                return None;
            }
        }
        _ => return None,
    }

    Some(matcher_arg)
}

fn is_bare_matcher_call(call: &ruby_prism::CallNode<'_>, name: &[u8]) -> bool {
    if call.name().as_slice() != name || call.receiver().is_some() {
        return false;
    }

    match call.arguments() {
        Some(args) => args.arguments().iter().next().is_none(),
        None => true,
    }
}

/// Build a structural fingerprint for a node that normalizes surface syntax
/// differences to match RuboCop's Parser-gem AST equality semantics.
///
/// Key normalizations:
/// - CallNode: `.` and `::` call operators are treated identically
/// - ArrayNode: compared structurally (handles `%i{}` vs `[]`)
/// - RegularExpressionNode: uses `unescaped()` content (handles `\§` vs `§`)
/// - StringNode/SymbolNode: uses `unescaped()` content
/// - All other nodes: falls back to source text comparison
fn ast_fingerprint(bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    // CallNode: normalize call operator
    if let Some(call) = node.as_call_node() {
        out.extend_from_slice(b"C:");
        if let Some(recv) = call.receiver() {
            ast_fingerprint(bytes, &recv, out);
            // Normalize `.` and `::` to the same separator
            out.push(b'.');
        }
        out.extend_from_slice(call.name().as_slice());
        out.push(b'(');
        if let Some(args) = call.arguments() {
            for (i, arg) in args.arguments().iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                ast_fingerprint(bytes, &arg, out);
            }
        }
        out.push(b')');
        if let Some(block) = call.block() {
            out.push(b'{');
            ast_fingerprint(bytes, &block, out);
            out.push(b'}');
        }
        return;
    }

    // ArrayNode: structural comparison (handles %i{} vs [] etc.)
    if let Some(array) = node.as_array_node() {
        out.extend_from_slice(b"A:[");
        for (i, elem) in array.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            ast_fingerprint(bytes, &elem, out);
        }
        out.push(b']');
        return;
    }

    // HashNode: structural comparison
    if let Some(hash) = node.as_hash_node() {
        out.extend_from_slice(b"H:{");
        for (i, elem) in hash.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            ast_fingerprint(bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    // KeywordHashNode: same structure as HashNode (keyword args like `foo(a: 1)`)
    if let Some(hash) = node.as_keyword_hash_node() {
        out.extend_from_slice(b"H:{");
        for (i, elem) in hash.elements().iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            ast_fingerprint(bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    // AssocNode (hash pair): key => value
    if let Some(assoc) = node.as_assoc_node() {
        ast_fingerprint(bytes, &assoc.key(), out);
        out.extend_from_slice(b"=>");
        ast_fingerprint(bytes, &assoc.value(), out);
        return;
    }

    // RegularExpressionNode: use unescaped content + flags
    if let Some(regex) = node.as_regular_expression_node() {
        out.extend_from_slice(b"R:");
        out.extend_from_slice(regex.unescaped());
        // Include flags (e.g., /i, /m)
        let closing = regex.closing_loc().as_slice();
        if closing.len() > 1 {
            out.push(b'/');
            out.extend_from_slice(&closing[1..]);
        }
        return;
    }

    // StringNode: use unescaped content
    if let Some(string) = node.as_string_node() {
        out.extend_from_slice(b"S:");
        out.extend_from_slice(string.unescaped());
        return;
    }

    // SymbolNode: use unescaped content
    if let Some(sym) = node.as_symbol_node() {
        out.extend_from_slice(b"Y:");
        out.extend_from_slice(sym.unescaped());
        return;
    }

    // ConstantPathNode: Foo::Bar
    if let Some(cp) = node.as_constant_path_node() {
        out.extend_from_slice(b"CP:");
        if let Some(parent) = cp.parent() {
            ast_fingerprint(bytes, &parent, out);
        }
        out.extend_from_slice(b"::");
        if let Some(name) = cp.name() {
            out.extend_from_slice(name.as_slice());
        }
        return;
    }

    // ConstantReadNode: simple constant
    if let Some(cr) = node.as_constant_read_node() {
        out.extend_from_slice(b"CR:");
        out.extend_from_slice(cr.name().as_slice());
        return;
    }

    // Default: use source text for all other node types (integers, floats,
    // local variable reads, etc.)
    let loc = node.location();
    out.extend_from_slice(&bytes[loc.start_offset()..loc.end_offset()]);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        IdenticalEqualityAssertion,
        "cops/rspec/identical_equality_assertion"
    );
}
