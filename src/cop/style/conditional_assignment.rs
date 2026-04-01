use crate::cop::node_type::{CASE_MATCH_NODE, CASE_NODE, IF_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

const MSG: &str = "Use the return of the conditional for variable assignment and comparison.";

#[derive(Clone)]
struct BranchAssignment {
    signature: Vec<u8>,
    assignment: Vec<u8>,
}

#[derive(Clone, Copy)]
struct LineLengthSettings {
    enabled: bool,
    max: usize,
}

const FALLBACK_CORRECTION_LINE_LIMIT: usize = 500;

/// Detects conditionals whose branches all assign the same target.
///
/// FN reduction (2026-04-01): Prism models `case` separately from `if`, and
/// setter/index assignments are `CallNode` attribute writes instead of bare
/// variable write nodes. The previous matcher only visited `IfNode` and only
/// recognized local/instance variable writes, so it missed `case` branches,
/// ternaries, and setter/index/class/global/constant targets. RuboCop also
/// suppresses offenses whose autocorrection would exceed `Layout/LineLength`,
/// so long default literals need that guard to avoid sampled corpus FPs. This
/// cop does not receive injected `Layout/LineLength` settings yet, so its
/// fallback only suppresses pathological multiline corrections instead of
/// treating every 120+ character rewrite as over the limit.
pub struct ConditionalAssignment;

impl Cop for ConditionalAssignment {
    fn name(&self) -> &'static str {
        "Style/ConditionalAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_MATCH_NODE, CASE_NODE, IF_NODE]
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
        if config.get_str("EnforcedStyle", "assign_to_condition") != "assign_to_condition" {
            return;
        }

        let single_line_only = config.get_bool("SingleLineConditionsOnly", true);
        let include_ternary = config.get_bool("IncludeTernaryExpressions", true);
        let line_length = LineLengthSettings::from_config(config);

        if let Some(if_node) = node.as_if_node() {
            self.check_if_node(
                source,
                &if_node,
                single_line_only,
                include_ternary,
                line_length,
                diagnostics,
            );
            return;
        }

        if let Some(case_node) = node.as_case_node() {
            self.check_case_node(
                source,
                &case_node,
                single_line_only,
                line_length,
                diagnostics,
            );
            return;
        }

        if let Some(case_match_node) = node.as_case_match_node() {
            self.check_case_match_node(
                source,
                &case_match_node,
                single_line_only,
                line_length,
                diagnostics,
            );
        }
    }
}

impl ConditionalAssignment {
    fn check_if_node(
        &self,
        source: &SourceFile,
        if_node: &ruby_prism::IfNode<'_>,
        single_line_only: bool,
        include_ternary: bool,
        line_length: LineLengthSettings,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let is_ternary = if_node.if_keyword_loc().is_none();

        if is_ternary && !include_ternary {
            return;
        }

        if let Some(keyword_loc) = if_node.if_keyword_loc() {
            if keyword_loc.as_slice() == b"elsif" {
                return;
            }
        }

        let else_clause = match if_node.subsequent() {
            Some(subsequent) => subsequent,
            None => return,
        };

        // Preserve the existing nitrocop behavior: simple if/else and ternary
        // only, not if/elsif/else chains.
        if else_clause.as_if_node().is_some() {
            return;
        }

        let else_node = match else_clause.as_else_node() {
            Some(else_node) => else_node,
            None => return,
        };

        let if_assignment =
            branch_assignment_from_statements(source, if_node.statements(), single_line_only);
        let else_assignment =
            branch_assignment_from_statements(source, else_node.statements(), single_line_only);

        if let (Some(if_assignment), Some(else_assignment)) = (if_assignment, else_assignment) {
            if if_assignment.signature == else_assignment.signature
                && !correction_exceeds_line_limit(
                    source,
                    if_node.location().start_offset(),
                    if_node.location().end_offset(),
                    &if_assignment.assignment,
                    line_length,
                )
            {
                self.push_offense(source, if_node.location().start_offset(), diagnostics);
            }
        }
    }

    fn check_case_node(
        &self,
        source: &SourceFile,
        case_node: &ruby_prism::CaseNode<'_>,
        single_line_only: bool,
        line_length: LineLengthSettings,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let else_clause = match case_node.else_clause() {
            Some(else_clause) => else_clause,
            None => return,
        };

        let mut assignments = Vec::new();

        for condition in case_node.conditions().iter() {
            let when_node = match condition.as_when_node() {
                Some(when_node) => when_node,
                None => return,
            };

            let Some(assignment) =
                branch_assignment_from_statements(source, when_node.statements(), single_line_only)
            else {
                return;
            };

            assignments.push(assignment);
        }

        let Some(else_assignment) =
            branch_assignment_from_statements(source, else_clause.statements(), single_line_only)
        else {
            return;
        };
        assignments.push(else_assignment);

        if all_signatures_match(&assignments)
            && !correction_exceeds_line_limit(
                source,
                case_node.location().start_offset(),
                case_node.location().end_offset(),
                &assignments[0].assignment,
                line_length,
            )
        {
            self.push_offense(source, case_node.location().start_offset(), diagnostics);
        }
    }

