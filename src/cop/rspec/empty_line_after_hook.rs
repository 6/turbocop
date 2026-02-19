use crate::cop::util::{
    self, is_blank_line, is_rspec_example_group, is_rspec_hook, line_at, node_on_single_line,
    RSPEC_DEFAULT_INCLUDE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for example group calls (including ::RSpec.describe)
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let allow_consecutive = config.get_bool("AllowConsecutiveOneLiners", true);
        let nodes: Vec<_> = stmts.body().iter().collect();

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

            // Check if next non-comment line is blank (or if there's a blank
            // line between the hook end and the next code line).
            // Skip rubocop directive comments and regular comments.
            let mut check_line = end_line + 1;
            let mut found_blank = false;
            loop {
                match line_at(source, check_line) {
                    None => {
                        found_blank = true; // end of file
                        break;
                    }
                    Some(line) => {
                        if is_blank_line(line) {
                            found_blank = true;
                            break;
                        }
                        let trimmed = line.iter()
                            .position(|&b| b != b' ' && b != b'\t')
                            .map(|start| &line[start..])
                            .unwrap_or(&[]);
                        if trimmed.starts_with(b"#") {
                            // Comment line — skip it and check the next line
                            check_line += 1;
                            continue;
                        }
                        // Non-blank, non-comment line without a preceding blank
                        break;
                    }
                }
            }
            if found_blank {
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

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterHook, "cops/rspec/empty_line_after_hook");
}
