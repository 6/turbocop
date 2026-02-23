use crate::cop::node_type::{CALL_NODE, ELSE_NODE, IF_NODE, RESCUE_NODE, UNLESS_NODE, WHILE_NODE};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            ELSE_NODE,
            IF_NODE,
            RESCUE_NODE,
            UNLESS_NODE,
            WHILE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            // Skip elsif nodes
            let is_elsif = if_node
                .if_keyword_loc()
                .is_some_and(|kw| kw.as_slice() == b"elsif");
            if is_elsif {
                return;
            }

            let predicate = if_node.predicate();
            let (receiver_text, is_present) = match extract_presence_check(source, &predicate) {
                Some(r) => r,
                None => return,
            };

            let then_clause = if_node.statements();
            let else_clause = if_node.subsequent();

            // Extract then single node + text
            let then_node = extract_single_stmt_node(&then_clause);
            let then_text = then_node.as_ref().map(|n| node_text(source, n));
            let then_text = match &then_text {
                Some(t) => t.as_str(),
                None => return,
            };

            // Extract else single expr or "nil"
            let else_node = extract_else_node_from_subsequent(&else_clause);
            let else_text_owned = match &else_node {
                ElseNodeResult::Absent => "nil".to_string(),
                ElseNodeResult::Single(n) => node_text(source, n),
                ElseNodeResult::Multi => return,
            };
            let else_text = else_text_owned.as_str();

            let else_is_ignored = is_else_ignored_from_subsequent(&else_clause);

            diagnostics.extend(check_presence_patterns(
                self,
                source,
                node,
                &receiver_text,
                is_present,
                then_text,
                else_text,
                &then_clause,
                else_is_ignored,
                then_node.as_ref(),
                match &else_node {
                    ElseNodeResult::Single(n) => Some(n),
                    _ => None,
                },
                config,
            ));
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            let predicate = unless_node.predicate();
            let (receiver_text, is_present_raw) = match extract_presence_check(source, &predicate) {
                Some(r) => r,
                None => return,
            };
            // `unless` flips: `unless present?` == `if blank?`
            let is_present = !is_present_raw;

            let then_clause = unless_node.statements();
            let else_clause = unless_node.else_clause();

            let then_node = extract_single_stmt_node(&then_clause);
            let then_text = then_node.as_ref().map(|n| node_text(source, n));
            let then_text = match &then_text {
                Some(t) => t.as_str(),
                None => return,
            };

            let else_node_result = extract_else_node_from_else_clause(&else_clause);
            let else_text_owned = match &else_node_result {
                ElseNodeResult::Absent => "nil".to_string(),
                ElseNodeResult::Single(n) => node_text(source, n),
                ElseNodeResult::Multi => return,
            };
            let else_text = else_text_owned.as_str();

            let else_is_ignored = is_else_ignored_from_else_node(&else_clause);

            diagnostics.extend(check_presence_patterns(
                self,
                source,
                node,
                &receiver_text,
                is_present,
                then_text,
                else_text,
                &then_clause,
                else_is_ignored,
                then_node.as_ref(),
                match &else_node_result {
                    ElseNodeResult::Single(n) => Some(n),
                    _ => None,
                },
                config,
            ));
        }
    }
}

/// Result of extracting a single node from an else clause.
enum ElseNodeResult<'a> {
    /// No else clause (modifier if, or `if ... end` with no else)
    Absent,
    /// Single expression in else
    Single(ruby_prism::Node<'a>),
    /// Multiple expressions or non-matching structure
    Multi,
}

/// Extract the single node from an IfNode's subsequent (Option<Node> wrapping ElseNode).
fn extract_else_node_from_subsequent<'a>(
    subsequent: &Option<ruby_prism::Node<'a>>,
) -> ElseNodeResult<'a> {
    match subsequent {
        Some(else_node) => {
            if let Some(else_n) = else_node.as_else_node() {
                if let Some(stmts) = else_n.statements() {
                    let body: Vec<_> = stmts.body().iter().collect();
                    if body.len() == 1 {
                        return ElseNodeResult::Single(body.into_iter().next().unwrap());
                    }
                    return ElseNodeResult::Multi;
                }
                // else clause with no statements (empty else)
                ElseNodeResult::Absent
            } else {
                ElseNodeResult::Multi
            }
        }
        None => ElseNodeResult::Absent,
    }
}

/// Extract the single node from an UnlessNode's else_clause (Option<ElseNode>).
fn extract_else_node_from_else_clause<'a>(
    else_clause: &Option<ruby_prism::ElseNode<'a>>,
) -> ElseNodeResult<'a> {
    match else_clause {
        Some(else_n) => {
            if let Some(stmts) = else_n.statements() {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 {
                    return ElseNodeResult::Single(body.into_iter().next().unwrap());
                }
                return ElseNodeResult::Multi;
            }
            ElseNodeResult::Absent
        }
        None => ElseNodeResult::Absent,
    }
}

/// Extract the single statement node from a StatementsNode.
fn extract_single_stmt_node<'a>(
    stmts: &Option<ruby_prism::StatementsNode<'a>>,
) -> Option<ruby_prism::Node<'a>> {
    let stmts = stmts.as_ref()?;
    let body: Vec<_> = stmts.body().iter().collect();
    if body.len() != 1 {
        return None;
    }
    Some(body.into_iter().next().unwrap())
}

/// Core logic for both if and unless: check Pattern 1 (exact match) and Pattern 2 (chain).
#[allow(clippy::too_many_arguments)] // internal helper threading branch components
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
    then_node: Option<&ruby_prism::Node<'_>>,
    else_node: Option<&ruby_prism::Node<'_>>,
    config: &CopConfig,
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

    // Pattern 2: value branch is a method call on receiver, nil branch is nil/absent.
    // e.g. `a.present? ? a.foo : nil` -> `a.presence&.foo`
    // e.g. `a.foo if a.present?` -> `a.presence&.foo`
    // This pattern was added in rubocop-rails 2.34. Older versions don't flag it.
    // We gate on VersionChanged: if the installed gem's config has VersionChanged >= 2.34,
    // the chain pattern exists; otherwise skip.
    let version_changed: f64 = config.get_str("VersionChanged", "").parse().unwrap_or(0.0);
    if nil_text == "nil" && version_changed >= 2.34 {
        let value_node = if is_present { then_node } else { else_node };
        if let Some(vn) = value_node {
            if let Some(diags) = check_chain_pattern(cop, source, node, receiver_text, vn) {
                return diags;
            }
        }
    }

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
    source
        .byte_slice(loc.start_offset(), loc.end_offset(), "")
        .to_string()
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
    if name == b"+" || name == b"-" || name == b"*" || name == b"/" || name == b"%" || name == b"**"
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

    fn test_config() -> crate::cop::CopConfig {
        let mut config = crate::cop::CopConfig::default();
        config.options.insert(
            "VersionChanged".to_string(),
            serde_yml::Value::String("2.34".to_string()),
        );
        config
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Presence,
            include_bytes!("../../../tests/fixtures/cops/rails/presence/offense.rb"),
            test_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Presence,
            include_bytes!("../../../tests/fixtures/cops/rails/presence/no_offense.rb"),
            test_config(),
        );
    }
}