    fn check_case_match_node(
        &self,
        source: &SourceFile,
        case_match_node: &ruby_prism::CaseMatchNode<'_>,
        single_line_only: bool,
        line_length: LineLengthSettings,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let else_clause = match case_match_node.else_clause() {
            Some(else_clause) => else_clause,
            None => return,
        };

        let mut assignments = Vec::new();

        for condition in case_match_node.conditions().iter() {
            let in_node = match condition.as_in_node() {
                Some(in_node) => in_node,
                None => return,
            };

            let Some(assignment) =
                branch_assignment_from_statements(source, in_node.statements(), single_line_only)
            else {
                return;
            };

            assignments.push(assignment);
        }

        let Some(else_assignment) =
            branch_assignment_from_statements(source, else_clause.statements(), single_line_only)
        else {
            return;
        };
        assignments.push(else_assignment);

        if all_signatures_match(&assignments)
            && !correction_exceeds_line_limit(
                source,
                case_match_node.location().start_offset(),
                case_match_node.location().end_offset(),
                &assignments[0].assignment,
                line_length,
            )
        {
            self.push_offense(
                source,
                case_match_node.location().start_offset(),
                diagnostics,
            );
        }
    }

    fn push_offense(
        &self,
        source: &SourceFile,
        start_offset: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let (line, column) = source.offset_to_line_col(start_offset);
        diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
    }
}

impl LineLengthSettings {
    fn from_config(config: &CopConfig) -> Self {
        let has_injected_line_length = config.options.contains_key("MaxLineLength")
            || config.options.contains_key("LineLengthEnabled");
        let max = if has_injected_line_length {
            config.get_usize("MaxLineLength", 120)
        } else {
            FALLBACK_CORRECTION_LINE_LIMIT
        };

        Self {
            enabled: config.get_bool("LineLengthEnabled", max > 0),
            max,
        }
    }
}

fn all_signatures_match(assignments: &[BranchAssignment]) -> bool {
    if assignments.len() < 2 {
        return false;
    }

    assignments
        .windows(2)
        .all(|pair| pair[0].signature == pair[1].signature)
}

fn branch_assignment_from_statements(
    source: &SourceFile,
    statements: Option<ruby_prism::StatementsNode<'_>>,
    single_line_only: bool,
) -> Option<BranchAssignment> {
    let statements = statements?;
    let body = statements.body();
    let tail = tail_statement(&body, single_line_only)?;
    assignment_details(source, &tail)
}

fn tail_statement<'pr>(
    body: &ruby_prism::NodeList<'pr>,
    single_line_only: bool,
) -> Option<ruby_prism::Node<'pr>> {
    if body.is_empty() {
        return None;
    }

    if single_line_only && body.len() != 1 {
        return None;
    }

    let tail = body.last()?;

    if let Some(begin_node) = tail.as_begin_node() {
        let statements = begin_node.statements()?;
        return tail_statement(&statements.body(), single_line_only);
    }

    Some(tail)
}

fn assignment_details(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<BranchAssignment> {
    let end_offset = assignment_end_offset(node)?;
    let assignment = source
        .content
        .get(node.location().start_offset()..end_offset)?
        .to_vec();

    Some(BranchAssignment {
        signature: assignment
            .iter()
            .copied()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect(),
        assignment,
    })
}

fn assignment_end_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(write) = node.as_local_variable_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_local_variable_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_local_variable_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_local_variable_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(write) = node.as_instance_variable_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_instance_variable_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_instance_variable_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_instance_variable_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(write) = node.as_class_variable_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_class_variable_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_class_variable_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_class_variable_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(write) = node.as_global_variable_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_global_variable_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_global_variable_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_global_variable_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_path_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_path_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_path_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_constant_path_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(call) = node.as_call_node() {
        if call.is_attribute_write() {
            call.equal_loc().map(|loc| loc.end_offset())
        } else {
            None
        }
    } else if let Some(write) = node.as_call_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_call_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_call_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else if let Some(write) = node.as_index_and_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_index_or_write_node() {
        Some(write.operator_loc().end_offset())
    } else if let Some(write) = node.as_index_operator_write_node() {
        Some(write.binary_operator_loc().end_offset())
    } else {
        None
    }
}

fn correction_exceeds_line_limit(
    source: &SourceFile,
    start_offset: usize,
    end_offset: usize,
    assignment: &[u8],
    line_length: LineLengthSettings,
) -> bool {
    if !line_length.enabled || line_length.max == 0 {
        return false;
    }

    let Some(node_source) = source.content.get(start_offset..end_offset) else {
        return false;
    };

    let longest_line_after_removal = node_source
        .split(|&byte| byte == b'\n')
        .map(trim_carriage_return)
        .map(|line| line.len() - assignment_prefix_len(line, assignment).unwrap_or(0))
        .max()
        .unwrap_or(0);

    assignment.len() + longest_line_after_removal > line_length.max
}

fn trim_carriage_return(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn assignment_prefix_len(line: &[u8], assignment: &[u8]) -> Option<usize> {
    for start in 0..=line.len() {
        if let Some(end) = match_assignment_at(line, start, assignment) {
            return Some(end - start);
        }
    }

    None
}

fn match_assignment_at(line: &[u8], start: usize, assignment: &[u8]) -> Option<usize> {
    let mut line_index = start;
    while line_index < line.len() && line[line_index].is_ascii_whitespace() {
        line_index += 1;
    }

    let mut assignment_index = 0;
    while assignment_index < assignment.len() {
        if assignment[assignment_index].is_ascii_whitespace() {
            while assignment_index < assignment.len()
                && assignment[assignment_index].is_ascii_whitespace()
            {
                assignment_index += 1;
            }

            while line_index < line.len() && line[line_index].is_ascii_whitespace() {
                line_index += 1;
            }
            continue;
        }

        if line.get(line_index).copied() != Some(assignment[assignment_index]) {
            return None;
        }

        line_index += 1;
        assignment_index += 1;
    }

    Some(line_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConditionalAssignment, "cops/style/conditional_assignment");
}
