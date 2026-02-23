use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, is_rspec_example_group};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ScatteredSetup;

/// Extract the scope argument from a before/after hook call.
/// Returns a key like b"each" (default), b"all", b"context", b"suite".
fn extract_hook_scope(call: &ruby_prism::CallNode<'_>) -> Vec<u8> {
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if let Some(sym) = arg.as_symbol_node() {
                return sym.unescaped().to_vec();
            }
        }
    }
    // No scope arg = :each (default)
    b"each".to_vec()
}

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && method_name == b"describe"
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

        // Collect all direct `before` hooks grouped by (hook_type, scope) and flag duplicates.
        // before :all and before :each (or before with no arg) are different scopes.
        let mut before_hooks: std::collections::HashMap<Vec<u8>, Vec<(usize, usize)>> =
            std::collections::HashMap::new();
        let mut after_hooks: std::collections::HashMap<Vec<u8>, Vec<(usize, usize)>> =
            std::collections::HashMap::new();

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

            let scope = extract_hook_scope(&c);

            if name == b"before" || name == b"prepend_before" || name == b"append_before" {
                before_hooks.entry(scope).or_default().push((line, column));
            } else if name == b"after" || name == b"prepend_after" || name == b"append_after" {
                after_hooks.entry(scope).or_default().push((line, column));
            }
        }

        // Flag duplicate before hooks (same scope only)
        for hooks in before_hooks.values() {
            if hooks.len() > 1 {
                for &(line, column) in hooks {
                    let other_lines: Vec<String> = hooks
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
        }

        // Flag duplicate after hooks (same scope only)
        for hooks in after_hooks.values() {
            if hooks.len() > 1 {
                for &(line, column) in hooks {
                    let other_lines: Vec<String> = hooks
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ScatteredSetup, "cops/rspec/scattered_setup");
}
