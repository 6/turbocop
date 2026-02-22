use crate::cop::node_type::{
    BLOCK_NODE, CALL_NODE, DEF_NODE, LOCAL_VARIABLE_READ_NODE, STATEMENTS_NODE, SYMBOL_NODE,
};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SubjectStub;

/// Flags stubbing methods on `subject`. The object under test should not be stubbed.
/// Detects: allow(subject_name).to receive(...), expect(subject_name).to receive(...)
impl Cop for SubjectStub {
    fn name(&self) -> &'static str {
        "RSpec/SubjectStub"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            DEF_NODE,
            LOCAL_VARIABLE_READ_NODE,
            STATEMENTS_NODE,
            SYMBOL_NODE,
        ]
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
        // Look for top-level describe/context blocks and track subject names
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Only process example group methods (describe, context, etc., including ::RSpec)
        let is_rspec_describe = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
                && is_rspec_example_group(method_name)
        } else {
            false
        };

        if !is_rspec_describe && !(call.receiver().is_none() && is_rspec_example_group(method_name))
        {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let mut subject_names: Vec<Vec<u8>> = Vec::new();
        // Always include "subject" as a subject name
        subject_names.push(b"subject".to_vec());

        collect_subject_stub_offenses(source, block_node, &mut subject_names, diagnostics, self);
    }
}

fn collect_subject_stub_offenses(
    source: &SourceFile,
    block: ruby_prism::BlockNode<'_>,
    subject_names: &mut Vec<Vec<u8>>,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    let body = match block.body() {
        Some(b) => b,
        None => return,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return,
    };

    // First pass: collect subject names defined in this scope
    let scope_start = subject_names.len();
    for stmt in stmts.body().iter() {
        if let Some(call) = stmt.as_call_node() {
            let name = call.name().as_slice();
            if (name == b"subject" || name == b"subject!") && call.receiver().is_none() {
                // Check if it has a name argument: subject(:foo)
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        if let Some(sym) = arg_list[0].as_symbol_node() {
                            subject_names.push(sym.unescaped().to_vec());
                        }
                    }
                }
            }
        }
    }

    // Second pass: check for stubs on subject names and recurse into nested groups
    for stmt in stmts.body().iter() {
        check_for_subject_stubs(source, &stmt, subject_names, diagnostics, cop);
    }

    // Restore subject names for this scope (don't leak child-scope subjects to siblings)
    subject_names.truncate(scope_start);
    // But re-add the implicit "subject"
    if !subject_names.contains(&b"subject".to_vec()) {
        subject_names.push(b"subject".to_vec());
    }
}

fn check_for_subject_stubs(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    subject_names: &[Vec<u8>],
    diagnostics: &mut Vec<Diagnostic>,
    cop: &SubjectStub,
) {
    if let Some(call) = node.as_call_node() {
        // Check for allow(subject_name).to receive(...) or expect(subject_name).to receive(...)
        let method = call.name().as_slice();
        if method == b"to" || method == b"not_to" || method == b"to_not" {
            // Check if the argument involves `receive`
            if has_receive_matcher(&call) {
                // Check receiver is allow/expect(subject_name) or is_expected
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        let recv_method = recv_call.name().as_slice();
                        if recv_method == b"is_expected" && recv_call.receiver().is_none() {
                            let loc = node.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(cop.diagnostic(
                                source,
                                line,
                                column,
                                "Do not stub methods of the object under test.".to_string(),
                            ));
                            return;
                        }
                        if (recv_method == b"allow" || recv_method == b"expect")
                            && recv_call.receiver().is_none()
                        {
                            if let Some(args) = recv_call.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if !arg_list.is_empty() {
                                    let arg_name = extract_simple_name(&arg_list[0]);
                                    if let Some(name) = arg_name {
                                        if subject_names.iter().any(|s| s == &name) {
                                            let loc = node.location();
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(
                                                cop.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Do not stub methods of the object under test."
                                                        .to_string(),
                                                ),
                                            );
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for `expect(subject_name).to have_received(...)`
        if method == b"to" || method == b"not_to" || method == b"to_not" {
            if has_have_received_matcher(&call) {
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        if recv_call.name().as_slice() == b"expect"
                            && recv_call.receiver().is_none()
                        {
                            if let Some(args) = recv_call.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if !arg_list.is_empty() {
                                    let arg_name = extract_simple_name(&arg_list[0]);
                                    if let Some(name) = arg_name {
                                        if subject_names.iter().any(|s| s == &name) {
                                            let loc = node.location();
                                            let (line, column) =
                                                source.offset_to_line_col(loc.start_offset());
                                            diagnostics.push(
                                                cop.diagnostic(
                                                    source,
                                                    line,
                                                    column,
                                                    "Do not stub methods of the object under test."
                                                        .to_string(),
                                                ),
                                            );
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Recurse into nested blocks (before, it, context, etc.)
        if let Some(block) = call.block() {
            if let Some(bn) = block.as_block_node() {
                let call_name = call.name().as_slice();
                if is_rspec_example_group(call_name) {
                    // Nested example group — create new scope with inherited subject names
                    let mut child_names = subject_names.to_vec();
                    collect_subject_stub_offenses(source, bn, &mut child_names, diagnostics, cop);
                } else {
                    // Non-example-group block (before, it, specify, def, etc.)
                    if let Some(body) = bn.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            for s in stmts.body().iter() {
                                check_for_subject_stubs(
                                    source,
                                    &s,
                                    subject_names,
                                    diagnostics,
                                    cop,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Check def nodes for subject stubs too
    if let Some(def_node) = node.as_def_node() {
        if let Some(body) = def_node.body() {
            if let Some(stmts) = body.as_statements_node() {
                for s in stmts.body().iter() {
                    check_for_subject_stubs(source, &s, subject_names, diagnostics, cop);
                }
            }
        }
    }
}

fn has_receive_matcher(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if contains_receive_call(&arg) {
                return true;
            }
        }
    }
    false
}

fn has_have_received_matcher(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if contains_have_received_call(&arg) {
                return true;
            }
        }
    }
    false
}

fn contains_receive_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"receive" || name == b"receive_messages" || name == b"receive_message_chain")
            && call.receiver().is_none()
        {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return contains_receive_call(&recv);
        }
        // Check arguments too (e.g., `all(receive(:baz))`)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if contains_receive_call(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

fn contains_have_received_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"have_received" && call.receiver().is_none() {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return contains_have_received_call(&recv);
        }
    }
    false
}

fn extract_simple_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
    // Extract simple method call name (receiverless call) or local variable
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() && call.arguments().is_none() {
            return Some(call.name().as_slice().to_vec());
        }
    }
    if let Some(lv) = node.as_local_variable_read_node() {
        return Some(lv.name().as_slice().to_vec());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SubjectStub, "cops/rspec/subject_stub");
}
