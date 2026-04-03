use crate::cop::shared::node_type::{PROGRAM_NODE, STATEMENTS_NODE};
use crate::cop::shared::util::is_rspec_example_group;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use std::collections::HashMap;

type DescriptionSignature = Vec<u8>;
type ExampleGroupName = Vec<u8>;
type ExampleGroupLocation = (usize, usize, ExampleGroupName);
type RepeatedDescriptionMap = HashMap<DescriptionSignature, Vec<ExampleGroupLocation>>;

/// RSpec/RepeatedExampleGroupDescription: Flag example groups with identical descriptions.
///
/// ## Investigation findings (2026-03-11)
///
/// Root cause of 119 FNs: the cop only handled `ProgramNode` (top-level) and `CallNode`
/// (example group blocks) as parent contexts. RuboCop's `on_begin` fires on ANY `begin`
/// node (equivalent to Prism's `StatementsNode`), which also includes module/class bodies,
/// method bodies, and any other compound statement context.
///
/// Fix: switched to handling `StatementsNode` directly, which covers all contexts where
/// sibling example groups can appear (top-level program, module/class bodies, block bodies).
///
/// Also added:
/// - `skip_or_pending` filtering: groups containing bare `skip` or `pending` calls are
///   excluded from duplicate checking (matches RuboCop's SkipOrPending mixin).
/// - Empty description filtering: groups with no arguments are excluded.
///
/// ## Corpus investigation (FN=79, 2026-03-15)
///
/// FN=79: description_signature used raw source bytes for comparison, causing
/// single-quoted vs double-quoted strings to be treated as different descriptions.
/// RuboCop's AST node equality treats "foo" and 'foo' identically (both are str nodes).
/// Fixed by normalizing StringNode/SymbolNode to their unescaped values.
///
/// ## Investigation (2026-03-30)
///
/// FN=2 in `garethr/garethr-docker`: duplicate `context` descriptions inside
/// `if` branches were missed even though the cop listened for `StatementsNode`.
/// Prism's generated visitor calls `visit_statements_node()` directly from
/// `visit_if_node`/`visit_unless_node`/`visit_when_node`/`visit_else_node`,
/// which bypasses `visit_branch_node_enter`; nested branch bodies never reached
/// `check_node`. Fixed by recursively checking conditional branch bodies from
/// each visited sibling statement list.
pub struct RepeatedExampleGroupDescription;

impl Cop for RepeatedExampleGroupDescription {
    fn name(&self) -> &'static str {
        "RSpec/RepeatedExampleGroupDescription"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[PROGRAM_NODE, STATEMENTS_NODE]
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
        // Handle ProgramNode (top-level) and StatementsNode (module/class/block bodies).
        // ProgramNode is needed because Prism visits the top-level statements through
        // ProgramNode rather than dispatching a standalone top-level StatementsNode.
        if let Some(program) = node.as_program_node() {
            check_sibling_groups(self, source, &program.statements(), diagnostics);
        } else if let Some(stmts_node) = node.as_statements_node() {
            check_sibling_groups(self, source, &stmts_node, diagnostics);
        }
    }
}

fn check_sibling_groups(
    cop: &RepeatedExampleGroupDescription,
    source: &SourceFile,
    stmts: &ruby_prism::StatementsNode<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let stmt_nodes: Vec<_> = stmts.body().iter().collect();

    // RuboCop's several_example_groups? only considers direct sibling groups in
    // the current statement list. Nested conditional branches are handled
    // separately below.
    let example_group_count = stmt_nodes
        .iter()
        .filter(|s| is_example_group_call(s))
        .count();

    if example_group_count >= 2 {
        let mut desc_map: RepeatedDescriptionMap = HashMap::new();

        for stmt in &stmt_nodes {
            let call = match extract_example_group_call(stmt) {
                Some(c) => c,
                None => continue,
            };

            // Skip groups with skip/pending inside the block
            if has_skip_or_pending_in_block(stmt) {
                continue;
            }

            // Skip groups with no description (empty_description?)
            if call.arguments().is_none() {
                continue;
            }

            let name = call.name().as_slice();

            // Extract the description signature (all args)
            let desc_sig = match description_signature(source, &call) {
                Some(s) => s,
                None => continue,
            };

            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            desc_map
                .entry(desc_sig)
                .or_default()
                .push((line, col, name.to_vec()));
        }

        push_repeated_description_diagnostics(cop, source, desc_map, diagnostics);
    }

    for stmt in stmt_nodes {
        collect_branch_diagnostics(cop, source, &stmt, diagnostics);
    }
}

