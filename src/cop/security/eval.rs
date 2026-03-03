use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=2, FN=5.
///
/// FP=2: Both in samg/timetrap `lib/Getopt/Declare.rb` (lines 32, 1266) on an
/// invalid-encoding file RuboCop treats as parser error. This remains a parser/file-drop
/// parity issue, not cop logic.
///
/// FN=5: Previously caused by `# standard:disable Security/Eval` directives being
/// treated as suppressions. Fixed by parsing only `rubocop:`/`nitrocop:` directives
/// in `parse/directives.rs` so Security/Eval now matches RuboCop directive behavior.
pub struct Eval;

impl Cop for Eval {
    fn name(&self) -> &'static str {
        "Security/Eval"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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

        if call.name().as_slice() != b"eval" {
            return;
        }

        // Match RuboCop pattern:
        //   (send {nil? (send nil? :binding) (const {cbase nil?} :Kernel)} :eval ...)
        let allowed = match call.receiver() {
            None => true,
            Some(recv) => {
                is_kernel_receiver(&recv, source)
                    || recv
                        .as_call_node()
                        .map(|c| c.name().as_slice() == b"binding" && c.receiver().is_none())
                        .unwrap_or(false)
            }
        };

        if !allowed {
            return;
        }

        // RuboCop skips:
        // 1) plain string literal first arg (`$!str` in node pattern)
        // 2) recursive-literal dstr first arg (e.g., `"foo#{2}"`)
        let args = match call.arguments() {
            Some(args) => args,
            None => return,
        };
        let mut arg_iter = args.arguments().iter();
        let Some(first_arg) = arg_iter.next() else {
            return;
        };

        if first_arg.as_string_node().is_some() {
            return;
        }
        if let Some(dstr) = first_arg.as_interpolated_string_node() {
            if dstr_is_recursive_literal(&dstr) {
                return;
            }
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "The use of `eval` is a serious security risk.".to_string(),
        ));
    }
}

fn is_kernel_receiver(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == b"Kernel";
    }
    if let Some(cp) = node.as_constant_path_node() {
        let loc = cp.location();
        let recv_src = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
        return recv_src == b"Kernel" || recv_src == b"::Kernel";
    }
    false
}

fn dstr_is_recursive_literal(dstr: &ruby_prism::InterpolatedStringNode<'_>) -> bool {
    dstr.parts().iter().all(|part| {
        if part.as_string_node().is_some() {
            return true;
        }
        let Some(embedded) = part.as_embedded_statements_node() else {
            return false;
        };
        let Some(statements) = embedded.statements() else {
            return false;
        };
        let body: Vec<ruby_prism::Node<'_>> = statements.body().into_iter().collect();
        if body.len() != 1 {
            return false;
        }
        is_recursive_literal(&body[0])
    })
}

fn is_recursive_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_regular_expression_node().is_some()
    {
        return true;
    }

    if let Some(array) = node.as_array_node() {
        return array.elements().iter().all(|e| is_recursive_literal(&e));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_recursive_literal(&assoc.key()) && is_recursive_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    if let Some(kh) = node.as_keyword_hash_node() {
        return kh.elements().iter().all(|e| {
            if let Some(assoc) = e.as_assoc_node() {
                is_recursive_literal(&assoc.key()) && is_recursive_literal(&assoc.value())
            } else {
                false
            }
        });
    }

    if let Some(inner_dstr) = node.as_interpolated_string_node() {
        return dstr_is_recursive_literal(&inner_dstr);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Eval, "cops/security/eval");
}
