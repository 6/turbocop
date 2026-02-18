use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Presence;

impl Cop for Presence {
    fn name(&self) -> &'static str {
        "Rails/Presence"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(if_node) = node.as_if_node() {
            // Skip elsif nodes
            let is_elsif = if_node
                .if_keyword_loc()
                .is_some_and(|kw| kw.as_slice() == b"elsif");
            if is_elsif {
                return Vec::new();
            }

            let predicate = if_node.predicate();
            let (receiver_text, is_present) = match extract_presence_check(source, &predicate) {
                Some(r) => r,
                None => return Vec::new(),
            };

            let then_clause = if_node.statements();
            let else_clause = if_node.subsequent();

            // Extract then single expr
            let then_text = match extract_single_stmt_text(source, &then_clause) {
                Some(t) => t,
                None => return Vec::new(),
            };

            // Extract else single expr or "nil"
            let else_text = extract_else_text_from_subsequent(source, &else_clause);
            let else_text = match &else_text {
                Some(t) => t.as_str(),
                None => return Vec::new(), // multi-expr else
            };

            let else_is_ignored =
                is_else_ignored_from_subsequent(&else_clause);

            return check_presence_patterns(
                self,
                source,
                node,
                &receiver_text,
                is_present,
                &then_text,
                else_text,
                &then_clause,
                else_is_ignored,
            );
        }

        if let Some(unless_node) = node.as_unless_node() {
            let predicate = unless_node.predicate();
            let (receiver_text, is_present_raw) =
                match extract_presence_check(source, &predicate) {
                    Some(r) => r,
                    None => return Vec::new(),
                };
            // `unless` flips: `unless present?` == `if blank?`
            let is_present = !is_present_raw;

            let then_clause = unless_node.statements();
            let else_clause = unless_node.else_clause();

            let then_text = match extract_single_stmt_text(source, &then_clause) {
                Some(t) => t,
                None => return Vec::new(),
            };

            let else_text = extract_else_text_from_else_node(source, &else_clause);
            let else_text = match &else_text {
                Some(t) => t.as_str(),
                None => return Vec::new(),
            };

            let else_is_ignored =
                is_else_ignored_from_else_node(&else_clause);

            return check_presence_patterns(
                self,
                source,
                node,
                &receiver_text,
                is_present,
                &then_text,
                else_text,
                &then_clause,
                else_is_ignored,
            );
        }

        Vec::new()
    }
}

/// Core logic for both if and unless: check Pattern 1 (exact match) and Pattern 2 (chain).
fn check_presence_patterns(
    cop: &Presence,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    receiver_text: &str,
    is_present: bool,
    then_text: &str,
    else_text: &str,
    then_clause: &Option<ruby_prism::StatementsNode<'_>>,
    else_is_ignored: bool,
) -> Vec<Diagnostic> {
    let (value_text, nil_text) = if is_present {
        (then_text, else_text)
    } else {
        (else_text, then_text)
    };

    // Pattern 1: value branch matches receiver exactly
    if value_text == receiver_text {
        if nil_text != "nil" {
            // Check if the "other" branch is an ignored node (if/rescue/while)
            let other_is_ignored = if is_present {
                // other = else branch
                else_is_ignored
            } else {
                // other = then branch
                if let Some(stmts) = then_clause {
                    let body: Vec<_> = stmts.body().iter().collect();
                    body.len() == 1 && is_ignored_other_node(&body[0])
                } else {
                    false
                }
            };
            if other_is_ignored {
                return Vec::new();
            }
        }

        let replacement = if nil_text == "nil" {
            format!("{receiver_text}.presence")
        } else {
            format!("{receiver_text}.presence || {nil_text}")
        };

        return emit_offense(cop, source, node, &replacement);
    }

    // Pattern 2: value branch is a method call on receiver, other is nil.
    // This "chain pattern" (e.g. `a.present? ? a.foo : nil` → `a.presence&.foo`)
    // was added in rubocop-rails 2.34.0. Skip it for now to avoid FPs on projects
    // using older rubocop-rails versions that don't have this detection.

    Vec::new()
}

/// Check if the value node is a method call on receiver (chain pattern).
fn check_chain_pattern(
    cop: &Presence,
    source: &SourceFile,
    if_node: &ruby_prism::Node<'_>,
    receiver_text: &str,
    value_node: &ruby_prism::Node<'_>,
) -> Option<Vec<Diagnostic>> {
    let call = value_node.as_call_node()?;
    if is_ignored_chain_node(&call) {
        return None;
    }
    // In RuboCop's parser gem, a call with a block is a `block` node (not `send`),
    // so the NodePattern `$(send _recv ...)` doesn't match it. Skip blocks.
    if call.block().is_some() {
        return None;
    }
    let call_recv = call.receiver()?;
    let call_recv_text = node_text(source, &call_recv);
    if call_recv_text != receiver_text {
        return None;
    }
    let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("?");
    let mut replacement = format!("{receiver_text}.presence&.{method_name}");
    if let Some(args) = call.arguments() {
        let args_text = args
            .arguments()
            .iter()
            .map(|a| node_text(source, &a))
            .collect::<Vec<_>>()
            .join(", ");
        replacement.push('(');
        replacement.push_str(&args_text);
        replacement.push(')');
    }
    Some(emit_offense(cop, source, if_node, &replacement))
}

