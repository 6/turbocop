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
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let predicate = if_node.predicate();
        let then_clause = if_node.statements();
        let else_clause = if_node.subsequent();

        // Check the condition for present?/blank? pattern
        let (receiver_text, is_present) = match extract_presence_check(source, &predicate) {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Extract then branch text (single expression)
        let then_text = match &then_clause {
            Some(stmts) => {
                let body: Vec<_> = stmts.body().iter().collect();
                if body.len() == 1 {
                    let loc = body[0].location();
                    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).unwrap_or("").to_string()
                } else {
                    return Vec::new();
                }
            }
            None => return Vec::new(),
        };

        // Extract else branch text (single expression or nil)
        let else_text = match &else_clause {
            Some(else_node) => {
                if let Some(else_if) = else_node.as_else_node() {
                    if let Some(stmts) = else_if.statements() {
                        let body: Vec<_> = stmts.body().iter().collect();
                        if body.len() == 1 {
                            let loc = body[0].location();
                            std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).unwrap_or("").to_string()
                        } else {
                            return Vec::new();
                        }
                    } else {
                        // else with no statements = nil
                        "nil".to_string()
                    }
                } else {
                    return Vec::new();
                }
            }
            None => "nil".to_string(),
        };

        // present? ? receiver : other  =>  receiver.presence || other
        // blank? ? other : receiver    =>  receiver.presence || other
        let (value_branch, nil_branch) = if is_present {
            (&then_text, &else_text)
        } else {
            (&else_text, &then_text)
        };

        // The value branch should match the receiver
        if value_branch != &receiver_text {
            return Vec::new();
        }

        // Generate the replacement suggestion
        let replacement = if nil_branch == "nil" {
            format!("{receiver_text}.presence")
        } else {
            format!("{receiver_text}.presence || {nil_branch}")
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `{}`.",
                    std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).unwrap_or("?")),
        )]
    }
}

/// Extract the receiver text and whether it's a `present?` (true) or `blank?` (false) check.
fn extract_presence_check(source: &SourceFile, node: &ruby_prism::Node<'_>) -> Option<(String, bool)> {
    let call = node.as_call_node()?;
    let method = call.name().as_slice();

    if method == b"present?" {
        let recv = call.receiver()?;
        let loc = recv.location();
        let text = std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).ok()?;
        return Some((text.to_string(), true));
    }

    if method == b"blank?" {
        let recv = call.receiver()?;
        let loc = recv.location();
        let text = std::str::from_utf8(&source.as_bytes()[loc.start_offset()..loc.end_offset()]).ok()?;
        return Some((text.to_string(), false));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Presence, "cops/rails/presence");
}
