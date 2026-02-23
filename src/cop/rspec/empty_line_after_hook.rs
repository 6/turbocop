use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{
    RSPEC_DEFAULT_INCLUDE, is_blank_line, is_rspec_hook, line_at, node_on_single_line,
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // We look for any block node whose body contains statements.
        // Among those statements, we find hook calls and check if there's
        // a blank line after them.
        let block = match node.as_block_node() {
            Some(bn) => bn,
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
            // A hook can be a bare call (before { ... }) which Prism parses as
            // a CallNode with a block child, or it can be a call_node without block
            // but that's unusual for hooks. Check if this statement is a hook call.
            let (name, loc) = if let Some(c) = stmt.as_call_node() {
                let n = c.name().as_slice();
                if c.receiver().is_some() || !is_rspec_hook(n) {
                    continue;
                }
                // Must have a block to be a hook invocation
                if c.block().is_none() {
                    continue;
                }
                (n, stmt.location())
            } else {
                continue;
            };

            // Check if there's a next statement
            if i + 1 >= nodes.len() {
                continue; // last statement, no need for blank line
            }

            let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(end_offset);

            // Check if next non-comment line is blank
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
                        let trimmed = line
                            .iter()
                            .position(|&b| b != b' ' && b != b'\t')
                            .map(|start| &line[start..])
                            .unwrap_or(&[]);
                        if trimmed.starts_with(b"#") {
                            check_line += 1;
                            continue;
                        }
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
                    if next_c.receiver().is_none()
                        && is_rspec_hook(next_name)
                        && next_c.block().is_some()
                    {
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
            } else if let Some(line_bytes) = line_at(source, end_line) {
                line_bytes.iter().take_while(|&&b| b == b' ').count()
            } else {
                0
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
