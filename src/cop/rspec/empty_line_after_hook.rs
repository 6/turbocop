use crate::cop::util::{
    is_blank_line, is_rspec_example_group, is_rspec_hook, line_at, node_on_single_line,
    RSPEC_DEFAULT_INCLUDE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterHook;

impl Cop for EmptyLineAfterHook {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterHook"
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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for example group calls
        let is_example_group = if let Some(recv) = call.receiver() {
            if let Some(rc) = recv.as_constant_read_node() {
                rc.name().as_slice() == b"RSpec" && method_name == b"describe"
            } else {
                false
            }
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

        let allow_consecutive = config.get_bool("AllowConsecutiveOneLiners", true);
        let nodes: Vec<_> = stmts.body().iter().collect();
        let mut diagnostics = Vec::new();

        for (i, stmt) in nodes.iter().enumerate() {
            let c = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            let name = c.name().as_slice();
            if c.receiver().is_some() || !is_rspec_hook(name) {
                continue;
            }

            // Check if there's a next statement
            if i + 1 >= nodes.len() {
                continue; // last statement, no need for blank line
            }

            let loc = stmt.location();
            let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(end_offset);

            // Check if next line is blank
            let next_line = end_line + 1;
            if let Some(line) = line_at(source, next_line) {
                if is_blank_line(line) {
                    continue;
                }
            } else {
                continue;
            }

            let is_one_liner = node_on_single_line(source, &loc);

            // If consecutive one-liners are allowed, check if next is also a one-liner hook
            if allow_consecutive && is_one_liner {
                let next_stmt = &nodes[i + 1];
                if let Some(next_c) = next_stmt.as_call_node() {
                    let next_name = next_c.name().as_slice();
                    if next_c.receiver().is_none() && is_rspec_hook(next_name) {
                        let next_loc = next_stmt.location();
                        if node_on_single_line(source, &next_loc) {
                            continue; // consecutive one-liner hooks allowed
                        }
                    }
                }
            }

            let hook_name = std::str::from_utf8(name).unwrap_or("before");
            let report_col = if is_one_liner {
                let (_, start_col) = source.offset_to_line_col(loc.start_offset());
                start_col
            } else {
                if let Some(line_bytes) = line_at(source, end_line) {
                    line_bytes.iter().take_while(|&&b| b == b' ').count()
                } else {
                    0
                }
            };

            diagnostics.push(self.diagnostic(
                source,
                end_line,
                report_col,
                format!("Add an empty line after `{hook_name}`."),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterHook, "cops/rspec/empty_line_after_hook");
}
