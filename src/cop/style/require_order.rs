use crate::cop::shared::util::{first_positional_arg, string_value};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/RequireOrder: Sort `require` and `require_relative` in alphabetical order.
///
/// Earlier revisions approximated this cop with line parsing, which matched the
/// common cases but missed RuboCop-valid AST shapes. The remaining false negatives
/// clustered around:
/// - whitespace-only spacer lines, which RuboCop does NOT treat as section breaks
///   unless the source literally contains `"\n\n"`
/// - modifier-form `if` / `unless`, including `if(...)` with no space
/// - `require` calls with extra arguments, receiverful calls like `Foo.require`,
///   and semicolon-separated statements on the same line
///
/// Fixed by switching to a `StatementsNode` visitor that mirrors RuboCop's
/// older-sibling walk: only direct sibling statements are compared, modifier
/// `if` / `unless` wrappers participate using their enclosing range for section
/// checks, and every other sibling acts as a separator.
pub struct RequireOrder;

impl Cop for RequireOrder {
    fn name(&self) -> &'static str {
        "Style/RequireOrder"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = RequireOrderVisitor {
            cop: self,
            source,
            diagnostics,
        };
        visitor.visit(&parse_result.node());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequireKind {
    Require,
    RequireRelative,
}

impl RequireKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Require => "require",
            Self::RequireRelative => "require_relative",
        }
    }
}

#[derive(Clone)]
struct RequireEntry {
    kind: RequireKind,
    path: Vec<u8>,
    report_offset: usize,
    search_start: usize,
    search_end: usize,
    can_precede: bool,
}

struct RequireOrderVisitor<'a, 'diag> {
    cop: &'a RequireOrder,
    source: &'a SourceFile,
    diagnostics: &'diag mut Vec<Diagnostic>,
}

impl RequireOrderVisitor<'_, '_> {
    fn check_statements(&mut self, stmts: &ruby_prism::StatementsNode<'_>) {
        let mut last_entry: Option<RequireEntry> = None;
        let mut max_path: Option<Vec<u8>> = None;

        for stmt in stmts.body().iter() {
            let Some(entry) = extract_require_entry(&stmt) else {
                last_entry = None;
                max_path = None;
                continue;
            };

            let same_group = last_entry.as_ref().is_some_and(|prev| {
                prev.kind == entry.kind && in_same_section(self.source, prev, &entry)
            });

            if !same_group {
                max_path = None;
            }

            if let Some(prev_max) = max_path.as_ref() {
                if entry.path.as_slice() < prev_max.as_slice() {
                    let (line, column) = self.source.offset_to_line_col(entry.report_offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!("Sort `{}` in alphabetical order.", entry.kind.as_str()),
                    ));
                } else if entry.can_precede {
                    max_path = Some(entry.path.clone());
                }
            } else if entry.can_precede {
                max_path = Some(entry.path.clone());
            }

            if entry.can_precede {
                last_entry = Some(entry);
            } else {
                last_entry = None;
                max_path = None;
            }
        }
    }
}

impl<'pr> Visit<'pr> for RequireOrderVisitor<'_, '_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        self.check_statements(node);
        ruby_prism::visit_statements_node(self, node);
    }
}

fn extract_require_entry(node: &ruby_prism::Node<'_>) -> Option<RequireEntry> {
    if let Some(call) = node.as_call_node() {
        let report_offset = call.location().start_offset();
        let search_loc = call.location();
        return require_entry_from_call(&call, report_offset, search_loc);
    }

    if let Some(if_node) = node.as_if_node() {
        return modifier_entry_from_if(if_node);
    }

    if let Some(unless_node) = node.as_unless_node() {
        return modifier_entry_from_unless(unless_node);
    }

    None
}

fn modifier_entry_from_if(node: ruby_prism::IfNode<'_>) -> Option<RequireEntry> {
    let if_loc = node.if_keyword_loc()?;
    let stmts = node.statements()?;
    let stmt = only_statement(&stmts)?;

    // Modifier-form `bar if foo` places the keyword after the statement body.
    if if_loc.start_offset() <= stmt.location().start_offset() || node.subsequent().is_some() {
        return None;
    }

    let call = stmt.as_call_node()?;
    let report_offset = call.location().start_offset();
    require_entry_from_call(&call, report_offset, node.location())
}

fn modifier_entry_from_unless(node: ruby_prism::UnlessNode<'_>) -> Option<RequireEntry> {
    let keyword_loc = node.keyword_loc();
    let stmts = node.statements()?;
    let stmt = only_statement(&stmts)?;

    if keyword_loc.start_offset() <= stmt.location().start_offset() || node.else_clause().is_some()
    {
        return None;
    }

    let call = stmt.as_call_node()?;
    let report_offset = call.location().start_offset();
    require_entry_from_call(&call, report_offset, node.location())
}

fn only_statement<'pr>(
    stmts: &'pr ruby_prism::StatementsNode<'pr>,
) -> Option<ruby_prism::Node<'pr>> {
    if stmts.body().len() != 1 {
        return None;
    }
    stmts.body().first()
}

fn require_entry_from_call(
    call: &ruby_prism::CallNode<'_>,
    report_offset: usize,
    search_loc: ruby_prism::Location<'_>,
) -> Option<RequireEntry> {
    let kind = match normalized_call_name(call.name().as_slice()) {
        b"require" => RequireKind::Require,
        b"require_relative" => RequireKind::RequireRelative,
        _ => return None,
    };

    let first_arg = first_positional_arg(call)?;
    let path = string_value(&first_arg)?;

    Some(RequireEntry {
        kind,
        path,
        report_offset,
        search_start: search_loc.start_offset(),
        search_end: search_loc.end_offset(),
        can_precede: call.receiver().is_none(),
    })
}

fn normalized_call_name(name: &[u8]) -> &[u8] {
    name.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(name)
}

fn in_same_section(source: &SourceFile, previous: &RequireEntry, current: &RequireEntry) -> bool {
    source
        .as_bytes()
        .get(previous.search_start..current.search_end)
        .is_some_and(|bytes| !bytes.windows(2).any(|window| window == b"\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireOrder, "cops/style/require_order");
}