fn emit_offense(
    cop: &Presence,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    replacement: &str,
) -> Vec<Diagnostic> {
    let loc = node.location();
    let current = node_text(source, node);
    let current_display = if current.contains('\n') {
        let first_line = current.lines().next().unwrap_or("?");
        format!("{first_line} ... end")
    } else {
        current
    };
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    vec![cop.diagnostic(
        source,
        line,
        column,
        format!("Use `{replacement}` instead of `{current_display}`."),
    )]
}

fn node_text(source: &SourceFile, node: &ruby_prism::Node<'_>) -> String {
    let loc = node.location();
    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()])
        .unwrap_or("")
        .to_string()
}

/// Extract the text of a single expression from a StatementsNode.
fn extract_single_stmt_text(
    source: &SourceFile,
    stmts: &Option<ruby_prism::StatementsNode<'_>>,
) -> Option<String> {
    let stmts = stmts.as_ref()?;
    let body: Vec<_> = stmts.body().iter().collect();
    if body.len() != 1 {
        return None;
    }
    Some(node_text(source, &body[0]))
}

/// Extract else text from IfNode's subsequent (which is Option<Node> wrapping ElseNode).
/// Returns Some("nil") for absent else, Some(text) for single-expr else, None for multi-expr.
fn extract_else_text_from_subsequent(
    source: &SourceFile,
    subsequent: &Option<ruby_prism::Node<'_>>,
) -> Option<String> {
    match subsequent {
        Some(else_node) => {
            let else_n = else_node.as_else_node()?;
            if let Some(stmts) = else_n.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() != 1 {
                    return None;
                }
                Some(node_text(source, &body[0]))
            } else {
                Some("nil".to_string())
            }
        }
        None => Some("nil".to_string()),
    }
}

/// Extract else text from UnlessNode's else_clause (which is Option<ElseNode>).
fn extract_else_text_from_else_node(
    source: &SourceFile,
    else_clause: &Option<ruby_prism::ElseNode<'_>>,
) -> Option<String> {
    match else_clause {
        Some(else_n) => {
            if let Some(stmts) = else_n.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() != 1 {
                    return None;
                }
                Some(node_text(source, &body[0]))
            } else {
                Some("nil".to_string())
            }
        }
        None => Some("nil".to_string()),
    }
}

/// Check if the else branch from IfNode's subsequent contains an ignored node.
fn is_else_ignored_from_subsequent(subsequent: &Option<ruby_prism::Node<'_>>) -> bool {
    match subsequent {
        Some(else_node) => {
            if let Some(else_n) = else_node.as_else_node() {
                if let Some(stmts) = else_n.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if body.len() == 1 {
                        return is_ignored_other_node(&body[0]);
                    }
                }
            }
            false
        }
        None => false,
    }
}

/// Check if the else branch from UnlessNode's else_clause contains an ignored node.
fn is_else_ignored_from_else_node(else_clause: &Option<ruby_prism::ElseNode<'_>>) -> bool {
    match else_clause {
        Some(else_n) => {
            if let Some(stmts) = else_n.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 {
                    return is_ignored_other_node(&body[0]);
                }
            }
            false
        }
        None => false,
    }
}

/// RuboCop's ignore_other_node?: returns true for if/rescue/while nodes
fn is_ignored_other_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_if_node().is_some()
        || node.as_unless_node().is_some()
        || node.as_rescue_node().is_some()
        || node.as_while_node().is_some()
}

/// RuboCop's ignore_chain_node?: skip chains that are [], []=, assignment,
/// arithmetic, or comparison operations.
fn is_ignored_chain_node(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    if name == b"[]" || name == b"[]=" {
        return true;
    }
    if name == b"+"
        || name == b"-"
        || name == b"*"
        || name == b"/"
        || name == b"%"
        || name == b"**"
    {
        return true;
    }
    if name == b"=="
        || name == b"!="
        || name == b"<"
        || name == b">"
        || name == b"<="
        || name == b">="
        || name == b"<=>"
    {
        return true;
    }
    if name.ends_with(b"=")
        && name != b"=="
        && name != b"!="
        && name != b"<="
        && name != b">="
        && name != b"<=>"
    {
        return true;
    }
    false
}

/// Extract the receiver text and whether it's a `present?` (true) or `blank?` (false) check.
/// Also handles negation: `!a.present?` => (a, false), `!a.blank?` => (a, true).
fn extract_presence_check(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<(String, bool)> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();

    if method == b"!" {
        let inner = call.receiver()?;
        let inner_call = inner.as_call_node()?;
        let inner_method = inner_call.name().as_slice();
        if inner_method == b"present?" {
            let recv = inner_call.receiver()?;
            return Some((node_text(source, &recv), false));
        }
        if inner_method == b"blank?" {
            let recv = inner_call.receiver()?;
            return Some((node_text(source, &recv), true));
        }
        return None;
    }

    if method == b"present?" {
        let recv = call.receiver()?;
        return Some((node_text(source, &recv), true));
    }

    if method == b"blank?" {
        let recv = call.receiver()?;
        return Some((node_text(source, &recv), false));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Presence, "cops/rails/presence");
}
