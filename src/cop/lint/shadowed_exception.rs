use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::BEGIN_NODE;

pub struct ShadowedException;

// Known Ruby exception hierarchy (simplified)
// Known Ruby exception hierarchy.
// LoadError, NotImplementedError and SyntaxError are subclasses of ScriptError
// (NOT StandardError). This matters for Lint/ShadowedException correctness.
const EXCEPTION_HIERARCHY: &[(&str, &[&str])] = &[
    ("Exception", &["StandardError", "ScriptError", "SecurityError", "SignalException",
                     "SystemExit", "SystemStackError", "NoMemoryError", "RuntimeError",
                     "NameError", "TypeError", "ArgumentError", "RangeError",
                     "IOError", "EOFError", "RegexpError", "ZeroDivisionError",
                     "ThreadError", "Errno::ENOENT", "Errno::EACCES", "LoadError",
                     "NotImplementedError", "NoMethodError", "StopIteration",
                     "IndexError", "KeyError", "Math::DomainError",
                     "Encoding::UndefinedConversionError", "Encoding::InvalidByteSequenceError",
                     "Encoding::ConverterNotFoundError", "Fiber::SchedulerError",
                     "Interrupt", "SyntaxError"]),
    ("StandardError", &["RuntimeError", "NameError", "TypeError", "ArgumentError",
                         "RangeError", "IOError", "EOFError", "RegexpError",
                         "ZeroDivisionError", "ThreadError", "Errno::ENOENT",
                         "Errno::EACCES", "NoMethodError", "StopIteration",
                         "IndexError", "KeyError"]),
    ("ScriptError", &["LoadError", "NotImplementedError", "SyntaxError"]),
    ("NameError", &["NoMethodError"]),
    ("RangeError", &["FloatDomainError"]),
    ("IOError", &["EOFError"]),
    ("IndexError", &["KeyError", "StopIteration"]),
    ("SignalException", &["Interrupt"]),
    // Standard library exception hierarchies
    ("IPAddr::Error", &["IPAddr::InvalidAddressError"]),
    ("Net::ProtocolError", &["Net::HTTPBadResponse", "Net::HTTPHeaderSyntaxError",
                              "Net::FTPPermError", "Net::FTPTempError", "Net::FTPProtoError",
                              "Net::FTPReplyError"]),
    ("Gem::Exception", &["Gem::LoadError", "Gem::InstallError", "Gem::DependencyError",
                          "Gem::FormatException", "Gem::CommandLineError"]),
];

fn is_ancestor_of(ancestor: &str, descendant: &str) -> bool {
    if ancestor == descendant {
        return false;
    }
    for &(parent, children) in EXCEPTION_HIERARCHY {
        if parent == ancestor && children.contains(&descendant) {
            return true;
        }
    }
    false
}

/// Check if a single group contains multiple levels of exceptions (ancestor/descendant pair).
fn contains_multiple_levels(group: &[String]) -> bool {
    if group.len() < 2 {
        return false;
    }
    // If group includes Exception and anything else, it has multiple levels
    if group.iter().any(|e| e == "Exception") {
        return true;
    }
    for i in 0..group.len() {
        for j in (i + 1)..group.len() {
            if is_ancestor_of(&group[i], &group[j]) || is_ancestor_of(&group[j], &group[i]) {
                return true;
            }
        }
    }
    false
}

/// Check if two consecutive groups are in sorted order (more specific first).
fn groups_sorted(earlier: &[String], later: &[String]) -> bool {
    // If earlier group includes Exception, it's always wrong order
    if earlier.iter().any(|e| e == "Exception") {
        return false;
    }
    // If later includes Exception or either group is empty, consider sorted
    if later.iter().any(|e| e == "Exception") || earlier.is_empty() || later.is_empty() {
        return true;
    }
    // Check that no earlier exception is an ancestor of a later one
    for e in earlier {
        for l in later {
            if is_ancestor_of(e, l) {
                return false;
            }
        }
    }
    true
}

impl Cop for ShadowedException {
    fn name(&self) -> &'static str {
        "Lint/ShadowedException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let mut rescue_opt = begin_node.rescue_clause();
        let mut all_clauses: Vec<(Vec<String>, usize)> = Vec::new();

        while let Some(rescue_node) = rescue_opt {
            let exceptions: Vec<String> = rescue_node
                .exceptions()
                .iter()
                .filter_map(|e| {
                    std::str::from_utf8(e.location().as_slice())
                        .ok()
                        .map(|s| s.to_string())
                })
                .collect();

            let kw_loc = rescue_node.keyword_loc();
            all_clauses.push((exceptions, kw_loc.start_offset()));
            rescue_opt = rescue_node.subsequent();
        }

        if all_clauses.len() < 2 && all_clauses.iter().all(|(excs, _)| excs.len() <= 1) {
            return;
        }

        let groups: Vec<&Vec<String>> = all_clauses.iter().map(|(excs, _)| excs).collect();

        // Check if any single group has multiple levels
        let has_multi_level = groups.iter().any(|g| contains_multiple_levels(g));

        // Check if groups are sorted
        let all_sorted = groups.windows(2).all(|w| {
            let earlier = if w[0].is_empty() { vec!["StandardError".to_string()] } else { w[0].clone() };
            let later = if w[1].is_empty() { vec!["StandardError".to_string()] } else { w[1].clone() };
            groups_sorted(&earlier, &later)
        });

        if !has_multi_level && all_sorted {
            return;
        }

        // Find the first offending rescue clause (matching RuboCop's find_shadowing_rescue)
        // First check: any group with multiple levels
        for (_i, (excs, offset)) in all_clauses.iter().enumerate() {
            let group = if excs.is_empty() {
                vec!["StandardError".to_string()]
            } else {
                excs.clone()
            };
            if contains_multiple_levels(&group) {
                let (line, column) = source.offset_to_line_col(*offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not shadow rescued Exceptions.".to_string(),
                ));
            }
        }

        // Second check: first clause that makes ordering unsorted
        let resolved_groups: Vec<Vec<String>> = all_clauses.iter().map(|(excs, _)| {
            if excs.is_empty() {
                vec!["StandardError".to_string()]
            } else {
                excs.clone()
            }
        }).collect();

        for i in 0..resolved_groups.len().saturating_sub(1) {
            if !groups_sorted(&resolved_groups[i], &resolved_groups[i + 1]) {
                let (line, column) = source.offset_to_line_col(all_clauses[i].1);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Do not shadow rescued Exceptions.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShadowedException, "cops/lint/shadowed_exception");
}
