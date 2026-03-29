use std::collections::HashSet;

use crate::cop::node_type::IF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks duplicate conditions across an `if`/`elsif` chain.
///
/// Investigation notes (2026-03):
/// - FP: comparing raw `predicate().location().as_slice()` bytes treated
///   heredoc-backed calls like `try_run(<<EOF)` as identical even when the
///   heredoc bodies differed, because Prism's predicate location only covers the
///   call opening and excludes the heredoc content.
/// - FN: Prism keeps `else` followed by a single nested `if` as an `ElseNode`
///   containing one `IfNode`, while Parser/RuboCop effectively continues the
///   conditional chain through that nested `if`.
///
/// Fix: compare conditions with a small AST-aware fingerprint that includes
/// call arguments and string contents, and unwrap `else { single if }` as the
/// next conditional branch when walking the chain.
pub struct DuplicateElsifCondition;

impl Cop for DuplicateElsifCondition {
    fn name(&self) -> &'static str {
        "Lint/DuplicateElsifCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
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
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Only process outer `if` nodes. Prism represents `elsif` as a nested
        // `IfNode`, and ternaries also use `IfNode` with no keyword location.
        if if_node.if_keyword_loc().is_none() || is_elsif(&if_node) {
            return;
        }

        let mut seen = HashSet::new();
        let bytes = source.as_bytes();

        // Add the first condition
        seen.insert(condition_fingerprint(bytes, &if_node.predicate()));

        // Walk the remaining conditional chain, including `else { single if }`
        // which Parser/RuboCop treats like another `elsif`.
        let mut current = next_branch_in_chain(&if_node);
        while let Some(branch) = current {
            let fingerprint = condition_fingerprint(bytes, &branch.predicate());
            if !seen.insert(fingerprint) {
                let loc = branch.predicate().location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Duplicate `elsif` condition detected.".to_string(),
                ));
            }
            current = next_branch_in_chain(&branch);
        }
    }
}

fn is_elsif(if_node: &ruby_prism::IfNode<'_>) -> bool {
    if_node
        .if_keyword_loc()
        .is_some_and(|kw| kw.as_slice() == b"elsif")
}

fn next_branch_in_chain<'pr>(if_node: &ruby_prism::IfNode<'pr>) -> Option<ruby_prism::IfNode<'pr>> {
    let subsequent = if_node.subsequent()?;

    if let Some(elsif_node) = subsequent.as_if_node() {
        return Some(elsif_node);
    }

    let else_node = subsequent.as_else_node()?;
    let stmts = else_node.statements()?;
    let mut body = stmts.body().iter();
    let nested = body.next()?;
    if body.next().is_some() {
        return None;
    }

    nested.as_if_node()
}

fn condition_fingerprint(bytes: &[u8], node: &ruby_prism::Node<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    write_condition_fingerprint(bytes, node, &mut out);
    out
}

fn write_condition_fingerprint(bytes: &[u8], node: &ruby_prism::Node<'_>, out: &mut Vec<u8>) {
    if let Some(call) = node.as_call_node() {
        out.extend_from_slice(b"C:");
        if let Some(recv) = call.receiver() {
            write_condition_fingerprint(bytes, &recv, out);
            out.push(b'.');
        }
        out.extend_from_slice(call.name().as_slice());
        out.push(b'(');
        if let Some(args) = call.arguments() {
            for (index, arg) in args.arguments().iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_condition_fingerprint(bytes, &arg, out);
            }
        }
        out.push(b')');
        if let Some(block) = call.block() {
            out.push(b'{');
            write_condition_fingerprint(bytes, &block, out);
            out.push(b'}');
        }
        return;
    }

    if let Some(array) = node.as_array_node() {
        out.extend_from_slice(b"A:[");
        for (index, elem) in array.elements().iter().enumerate() {
            if index > 0 {
                out.push(b',');
            }
            write_condition_fingerprint(bytes, &elem, out);
        }
        out.push(b']');
        return;
    }

    if let Some(hash) = node.as_hash_node() {
        out.extend_from_slice(b"H:{");
        for (index, elem) in hash.elements().iter().enumerate() {
            if index > 0 {
                out.push(b',');
            }
            write_condition_fingerprint(bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        out.extend_from_slice(b"H:{");
        for (index, elem) in hash.elements().iter().enumerate() {
            if index > 0 {
                out.push(b',');
            }
            write_condition_fingerprint(bytes, &elem, out);
        }
        out.push(b'}');
        return;
    }

    if let Some(assoc) = node.as_assoc_node() {
        write_condition_fingerprint(bytes, &assoc.key(), out);
        out.extend_from_slice(b"=>");
        write_condition_fingerprint(bytes, &assoc.value(), out);
        return;
    }

    if let Some(regex) = node.as_regular_expression_node() {
        out.extend_from_slice(b"R:");
        out.extend_from_slice(regex.unescaped());
        let closing = regex.closing_loc().as_slice();
        if closing.len() > 1 {
            out.push(b'/');
            out.extend_from_slice(&closing[1..]);
        }
        return;
    }

    if let Some(string) = node.as_string_node() {
        out.extend_from_slice(b"S:");
        out.extend_from_slice(string.unescaped());
        return;
    }

    if let Some(interpolated) = node.as_interpolated_string_node() {
        out.extend_from_slice(b"D:");
        for part in interpolated.parts().iter() {
            out.push(b'[');
            write_condition_fingerprint(bytes, &part, out);
            out.push(b']');
        }
        return;
    }

    if let Some(sym) = node.as_symbol_node() {
        out.extend_from_slice(b"Y:");
        out.extend_from_slice(sym.unescaped());
        return;
    }

    if let Some(interpolated) = node.as_interpolated_symbol_node() {
        out.extend_from_slice(b"YS:");
        for part in interpolated.parts().iter() {
            out.push(b'[');
            write_condition_fingerprint(bytes, &part, out);
            out.push(b']');
        }
        return;
    }

    if let Some(embedded) = node.as_embedded_statements_node() {
        out.extend_from_slice(b"ES:{");
        if let Some(stmts) = embedded.statements() {
            for (index, stmt) in stmts.body().iter().enumerate() {
                if index > 0 {
                    out.push(b';');
                }
                write_condition_fingerprint(bytes, &stmt, out);
            }
        }
        out.push(b'}');
        return;
    }

    if let Some(embedded) = node.as_embedded_variable_node() {
        out.extend_from_slice(b"EV:{");
        write_condition_fingerprint(bytes, &embedded.variable(), out);
        out.push(b'}');
        return;
    }

    if let Some(cp) = node.as_constant_path_node() {
        out.extend_from_slice(b"CP:");
        if let Some(parent) = cp.parent() {
            write_condition_fingerprint(bytes, &parent, out);
        }
        out.extend_from_slice(b"::");
        if let Some(name) = cp.name() {
            out.extend_from_slice(name.as_slice());
        }
        return;
    }

    if let Some(cr) = node.as_constant_read_node() {
        out.extend_from_slice(b"CR:");
        out.extend_from_slice(cr.name().as_slice());
        return;
    }

    let loc = node.location();
    out.extend_from_slice(&bytes[loc.start_offset()..loc.end_offset()]);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DuplicateElsifCondition,
        "cops/lint/duplicate_elsif_condition"
    );
}