fn push_repeated_description_diagnostics(
    cop: &RepeatedExampleGroupDescription,
    source: &SourceFile,
    desc_map: RepeatedDescriptionMap,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for locs in desc_map.values() {
        if locs.len() <= 1 {
            continue;
        }

        for (idx, (line, col, group_name)) in locs.iter().enumerate() {
            let other_lines: Vec<String> = locs
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, (l, _, _))| l.to_string())
                .collect();
            let group_type = std::str::from_utf8(group_name).unwrap_or("describe");
            let display_type = group_type
                .strip_prefix('f')
                .or(group_type.strip_prefix('x'))
                .unwrap_or(group_type);
            let msg = format!(
                "Repeated {} block description on line(s) [{}]",
                display_type,
                other_lines.join(", ")
            );
            diagnostics.push(cop.diagnostic(source, *line, *col, msg));
        }
    }
}

fn collect_branch_diagnostics(
    cop: &RepeatedExampleGroupDescription,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            check_sibling_groups(cop, source, &stmts, diagnostics);
        }
        if let Some(subsequent) = if_node.subsequent() {
            collect_branch_diagnostics(cop, source, &subsequent, diagnostics);
        }
    } else if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            check_sibling_groups(cop, source, &stmts, diagnostics);
        }
        if let Some(else_clause) = unless_node.else_clause() {
            collect_branch_diagnostics(cop, source, &else_clause.as_node(), diagnostics);
        }
    } else if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            check_sibling_groups(cop, source, &stmts, diagnostics);
        }
    } else if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            collect_branch_diagnostics(cop, source, &condition, diagnostics);
        }
        if let Some(else_clause) = case_node.else_clause() {
            collect_branch_diagnostics(cop, source, &else_clause.as_node(), diagnostics);
        }
    } else if let Some(when_node) = node.as_when_node() {
        if let Some(stmts) = when_node.statements() {
            check_sibling_groups(cop, source, &stmts, diagnostics);
        }
    } else if let Some(in_node) = node.as_in_node() {
        if let Some(stmts) = in_node.statements() {
            check_sibling_groups(cop, source, &stmts, diagnostics);
        }
    }
}

/// Check if a node is an example group call (with or without receiver).
fn is_example_group_call(node: &ruby_prism::Node<'_>) -> bool {
    extract_example_group_call(node).is_some()
}

/// Extract the CallNode from a node if it's an example group call.
/// The node might be a bare CallNode or a CallNode with a block.
fn extract_example_group_call<'a>(
    node: &'a ruby_prism::Node<'a>,
) -> Option<ruby_prism::CallNode<'a>> {
    let call = node.as_call_node()?;
    if is_rspec_group_for_desc(&call) {
        // Must have a block to be an example group
        if call.block().is_some() {
            return Some(call);
        }
    }
    None
}

/// Check if a block-level example group contains `skip` or `pending` calls.
/// Matches RuboCop's `skip_or_pending_inside_block?` pattern:
///   (block <(send nil? {:skip :pending} ...) ...>)
fn has_skip_or_pending_in_block(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let block = match call.block() {
        Some(b) => b,
        None => return false,
    };
    let block_node = match block.as_block_node() {
        Some(b) => b,
        None => return false,
    };
    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };
    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };
    for stmt in stmts.body().iter() {
        if let Some(inner_call) = stmt.as_call_node() {
            let name = inner_call.name().as_slice();
            if (name == b"skip" || name == b"pending") && inner_call.receiver().is_none() {
                return true;
            }
        }
    }
    false
}

fn description_signature(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let args = call.arguments()?;
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return None;
    }

    let mut sig: Vec<u8> = Vec::new();

    // Normalize the first argument (description string/symbol) to its unescaped value.
    // RuboCop compares AST node equality where "foo" and 'foo' produce the same (str "foo") node.
    let first = &arg_list[0];
    if let Some(s) = first.as_string_node() {
        sig.extend_from_slice(b"str:");
        sig.extend_from_slice(s.unescaped());
    } else if let Some(sym) = first.as_symbol_node() {
        sig.extend_from_slice(b"sym:");
        sig.extend_from_slice(sym.unescaped());
    } else {
        // Constants, interpolated strings, etc: use source text
        let loc = first.location();
        sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
    }

    // Metadata args: use source text (symbols here won't have quote style differences)
    for arg in &arg_list[1..] {
        sig.push(b',');
        let loc = arg.location();
        sig.extend_from_slice(&source.as_bytes()[loc.start_offset()..loc.end_offset()]);
    }

    Some(sig)
}

fn is_rspec_group_for_desc(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if name == b"shared_examples" || name == b"shared_examples_for" || name == b"shared_context" {
        return false;
    }
    if !is_rspec_example_group(name) {
        return false;
    }
    match call.receiver() {
        None => true,
        Some(recv) => {
            if let Some(cr) = recv.as_constant_read_node() {
                cr.name().as_slice() == b"RSpec"
            } else if let Some(cp) = recv.as_constant_path_node() {
                cp.name().is_some_and(|n| n.as_slice() == b"RSpec") && cp.parent().is_none()
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        RepeatedExampleGroupDescription,
        "cops/rspec/repeated_example_group_description"
    );
}
