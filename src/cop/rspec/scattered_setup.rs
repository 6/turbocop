use crate::cop::util::{self, is_rspec_example_group, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ScatteredSetup;

impl Cop for ScatteredSetup {
    fn name(&self) -> &'static str {
        "RSpec/ScatteredSetup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Collect all direct `before` hooks (same scope type) and flag duplicates
        let mut before_hooks: Vec<(usize, usize)> = Vec::new(); // (line, col)
        let mut after_hooks: Vec<(usize, usize)> = Vec::new();
        let mut diagnostics = Vec::new();

        for stmt in stmts.body().iter() {
            let c = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            let name = c.name().as_slice();
            if c.receiver().is_some() {
                continue;
            }

            let loc = stmt.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());

            if name == b"before" || name == b"prepend_before" || name == b"append_before" {
                before_hooks.push((line, column));
            } else if name == b"after" || name == b"prepend_after" || name == b"append_after" {
                after_hooks.push((line, column));
            }
        }

        // Flag duplicate before hooks
        if before_hooks.len() > 1 {
            for &(line, column) in &before_hooks {
                let other_lines: Vec<String> = before_hooks
                    .iter()
                    .filter(|&&(l, _)| l != line)
                    .map(|&(l, _)| l.to_string())
                    .collect();
                let also = if other_lines.len() == 1 {
                    format!("line {}", other_lines[0])
                } else {
                    format!("lines {}", other_lines.join(", "))
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Do not define multiple `before` hooks in the same example group (also defined on {also})."
                    ),
                ));
            }
        }

        // Flag duplicate after hooks
        if after_hooks.len() > 1 {
            for &(line, column) in &after_hooks {
                let other_lines: Vec<String> = after_hooks
                    .iter()
                    .filter(|&&(l, _)| l != line)
                    .map(|&(l, _)| l.to_string())
                    .collect();
                let also = if other_lines.len() == 1 {
                    format!("line {}", other_lines[0])
                } else {
                    format!("lines {}", other_lines.join(", "))
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Do not define multiple `after` hooks in the same example group (also defined on {also})."
                    ),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ScatteredSetup, "cops/rspec/scattered_setup");
}
